use crate::message::message::MessageType;

use super::message::{Message, MessageError};
use serde::{Serialize, Deserialize};

/// 位置向量报文，强 制 动 态 报 文 ，主要包含位置 ， 高度 ， 速度 ， 时间等标识 UA 运行情况的报文 。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionVectorMessage {
    // 第1字节 (运行状态和标志位)
    pub run_status: u8,         // 运行状态 (7-4位)
    #[serde(default)]
    pub reserved_flag: bool,     // 预留标志位 (3位)
    pub height_type: u8,        // 高度类型位 (2位) - 0-3
    pub track_direction: u8,   // 航迹角 E/W 方向标志 (1位)
    pub speed_multiplier: u8,  // 速度乘数 (0位)

    // 第2-4字节
    pub track_angle: u8,        // 航迹角 (1字节)
    pub ground_speed: i8,       // 地速 (1字节, 有正负)
    pub vertical_speed: i8,     // 垂直速度 (1字节, 有正负, 可选)

    // 第5-18字节
    pub latitude: i32,           // 纬度 (4字节小端序)
    pub longitude: i32,          // 经度 (4字节小端序)
    pub pressure_altitude: i16, // 气压高度 (2字节小端序, 可选)
    pub geometric_altitude: i16, // 几何高度 (2字节小端序, 可选)
    pub ground_altitude: i16,    // 距地高度 (2字节小端序)

    // 第19-22字节
    pub vertical_accuracy: u8,   // 垂直精度 (7-4位, 4 bits)
    pub horizontal_accuracy: u8, // 水平精度 (3-0位, 4 bits)
    pub speed_accuracy: u8,      // 速度精度 (3-0位, 4 bits)
    pub timestamp: u16,          // 时间戳 (2字节小端序)

    // 第23-24字节
    #[serde(default)]
    pub timestamp_accuracy: u8, // 时间戳精度 (3-0位, 4 bits)
    #[serde(default)]
    pub reserved: u8,           // 预留 (1字节)
}

impl PositionVectorMessage {
    pub const MESSAGE_TYPE: u8 = 0x01;
    const EXPECTED_LENGTH: usize = 24;

    fn calculate_full_track_angle(&self) -> u16 {
        if self.track_direction == 1 {
            self.track_angle as u16 + 180
        } else {
            self.track_angle as u16
        }
    }
    
    fn calculate_ground_speed_knots(&self) -> f32 {
        if self.speed_multiplier == 1{
            self.ground_speed as f32 * 10.0
        } else {
            self.ground_speed as f32
        }
    }
    


}

impl Message for PositionVectorMessage {
    /// 从u8数组解析为PositionVectorMessage
    ///
    /// 根据表格描述，消息总长度为24字节
    ///
    /// # 参数
    /// - `data`: 至少包含24字节的输入数据
    ///
    /// # 错误
    /// 当输入数据长度不足时返回ParseError
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError>  {
        // 验证数据长度
        if data.len() < Self::EXPECTED_LENGTH {
            return Err(MessageError::InsufficientLength(Self::EXPECTED_LENGTH, data.len()));
        }

        // 解析第1字节 (运行状态和标志位)
        let byte0 = data[0];
        let run_status = (byte0 >> 4) & 0x0F; // 7-4位: 运行状态
        let reserved_flag = (byte0 & 0x08) != 0; // 3位: 预留标志位
        let height_type = (byte0 & 0x06) >> 1; // 2位: 高度类型位 (00, 01, 10, 11)
        let track_direction = byte0 & 0x01 ; // 1位: 航迹角方向标志
        let speed_multiplier = byte0 & 0x01 ; // 0位: 速度乘数

        // 解析后续字节
        let track_angle = data[1];         // 第2字节: 航迹角 (0-179)
        let ground_speed = data[2] as i8;  // 第3字节: 地速 (有符号)
        let vertical_speed = data[3] as i8; // 第4字节: 垂直速度 (有符号)

        // 解析纬度、经度 (小端序)
        let latitude = i32::from_le_bytes([
            data[4], data[5], data[6], data[7]
        ]);
        let longitude = i32::from_le_bytes([
            data[8], data[9], data[10], data[11]
        ]);

        // 解析高度值 (小端序)
        let pressure_altitude = i16::from_le_bytes([data[12], data[13]]);
        let geometric_altitude = i16::from_le_bytes([data[14], data[15]]);
        let ground_altitude = i16::from_le_bytes([data[16], data[17]]);

        // 解析精度值
        let byte18 = data[18];
        let vertical_accuracy = byte18 >> 4;     // 高4位: 垂直精度
        let horizontal_accuracy = byte18 & 0x0F; // 低4位: 水平精度
        
        let byte19 = data[19];
        let speed_accuracy = byte19 & 0x0F;      // 低4位: 速度精度

        // 解析时间戳和小端序
        let timestamp = u16::from_le_bytes([data[20], data[21]]);
        
        // 解析时间戳精度和保留字段
        let byte22 = data[22];
        let timestamp_accuracy = byte22 & 0x0F; // 低4位: 时间戳精度
        let reserved = data[23];                // 保留字段

        Ok(Self {
            run_status,
            reserved_flag,
            height_type,
            track_direction,
            speed_multiplier,
            track_angle,
            ground_speed,
            vertical_speed,
            latitude,
            longitude,
            pressure_altitude,
            geometric_altitude,
            ground_altitude,
            vertical_accuracy,
            horizontal_accuracy,
            speed_accuracy,
            timestamp,
            timestamp_accuracy,
            reserved,
        })
    }

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        let message_type = MessageType::PositionVectorMessageType as u8;
        let message_protocol = (message_type << 4) | 0x01;
        bytes.push(message_protocol);
        // 第1字节编码
        let mut byte1 = (self.run_status << 4) as u8;
        byte1 |= (self.reserved_flag as u8) << 3;
        byte1 |= (self.height_type & 0x03) << 2;
        byte1 |= self.track_direction << 1;
        byte1 |= self.speed_multiplier;
        bytes.push(byte1);
        
        // 第2-4字节
        bytes.push(self.track_angle);
        bytes.push(self.ground_speed as u8);
        bytes.push(self.vertical_speed as u8);
        
        // 经纬度编码（小端序）
        bytes.extend_from_slice(&self.latitude.to_le_bytes());
        bytes.extend_from_slice(&self.longitude.to_le_bytes());
        
        // 高度字段
        bytes.extend_from_slice(&self.pressure_altitude.to_le_bytes());
        bytes.extend_from_slice(&self.geometric_altitude.to_le_bytes());
        bytes.extend_from_slice(&self.ground_altitude.to_le_bytes());
        
        // 精度和时间戳
        let accuracy_byte = (self.vertical_accuracy << 4) | (self.horizontal_accuracy & 0x0F);
        bytes.push(accuracy_byte);
        bytes.push(self.speed_accuracy & 0x0F);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        
        // 最后2字节
        bytes.push((self.timestamp_accuracy << 4) | (self.reserved & 0x0F));
        bytes.push(self.reserved);
        
        bytes
    }

    
    fn print(&self) {
        println!("=== PositionVectorMessage ===");
        println!("运行状态: 0x{:X}", self.run_status);
        println!("高度类型: {}", self.height_type);
        println!("航迹方向: {}", if self.track_direction == 1 { "西" } else { "东" });
        println!("航迹角: {}° (完整: {}°)", self.track_angle, self.calculate_full_track_angle());
        println!("地速: {}节 (×{})", self.calculate_ground_speed_knots(), 
                 if self.speed_multiplier == 1 { 10 } else { 1 });
        println!("垂直速度: {} m/s", self.vertical_speed);
        println!("位置: ({}, {})", 
                 self.latitude , 
                 self.longitude);
        println!("高度: 气压={}m, 几何={}m, 距地={}m", 
                 self.pressure_altitude, self.geometric_altitude, self.ground_altitude);
        println!("精度: 垂直={}, 水平={}, 速度={}", 
                 self.vertical_accuracy, self.horizontal_accuracy, self.speed_accuracy);
        println!("时间戳: {} (0.1秒)", self.timestamp);
        println!("时间精度: {}", self.timestamp_accuracy);
        println!("预留: {:02X}", self.reserved);
    }
}
