pub mod rid_simulator;
pub mod message;

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use rumqttc::{AsyncClient, Event, MqttOptions, QoS, Packet, Publish};
use tauri::{AppHandle, Emitter};
use crate::rid_simulator::RidSimulator;
use crate::message::packet_message::PacketMessage;
use crate::message::message::Message;
use serde_json;

// Global MQTT client state
static MQTT_CLIENT: once_cell::sync::OnceCell<Arc<Mutex<Option<AsyncClient>>>> = once_cell::sync::OnceCell::new();
static RID_SIMULATOR: once_cell::sync::OnceCell<Arc<Mutex<RidSimulator>>> = once_cell::sync::OnceCell::new();
static APP_HANDLE: once_cell::sync::OnceCell<AppHandle> = once_cell::sync::OnceCell::new();
static PORT: u16 = 443;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn connect_to_mqtt_server(host: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    // Store app handle globally for later use
    if APP_HANDLE.get().is_none() {
        APP_HANDLE.set(app_handle.clone()).ok();
    }

    // Initialize RidSimulator singleton
    if RID_SIMULATOR.get().is_none() {
        let mut simulator = RidSimulator::new();
        simulator.start_simulator();
        let simulator_arc = Arc::new(Mutex::new(simulator));
        RID_SIMULATOR.set(simulator_arc).ok();
        info!("RidSimulator initialized");
    }

    info!("Connecting to MQTT broker: {}, port: {}", host, PORT);
    // Configure MQTT options
    let mut mqtt_options = MqttOptions::new("rid-simulator-app", host, PORT);
    
    // Enable WebSocket transport if needed
    mqtt_options
        .set_credentials("rabbitmq", "x8I3RGgu4b9YEDPu")
        .set_transport(rumqttc::Transport::wss_with_default_config())
        .set_keep_alive(std::time::Duration::from_secs(600))
        .set_clean_session(true);
    
    // Create client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);
    
    // Store client globally
    let client_arc = Arc::new(Mutex::new(Some(client.clone())));
    MQTT_CLIENT.set(client_arc).map_err(|_| "Failed to store MQTT client".to_string())?;
    
    // Start event loop in background
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(packet)) => {
                    info!("MQTT packet received: {:?}", packet);
                    
                    // Handle Publish packets
                    if let Packet::Publish(publish) = packet {
                        handle_publish_packet(publish).await;
                    }
                }
                Ok(Event::Outgoing(packet)) => {
                    info!("MQTT packet sent: {:?}", packet);
                }
                Err(e) => {
                    error!("MQTT error: {}", e);
                    break;
                }
            }
        }
    });
    
    // Wait a moment for connection to establish
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Subscribe to the required topic
    if let Some(client_arc) = MQTT_CLIENT.get() {
        let client_guard = client_arc.lock().await;
        if let Some(client) = client_guard.as_ref() {
            match client.subscribe("mx-lafs-simulation/filght-info-rid", QoS::AtLeastOnce).await {
                Ok(_) => {
                    info!("Successfully subscribed to mx-lafs-simulation/filght-info-rid");
                    return Ok("连接成功".to_string());
                }
                Err(e) => {
                    error!("Failed to subscribe to topic: {}", e);
                    return Err(format!("订阅主题失败: {}", e));
                }
            }
        }
    }
    
    Err("MQTT客户端未初始化".to_string())
}

// Helper function to send log messages to frontend
pub fn send_log_to_frontend(message: &str) {
    if let Some(app_handle) = APP_HANDLE.get() {
        let _ = app_handle.emit("log-message", message.to_string());
    }
}

// Handle incoming Publish packets
async fn handle_publish_packet(publish: Publish) {
    let topic = publish.topic;
    let payload = publish.payload;
    
    info!("Received message on topic: {}, payload size: {} bytes", topic, payload.len());
    
    // Send log message to frontend
    send_log_to_frontend(&format!("收到MQTT消息: 主题={}, 大小={}字节", topic, payload.len()));
    
    // Parse JSON payload into PacketMessage
    match serde_json::from_slice::<PacketMessage>(&payload) {
        Ok(message) => {
            info!("Successfully parsed PacketMessage from JSON");
            
            // Send log to frontend
            send_log_to_frontend("成功解析PacketMessage数据");
            
            // Get the RidSimulator singleton and send RID
            if let Some(simulator_arc) = RID_SIMULATOR.get() {
                let simulator = simulator_arc.lock().await;
                let ssid = message.get_ssid();
                let encoded_data = message.encode();
                
                match simulator.build_and_send_rid(&ssid, encoded_data) {
                    Ok(_) => {
                        info!("Successfully sent RID for SSID: {}", ssid);
                        send_log_to_frontend(&format!("成功发送RID数据包: SSID={}", ssid));
                    }
                    Err(e) => {
                        error!("Failed to send RID: {}", e);
                        send_log_to_frontend(&format!("发送RID失败: {}", e));
                    }
                }
            } else {
                error!("RidSimulator not initialized");
                send_log_to_frontend("错误: RidSimulator未初始化");
            }
        }
        Err(e) => {
            error!("Failed to parse JSON payload into PacketMessage: {}", e);
            // Log the payload for debugging
            if let Ok(payload_str) = String::from_utf8(payload.to_vec()) {
                error!("Payload content: {}", payload_str);
                send_log_to_frontend(&format!("解析JSON失败: {} - 数据: {}", e, payload_str));
            }
        }
    }
}

#[tauri::command]
async fn disconnect_mqtt() -> Result<String, String> {
    info!("Disconnecting from MQTT broker");
    
    if let Some(client_arc) = MQTT_CLIENT.get() {
        let mut client_guard = client_arc.lock().await;
        if let Some(client) = client_guard.take() {
            match client.disconnect().await {
                Ok(_) => {
                    info!("Successfully disconnected from MQTT broker");
                    return Ok("断开连接成功".to_string());
                }
                Err(e) => {
                    error!("Failed to disconnect: {}", e);
                    return Err(format!("断开连接失败: {}", e));
                }
            }
        }
    }
    
    Ok("未连接到MQTT服务器".to_string())
}

#[tauri::command]
async fn add_log_from_rust(message: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    info!("Sending log message to frontend: {}", message);
    
    // Emit the log event to the frontend
    if let Err(e) = app_handle.emit("log-message", message) {
        error!("Failed to emit log message: {}", e);
        return Err(format!("Failed to send log message: {}", e));
    }
    
    Ok(())
}

fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Create logs directory if it doesn't exist
    let log_dir = std::env::current_dir()?.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    // Configure file appender with daily rotation
    let file_appender = tracing_appender::rolling::daily(log_dir, "rid-simulator-app.log");
    
    // Configure console output
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    // Configure file output with JSON format
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(file_appender)
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true);

    // Configure log level from environment variable or default to INFO
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Initialize the subscriber with both console and file layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    info!("Logging initialized successfully");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    if let Err(e) = setup_logging() {
        error!("Failed to initialize logging: {}", e);
    }

    info!("Starting Rid simulator application");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![connect_to_mqtt_server, disconnect_mqtt, add_log_from_rust])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
