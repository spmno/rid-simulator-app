import { invoke } from "@tauri-apps/api/core";

// New elements for connection functionality
let environmentSelectEl: HTMLSelectElement | null;
let connectBtnEl: HTMLButtonElement | null;
let connectionStatusEl: HTMLElement | null;
let logDisplayEl: HTMLElement | null;

// Connection state
let isConnected = false;

// Log function to add messages to the log display
function addLog(message: string) {
  if (logDisplayEl) {
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = document.createElement('p');
    logEntry.textContent = `[${timestamp}] ${message}`;
    logDisplayEl.appendChild(logEntry);
    logDisplayEl.scrollTop = logDisplayEl.scrollHeight;
  }
}

// Update connection status display
function updateConnectionStatus(status: string, connected: boolean) {
  if (connectionStatusEl) {
    connectionStatusEl.textContent = status;
    isConnected = connected;
  }
}

// Handle connect button click
async function handleConnect() {
  if (!environmentSelectEl) return;

  const selectedEnvironment = environmentSelectEl.value;
  const environmentName = environmentSelectEl.options[environmentSelectEl.selectedIndex].text;
  
  addLog(`正在连接到 ${environmentName}...`);
  updateConnectionStatus(`正在连接 ${environmentName}...`, false);
  
  try {
    const result = await invoke("connect_to_mqtt_server", {
      host: selectedEnvironment
    });
    
    updateConnectionStatus(`已连接到 ${environmentName}`, true);
    addLog(result as string);
    addLog(`MQTT服务器: ${selectedEnvironment}`);
    addLog(`订阅主题: mx-lafs-simulation/filght-info-rid`);
    
  } catch (error) {
    updateConnectionStatus("连接失败", false);
    addLog(`连接失败: ${error}`);
  }
}

// Handle disconnect
async function handleDisconnect() {
  try {
    const result = await invoke("disconnect_mqtt");
    updateConnectionStatus("未连接", false);
    addLog(result as string);
  } catch (error) {
    addLog(`断开连接失败: ${error}`);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  // Initialize new connection elements
  environmentSelectEl = document.querySelector("#environment-select");
  connectBtnEl = document.querySelector("#connect-btn");
  connectionStatusEl = document.querySelector("#connection-status");
  logDisplayEl = document.querySelector("#log-display");

  // Add event listener for connect button
  connectBtnEl?.addEventListener("click", () => {
    if (isConnected) {
      handleDisconnect();
    } else {
      handleConnect();
    }
  });

  // Listen for log messages from Rust
  import('@tauri-apps/api/event').then(({ listen }) => {
    listen('log-message', (event) => {
      const message = event.payload as string;
      addLog(message);
    });
  }).catch(error => {
    console.error('Failed to set up Rust log listener:', error);
  });

  // Add initial log
  addLog("系统初始化完成");
});
