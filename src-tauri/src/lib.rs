pub mod rid_simulator;
pub mod message;
pub mod mqtt_manager;

use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tauri::Emitter;
use crate::mqtt_manager::get_mqtt_manager;

#[tauri::command]
async fn connect_to_mqtt_server(host: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    let manager = get_mqtt_manager();
    manager.connect(host, 443, app_handle).await
}

#[tauri::command]
async fn disconnect_mqtt() -> Result<String, String> {
    let manager = get_mqtt_manager();
    manager.disconnect().await
}

#[tauri::command]
async fn get_connection_status() -> Result<bool, String> {
    let manager = get_mqtt_manager();
    Ok(manager.is_connected().await)
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
    // Set necessary environment variables for WebKit to prevent initialization errors
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    
    // Initialize logging
    if let Err(e) = setup_logging() {
        error!("Failed to initialize logging: {}", e);
    }

    info!("Starting Rid simulator application");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            connect_to_mqtt_server, 
            disconnect_mqtt, 
            get_connection_status,
            add_log_from_rust
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}