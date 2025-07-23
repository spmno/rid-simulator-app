use std::convert::TryInto;
use tracing::info;
use serde::{Serialize, Deserialize};
use super::message::{Message, MessageError, MessageType};
use std::time::{SystemTime, UNIX_EPOCH};

// SystemMessage 结构体，系统报文（报文类型 0x4）为周期性，强制静态报文，用于描述无人驾驶航空器控制站位置和高度 、 航空器组群及额外的系统信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    // 起始字节1 (1字节)
    pub coordinate_system: u8,     // 坐标系类型 (7位)
    #[serde(default)]
    pub reserved_bits: u8,         // 预留位 (6-5位)
    pub classification_region: u8, // 等级分类归属区域 (4-2位)
    pub station_type: u8,          // 控制站位置类型 (1-0位)

    // 起始字节2 (4字节)
    pub latitude: i32,             // 控制站纬度 (小端序)

    // 起始字节6 (4字节)
    pub longitude: i32,             // 控制站经度 (小端序)

    // 可选字段
    pub operation_count: u16, // 运行区域计数 (小端序)
    pub operation_radius: u8, // 运行区域半径 (*10)
    pub altitude_upper: u16,  // 运行区域高度上限 (几何高度, 小端序)
    pub altitude_lower: u16,  // 运行区域高度下限 (几何高度, 小端序)

    // 起始字节17 (1字节)
    #[serde(default)]
    pub ua_category: u8,           // UA运行类别

    // 起始字节18 (1字节)
    #[serde(default)]
    pub ua_level: u8,              // UA等级

    // 起始字节19 (2字节)
    #[serde(default)]
    pub station_altitude: u16,     // 控制站高度 (小端序)

    // 时间戳
    pub timestamp: u32,     // 时间戳 (Unix时间, 秒)
    #[serde(default)]
    pub reserved: u8,       // 预留
}

impl SystemMessage {
    pub const MESSAGE_TYPE: u8 = 0x04;
    const EXPECTED_LENGTH: usize = 24;

}


// 实现 Message trait
impl Message for SystemMessage {
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError> {
        
        if data.len() < Self::EXPECTED_LENGTH {
            return Err(MessageError::InsufficientLength(
                Self::EXPECTED_LENGTH,
                data.len()
            ));
        }

        // 解析起始字节1
        let byte0 = data[0];
        let coordinate_system = (byte0 >> 5) & 0x07; // 取bit7-5
        let reserved_bits = (byte0 >> 3) & 0x03;    // 取bit6-5
        let classification_region = (byte0 >> 2) & 0x07; // 取bit4-2
        
        // 验证分类区域值
        if classification_region == 0 || classification_region > 3 {
            info!("class region = {}", classification_region);
            return Err(MessageError::UnknownMessageType(1));
        }
        
        let station_type = byte0 & 0x03; // 取bit1-0

        // 解析控制站纬度 (小端序)
        let latitude = i32::from_le_bytes(data[1..5].try_into()
            .map_err(|_| MessageError::InsufficientLength(5, data.len()))?);

        // 解析控制站经度 (小端序)
        let longitude = i32::from_le_bytes(data[5..9].try_into()
            .map_err(|_| MessageError::InsufficientLength(9, data.len()))?);

        // 处理可选字段（起始字节10）
        let mut offset = 9;
        let value = u16::from_le_bytes([data[offset], data[offset+1]]);
        let operation_count = value;
        offset += 2;

        let value = data[offset];
        let operation_radius = value;
        offset += 1;

        let value = u16::from_le_bytes([data[offset], data[offset+1]]);
        let altitude_upper = value;
        offset += 2;

        let value = u16::from_le_bytes([data[offset], data[offset+1]]);
        let altitude_lower = value;
        offset += 2;

        // 解析必送字段
        let ua_category = data[offset];
        offset += 1;
        
        let ua_level = data[offset];
        offset += 1;
        
        // 解析控制站高度
        let station_altitude = u16::from_le_bytes([data[offset], data[offset+1]]);
        offset += 2;
   

        // 时间及尾部字段
        let timestamp = u32::from_le_bytes([
            data[offset], data[offset+1], data[offset+2], data[offset+3]
        ]);
        offset += 4;

        let reserved = data[offset];

        Ok(Self {
            coordinate_system,
            reserved_bits,
            classification_region,
            station_type,
            latitude,
            longitude,
            operation_count,
            operation_radius,
            altitude_upper,
            altitude_lower,
            ua_category,
            ua_level,
            station_altitude,
            timestamp,
            reserved,
        })
    }

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        let message_type = MessageType::SystemMessageType as u8;
        let message_protocol = (message_type << 4) | 0x01;
        bytes.push(message_protocol);

        // 第1字节编码
        let mut byte1 = (self.coordinate_system & 0x7F) << 1;
        byte1 |= (self.reserved_bits & 0x03) << 5;
        byte1 |= (self.classification_region & 0x07) << 2;
        byte1 |= self.station_type & 0x03;
        bytes.push(byte1 as u8);
        
        // 经纬度编码（小端序）
        bytes.extend_from_slice(&self.latitude.to_le_bytes());
        bytes.extend_from_slice(&self.longitude.to_le_bytes());
        
        // count and radius
        bytes.extend_from_slice(&self.operation_count.to_le_bytes());
        bytes.push(self.operation_radius);
        bytes.extend_from_slice(&self.altitude_upper.to_le_bytes());
        bytes.extend_from_slice(&self.altitude_lower.to_le_bytes());
        
        // UA类别和等级
        let ua_category_level = self.ua_category << 4 | self.ua_level;
        bytes.push(ua_category_level);
        
        // 控制站高度
        bytes.extend_from_slice(&self.station_altitude.to_le_bytes());
        
        // 时间戳和预留
         let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32;
        bytes.extend_from_slice(&timestamp.to_le_bytes());

        bytes.push(self.reserved);
        
        bytes
    }

    fn print(&self) {
        println!("=== 系统消息 (SystemMessage) ===");
        println!("坐标系类型: {}", self.coordinate_system);
        println!("预留位: {:02b}", self.reserved_bits);
        println!("等级分类归属区域: {}", match self.classification_region {
            2 => "中国",
            3..=7 => "预留",
            _ => "未定义或无效",
        });
        println!("控制站位置类型: {}", self.station_type);
        println!("控制站纬度: {:.6}°", self.latitude as f64 * 1e-7);
        println!("控制站经度: {:.6}°", self.longitude as f64 * 1e-7);
        
        println!("运行区域计数: {}", self.operation_count);
        println!("运行区域半径: {} (实际: {} 米)", self.operation_radius, self.operation_radius as f32 * 10.0);
        
        println!("运行区域高度上限: {} (实际: {:.1} 米)", self.altitude_upper, self.altitude_upper as f32 * 0.1);
        println!("运行区域高度下限: {} (实际: {:.1} 米)", self.altitude_lower, self.altitude_lower as f32 * 0.1);
        
        println!("UA运行类别: {}", self.ua_category);
        println!("UA等级: {}", self.ua_level);
        println!("控制站高度: {} (实际: {:.1} 米)", self.station_altitude, self.station_altitude as f32 * 0.1);
        
        // 实际应用中可将时间戳转换为可读时间
        println!("时间戳: {}", self.timestamp);

        println!("预留字段: {:02X}", self.reserved);
        
    }
}
