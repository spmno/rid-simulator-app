use pnet::datalink::{interfaces, Channel, NetworkInterface};
use libwifi::{FrameProtocolVersion, FrameType, FrameSubType};
use libwifi::frame::Beacon;
use libwifi::frame::components::{ManagementHeader, FrameControl, MacAddress, SequenceControl, StationInfo, VendorSpecificInfo};
use tracing::{info, error};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::Utc;
pub struct RidSimulator {
    wifi_devices: Vec<NetworkInterface>,
}

static SEQ_COUNTER: AtomicU16 = AtomicU16::new(0);

/// RadioTap头，主要是写一些WIFI管理帧的信息
#[derive(Debug, Default)]
struct RadioTapHeader {
    it_version: u8,     // 固定为 0
    it_pad: u8,          // 填充 0
    it_len: u16,         // 头部总长度（需动态计算）
    it_present1: u32,     // 字段存在位掩码
    it_present2: u32,
    it_present3: u32,
    timestamp: u64,
    flags: u8,
    datarate: u8,
    channel_info_freq: u16,
    channel_info_flags: u16,
    antenna_signal1: u8,
    reserved:u8,
    rx_flags: u16,
    antenna_signal2: u8,
    antenna1: u8,
    antenna_signal3: u8,
    antenna2: u8,
}

impl RidSimulator {
    pub fn new() -> Self {
        RidSimulator {
            wifi_devices: Vec::new(),
        }
    }

    fn get_wifi_devices(&mut self) {
        let interfaces = interfaces();

        info!("Available WiFi network devices:");
        for interface in interfaces {
            // 根据操作系统调整过滤条件, 分别为台式机, raspberry pi， 笔记本的wifi名称
            if interface.name.contains("wlx") || interface.name.contains("wlan1") || interface.name.contains("wlp4") {
                info!("Name: {}, MAC: {:?}", interface.name, interface.mac);
                self.wifi_devices.push(interface);
            } else {
                info!("no wifi device: Name: {}, MAC: {:?}", interface.name, interface.mac);
            }
        }
    }

    pub fn start_simulator(&mut self) {
        self.get_wifi_devices();
        if self.wifi_devices.is_empty() {
            panic!("No WiFi devices found");
        }
        info!("device counter: {}", self.wifi_devices.len());
    }

    pub fn build_and_send_rid(&self, ssid: &str, data: Vec<u8>) -> Result<String, String> {
        let radiotap_bytes = self.build_radiotap_header();
        let beacon_frame = self.build_rid_beacon(ssid, data.as_slice());
        let full_frame = [radiotap_bytes, beacon_frame].concat();
        self.send_beacon(&full_frame)?;  // 添加错误传播
        info!("beacon frame: {:?}", full_frame);
        Ok("OK".to_string())  // 修改返回Result
    }

    fn build_radiotap_header(&self) -> Vec<u8> {
        let mut header = RadioTapHeader {
            it_version: 0,
            it_pad: 0,
            it_len: 8, // 初始长度（不含扩展字段）
            it_present1: 0xa000_402f, // 示例：启用 Channel 字段（位掩码第15位）
            it_present2: 0xa000_0820, // 示例：启用 Channel 字段（位掩码第15位）
            it_present3: 0x0000_0820,
            timestamp:  Utc::now().timestamp_micros() as u64,
            flags: 0x10,
            datarate: 0x0c,
            channel_info_freq: 2437u16,
            channel_info_flags: 0x00c0,
            antenna_signal1:0xc4,
            reserved:0x00,
            rx_flags: 0x0000,
            antenna_signal2:0xc3,
            antenna1: 0x00,
            antenna_signal3:0xc4,
            antenna2: 0x01,
        };

        // 更新长度：基础头(8字节) + 信道字段(4字节)
        header.it_len = 38 as u16;

        // 序列化为字节流
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&header.it_version.to_le_bytes());
        bytes.extend_from_slice(&header.it_pad.to_le_bytes());
        bytes.extend_from_slice(&header.it_len.to_le_bytes());
        bytes.extend_from_slice(&header.it_present1.to_le_bytes());
        bytes.extend_from_slice(&header.it_present2.to_le_bytes());
        bytes.extend_from_slice(&header.it_present3.to_le_bytes());
        bytes.extend_from_slice(&header.timestamp.to_le_bytes());
        bytes.extend_from_slice(&header.flags.to_le_bytes());
        bytes.extend_from_slice(&header.datarate.to_le_bytes());
        bytes.extend_from_slice(&header.channel_info_freq.to_le_bytes());
        bytes.extend_from_slice(&header.channel_info_flags.to_le_bytes());
        bytes.extend_from_slice(&header.antenna_signal1.to_le_bytes());
        bytes.extend_from_slice(&header.reserved.to_le_bytes());
        bytes.extend_from_slice(&header.rx_flags.to_le_bytes());
        bytes.extend_from_slice(&header.antenna_signal2.to_le_bytes());
        bytes.extend_from_slice(&header.antenna1.to_le_bytes());
        bytes.extend_from_slice(&header.antenna_signal3.to_le_bytes());
        bytes.extend_from_slice(&header.antenna2.to_le_bytes());
        println!("byte len: {}", bytes.len());
        bytes
    }

    // 构造含RID的Beacon帧
    pub fn build_rid_beacon(&self, ssid: &str, rid_data: &[u8]) -> Vec<u8> {
        let seq = SEQ_COUNTER.fetch_add(0x10, Ordering::SeqCst); // 序列号按802.11规范递增
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64; // 动态时间戳[1](@ref)

        let header = ManagementHeader {
            frame_control: FrameControl {
                protocol_version: FrameProtocolVersion::PV0,
                frame_type: FrameType::Management,
                frame_subtype: FrameSubType::Beacon,
                flags: 0,
            },
            duration: [0, 0],
            address_1: MacAddress([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]), // 广播地址
            address_2: MacAddress([0x00, 0xE0, 0x4B, 0xD3, 0xDE, 0xD6]), // 发送端MAC（需替换）
            address_3: MacAddress([0x00, 0xE0, 0x4B, 0xD3, 0xDE, 0xD6]), // BSSID
            sequence_control: SequenceControl {
                fragment_number: 0,
                sequence_number: seq,
            },
        };

        let mut station_info = StationInfo {
            // 没啥用先注了
            //supported_rates: vec![
            //    SupportedRate { mandatory: true, rate: 1.0 }, // 1 Mbps（必选）[4](@ref)
            //    SupportedRate { mandatory: false, rate: 6.0 },
            //],
            ssid: Some(ssid.to_string()),
            ssid_length: Some(ssid.len()),
            ds_parameter_set: Some(6), // 信道6（需与物理信道一致）
            ..Default::default()
        };

        // 直接嵌入原始RID数据（不添加OUI头部）
        station_info.vendor_specific.push(VendorSpecificInfo {
            element_id: 221,             // IEEE自定义元素ID
            length: (rid_data.len()) as u8, // 
            oui: [0xfa, 0x0b, 0xbc],                 
            oui_type: 13,                 
            data: rid_data.to_vec(),
        });

        let beacon = Beacon {
            header,
            timestamp,
            beacon_interval: 100, // 100 TU ≈ 102.4ms（10Hz）[4](@ref)
            capability_info: 0x1104, // 开放网络 + 短前导码
            station_info,
        };

        beacon.encode()
    }

    pub fn send_beacon(&self, beacon_data: &[u8]) -> Result<(), String> {
        match pnet::datalink::channel(&self.wifi_devices[0], Default::default()) {
            Ok(Channel::Ethernet(mut tx, _rx)) => {
                if let Some(_) = tx.send_to(beacon_data, None) {
                    info!("send rid.");
                    Ok(())
                } else {
                    error!("Failed to send packet");
                    Err("发送失败".into())
                }
            },
            Ok(_) => {
                error!("Unsupported channel type");
                Err("不支持的通道类型".into())
            }
            Err(e) => {
                error!("Failed to create channel: {}", e);
                Err(format!("通道创建失败: {}", e))
            }
        }
    }

}

