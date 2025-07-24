use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, QoS, Packet, Publish};
use tauri::{AppHandle, Emitter};
use once_cell::sync::OnceCell;
use tracing::{info, error};

use crate::message::packet_message::PacketMessage;
use crate::message::message::Message;
use crate::rid_simulator::RidSimulator;

#[derive(Debug, Clone)]
pub struct MqttManager {
    client: Arc<Mutex<Option<AsyncClient>>>,
    event_loop_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    connection_status: Arc<Mutex<bool>>,
    rid_simulator: Arc<Mutex<Option<Arc<Mutex<RidSimulator>>>>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl MqttManager {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            event_loop_handle: Arc::new(Mutex::new(None)),
            connection_status: Arc::new(Mutex::new(false)),
            rid_simulator: Arc::new(Mutex::new(None)),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, host: String, port: u16, app_handle: AppHandle) -> Result<String, String> {
        info!("Connecting to MQTT broker: {}, port: {}", host, port);

        // Initialize app handle if not set
        {
            let mut handle_guard = self.app_handle.lock().await;
            if handle_guard.is_none() {
                *handle_guard = Some(app_handle.clone());
            }
        }

        // Initialize RidSimulator if not exists
        {
            let mut sim_guard = self.rid_simulator.lock().await;
            if sim_guard.is_none() {
                let mut simulator = RidSimulator::new();
                simulator.start_simulator();
                *sim_guard = Some(Arc::new(Mutex::new(simulator)));
                info!("RidSimulator initialized");
            }
        }

        // Configure MQTT options
        let mut mqtt_options = MqttOptions::new("rid-simulator-app", host, port);
        mqtt_options
            .set_credentials("rabbitmq", "x8I3RGgu4b9YEDPu")
            .set_transport(rumqttc::Transport::wss_with_default_config())
            .set_keep_alive(std::time::Duration::from_secs(30))
            .set_clean_session(true)
            .set_max_packet_size(1024 * 1024, 1024 * 1024);

        // Create client and event loop
        let (client, eventloop) = AsyncClient::new(mqtt_options, 10);

        // Store client
        {
            let mut client_guard = self.client.lock().await;
            *client_guard = Some(client.clone());
        }

        // Update connection status
        {
            let mut status = self.connection_status.lock().await;
            *status = true;
        }

        // Start event loop
        let handle = self.start_event_loop(eventloop).await;
        {
            let mut handle_guard = self.event_loop_handle.lock().await;
            *handle_guard = Some(handle);
        }

        // Subscribe to topic
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match client.subscribe("mx-lafs-simulation/filght-info-rid", QoS::AtLeastOnce).await {
            Ok(_) => {
                info!("Successfully subscribed to mx-lafs-simulation/filght-info-rid");
                Ok("连接成功".to_string())
            }
            Err(e) => {
                error!("Failed to subscribe to topic: {}", e);
                self.disconnect().await?;
                Err(format!("订阅主题失败: {}", e))
            }
        }
    }

    pub async fn disconnect(&self) -> Result<String, String> {
        info!("Disconnecting from MQTT broker");

        // Update connection status to signal event loop to exit
        {
            let mut status = self.connection_status.lock().await;
            *status = false;
        }

        // Abort event loop
        {
            let mut handle_guard = self.event_loop_handle.lock().await;
            if let Some(handle) = handle_guard.take() {
                handle.abort();
                info!("MQTT event loop aborted");
            }
        }

        // Disconnect client
        {
            let mut client_guard = self.client.lock().await;
            if let Some(client) = client_guard.take() {
                let _ = client.disconnect().await;
                info!("Successfully disconnected from MQTT broker");
            }
        }

        Ok("断开连接成功".to_string())
    }

    pub async fn is_connected(&self) -> bool {
        let status = self.connection_status.lock().await;
        *status
    }

    async fn start_event_loop(&self, mut eventloop: EventLoop) -> JoinHandle<()> {
        let connection_status = self.connection_status.clone();
        let rid_simulator = self.rid_simulator.clone();
        let app_handle = self.app_handle.clone();

        tokio::spawn(async move {
            let mut retry_count = 0;
            const MAX_RETRIES: u32 = 5;

            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(packet)) => {
                        info!("MQTT packet received: {:?}", packet);
                        retry_count = 0;

                        if let Packet::Publish(publish) = packet {
                            Self::handle_publish_packet(
                                publish,
                                rid_simulator.clone(),
                                app_handle.clone(),
                            ).await;
                        }
                    }
                    Ok(Event::Outgoing(packet)) => {
                        info!("MQTT packet sent: {:?}", packet);
                    }
                    Err(e) => {
                        // Check if we should continue retrying
                        {
                            let status = connection_status.lock().await;
                            if !*status {
                                info!("MQTT event loop exiting due to intentional disconnect");
                                break;
                            }
                        }

                        error!("MQTT error: {}", e);
                        Self::send_log_to_frontend(
                            app_handle.clone(),
                            &format!("MQTT连接错误: {}", e),
                        ).await;

                        if retry_count < MAX_RETRIES {
                            let backoff = std::time::Duration::from_secs(2u64.pow(retry_count));
                            retry_count += 1;
                            info!("Retrying connection in {:?} (attempt {}/{})", backoff, retry_count, MAX_RETRIES);
                            Self::send_log_to_frontend(
                                app_handle.clone(),
                                &format!("{}秒后重试连接...", backoff.as_secs()),
                            ).await;
                            tokio::time::sleep(backoff).await;
                        } else {
                            info!("Max retries reached, waiting longer before retry...");
                            Self::send_log_to_frontend(
                                app_handle.clone(),
                                "达到最大重试次数，等待更长时间后重试...",
                            ).await;
                            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                            retry_count = 0;
                        }
                        continue;
                    }
                }
            }
        })
    }

    async fn handle_publish_packet(
        publish: Publish,
        rid_simulator: Arc<Mutex<Option<Arc<Mutex<RidSimulator>>>>>,
        app_handle: Arc<Mutex<Option<AppHandle>>>,
    ) {
        let topic = publish.topic;
        let payload = publish.payload;

        info!("Received message on topic: {}, payload size: {} bytes", topic, payload.len());
        Self::send_log_to_frontend(
            app_handle.clone(),
            &format!("收到MQTT消息: 主题={}, 大小={}字节", topic, payload.len()),
        ).await;

        match serde_json::from_slice::<PacketMessage>(&payload) {
            Ok(message) => {
                info!("Successfully parsed PacketMessage from JSON");
                Self::send_log_to_frontend(app_handle.clone(), "成功解析PacketMessage数据").await;

                if let Some(sim_arc) = rid_simulator.lock().await.as_ref() {
                    let simulator = sim_arc.lock().await;
                    let ssid = message.get_ssid();
                    let encoded_data = message.encode();

                    match simulator.build_and_send_rid(&ssid, encoded_data) {
                        Ok(_) => {
                            info!("Successfully sent RID for SSID: {}", ssid);
                            Self::send_log_to_frontend(
                                app_handle.clone(),
                                &format!("成功发送RID数据包: SSID={}", ssid),
                            ).await;
                        }
                        Err(e) => {
                            error!("Failed to send RID: {}", e);
                            Self::send_log_to_frontend(
                                app_handle.clone(),
                                &format!("发送RID失败: {}", e),
                            ).await;
                        }
                    }
                } else {
                    error!("RidSimulator not initialized");
                    Self::send_log_to_frontend(app_handle.clone(), "错误: RidSimulator未初始化").await;
                }
            }
            Err(e) => {
                error!("Failed to parse JSON payload into PacketMessage: {}", e);
                if let Ok(payload_str) = String::from_utf8(payload.to_vec()) {
                    error!("Payload content: {}", payload_str);
                    Self::send_log_to_frontend(
                        app_handle.clone(),
                        &format!("解析JSON失败: {} - 数据: {}", e, payload_str),
                    ).await;
                }
            }
        }
    }

    async fn send_log_to_frontend(app_handle: Arc<Mutex<Option<AppHandle>>>, message: &str) {
        if let Some(handle) = app_handle.lock().await.as_ref() {
            let _ = handle.emit("log-message", message.to_string());
        }
    }
}

// Global singleton
static MQTT_MANAGER: OnceCell<Arc<MqttManager>> = OnceCell::new();

pub fn get_mqtt_manager() -> Arc<MqttManager> {
    MQTT_MANAGER
        .get_or_init(|| Arc::new(MqttManager::new()))
        .clone()
}