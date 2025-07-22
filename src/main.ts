import { invoke } from "@tauri-apps/api/core";

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;

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
    // Here you would implement the actual connection logic
    // For now, we'll simulate a connection attempt
    
    addLog(`连接到: ${selectedEnvironment}`);
    
    // Simulate connection delay
    setTimeout(() => {
      updateConnectionStatus(`已连接到 ${environmentName}`, true);
      addLog(`成功连接到 ${environmentName}`);
      addLog(`WebSocket地址: ${selectedEnvironment}`);
    }, 1000);
    
  } catch (error) {
    updateConnectionStatus("连接失败", false);
    addLog(`连接失败: ${error}`);
  }
}

// Handle disconnect
function handleDisconnect() {
  updateConnectionStatus("未连接", false);
  addLog("已断开连接");
}

async function greet() {
  if (greetMsgEl && greetInputEl) {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
  }
}

window.addEventListener("DOMContentLoaded", () => {
  // Initialize existing elements
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  document.querySelector("#greet-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

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

  // Add initial log
  addLog("系统初始化完成");
});
