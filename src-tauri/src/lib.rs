pub mod rid_simulator;
pub mod message;

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};

// Global MQTT client state
static MQTT_CLIENT: once_cell::sync::OnceCell<Arc<Mutex<Option<AsyncClient>>>> = once_cell::sync::OnceCell::new();
static PORT: u16 = 443;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn connect_to_mqtt_server(host: String) -> Result<String, String> {

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
        .invoke_handler(tauri::generate_handler![connect_to_mqtt_server, disconnect_mqtt])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
