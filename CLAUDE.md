# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tauri-based Remote ID (RID) simulator application that receives drone flight information via MQTT and broadcasts it as WiFi beacon frames. The application simulates drone Remote ID broadcasts for testing purposes.

## Architecture

The application is structured as a Tauri app with:
- **Frontend**: Vanilla TypeScript/HTML/CSS running in a Tauri webview
- **Backend**: Rust code handling MQTT communication and WiFi frame broadcasting
- **Communication**: Tauri commands for frontend-backend interaction

## Key Components

### Backend (Rust - `src-tauri/src/`)

- **lib.rs**: Main Tauri application setup with command handlers
- **mqtt_manager.rs**: MQTT client management with connection, subscription, and message handling
- **rid_simulator.rs**: Core WiFi frame generation and broadcasting functionality
- **message/**: Message serialization/deserialization modules
  - `packet_message.rs`: Main packet structure combining base, system, and position messages
  - `base_message.rs`, `system_message.rs`, `position_vector_message.rs`: Individual message types

### Frontend (TypeScript - `src/`)

- **main.ts**: Frontend logic for MQTT connection management and log display
- **index.html**: Simple UI with environment selection, connection controls, and logging

## Key Commands

### Development Commands

```bash
# Install dependencies
pnpm install

# Development mode
pnpm tauri dev

# Build for production
pnpm tauri build

# Type checking
pnpm tsc

# Frontend build only
pnpm build
```

### Tauri Commands (Frontend → Backend)

- `connect_to_mqtt_server(host: string)` - Connect to MQTT broker
- `disconnect_mqtt()` - Disconnect from MQTT broker  
- `get_connection_status()` - Get current MQTT connection status
- `add_log_from_rust(message: string)` - Send log messages to frontend

## Configuration

### MQTT Configuration
- **Topic**: `mx-lafs-simulation/filght-info-rid`
- **Environments**: 
  - test: `wss://mx-lasm-mqtt-test.mxnavi.com/ws`
  - pre: `wss://mx-lasm-mqtt-pre.mxnavi.com/ws`
- **Credentials**: Hardcoded in mqtt_manager.rs (rabbitmq/x8I3RGgu4b9YEDPu)

### WiFi Configuration
- **Frame Type**: Beacon frames with vendor-specific RID data
- **SSID Format**: `RID-{uas_id}`
- **Channel**: 6 (2.437 GHz)
- **OUI**: 0xfa, 0x0b, 0xbc (vendor-specific)

## Data Flow

1. **MQTT Subscription**: Backend subscribes to drone flight info topic
2. **JSON Parsing**: Receives JSON payload and parses into `PacketMessage`
3. **Message Encoding**: Converts `PacketMessage` to binary format with CRC16
4. **WiFi Broadcasting**: Sends beacon frames with RID data via WiFi interface
5. **Logging**: Real-time logging displayed in frontend UI

## Build System

- **Backend**: Cargo (Rust)
- **Frontend**: Vite + TypeScript
- **Bundler**: Tauri build system
- **Target**: Desktop application (Windows, macOS, Linux)

## WiFi Device Detection

The system auto-detects WiFi interfaces with names containing:
- `wlx*` (desktop)
- `wlan1` (Raspberry Pi)
- `wlp4*` (laptop)

## Security Notes

- MQTT credentials are hardcoded (consider environment variables for production)
- WiFi frames broadcast in clear text
- Application requires appropriate WiFi permissions for packet injection