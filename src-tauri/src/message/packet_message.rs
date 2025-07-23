use crate::message::{base_message::BaseMessage, position_vector_message::PositionVectorMessage, system_message::SystemMessage};
use super::message::{Message, MessageError};
use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicU8, Ordering};

static RID_COUNTER: AtomicU8 = AtomicU8::new(1);

/// 以整包形式发送，其中包含了BaseMessage， SystemMessage, PositionVectorMessage，主要模仿收到大疆的结构类型
#[derive(Debug, Serialize, Deserialize)]
pub struct PacketMessage {
    protocol_version: u8,          // 协议版本（1字节）
    message_counter: u8,          // 消息计数器（2字节）
    message_size: u8,             // 消息总大小（2字节）
    message_quantity: u8,          // 包含消息数量（1字节）
    base_message: BaseMessage,
    system_message: SystemMessage,
    position_message: PositionVectorMessage,
    checksum: u16,                 // CRC16校验和（2字节）
    reserved: [u8; 3],             // 3字节预留
}

impl PacketMessage {
    // 每一帧的大小
    const MESSAGE_SIZE:u8 = 25;
    // 每包一共3帧
    const MESSAGE_QUANTITY:u8 = 3;
    pub fn new(
        base: BaseMessage,
        system: SystemMessage,
        position: PositionVectorMessage
    ) -> Self {
        Self {
            protocol_version: 0xf1,
            message_counter: 1,
            message_size: Self::MESSAGE_SIZE,
            message_quantity: Self::MESSAGE_QUANTITY,
            base_message: base,
            system_message: system,
            position_message: position,
            checksum: 0,
            reserved: [0; 3],
        }
    }
    // 获取rid加前缀为ssid，仿大疆
    pub fn get_ssid(&self) -> String {
        return format!("RID-{}", self.base_message.uas_id.clone());
    }
}

impl Message for PacketMessage {
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError> {
        if data.len() < 16 {
            return Err(MessageError::InsufficientLength(16, data.len()));
        }

        // 解析头部
        let message_counter = data[1];
        let protocol_version = data[2];
        let message_size = data[3];
        let message_quantity = data[4];
        
        // 解析消息体
        let base = BaseMessage::from_bytes(&data[5..29])?;
        let system = SystemMessage::from_bytes(&data[29..61])?;
        let position = PositionVectorMessage::from_bytes(&data[61..85])?;
        
        // 解析尾部
        let checksum = u16::from_le_bytes([data[85], data[86]]);
        let reserved = [data[87], data[88], data[89]];

        Ok(Self {
            protocol_version,
            message_counter,
            message_size,
            message_quantity,
            base_message: base,
            system_message: system,
            position_message: position,
            checksum,
            reserved,
        })
    }

    fn encode(&self) -> Vec<u8> {
        self.print();
        let mut bytes = Vec::new();
        
        // 编码头部
        let rid_counter: u8 = RID_COUNTER.fetch_add(0x01, Ordering::SeqCst); // 序列号按802.11规范递增

        bytes.push(rid_counter);
        bytes.push(self.protocol_version);
        
        bytes.push(self.message_size);
        bytes.push(self.message_quantity);
        
        // 编码子消息
        bytes.extend(self.base_message.encode());
        bytes.extend(self.position_message.encode());
        bytes.extend(self.system_message.encode());

        
        // 计算校验和
        let checksum = crc16::State::<crc16::XMODEM>::calculate(&bytes);
        bytes.extend_from_slice(&checksum.to_le_bytes());
        
        // 添加预留字段
        bytes.extend_from_slice(&self.reserved);
        
        bytes
    }



    fn print(&self) {
        println!("=== Packet Message ===");
        println!("Protocol Version: 0x{:02X}", self.protocol_version);
        println!("Message Counter: {}", self.message_counter);
        println!("Total Size: {} bytes", self.message_size);
        println!("Contains {} messages", self.message_quantity);
        
        println!("\nBase Message:");
        self.base_message.print();
        
        println!("\nSystem Message:");
        self.system_message.print();
        
        println!("\nPosition Message:");
        self.position_message.print();
        
        println!("\nChecksum: 0x{:04X}", self.checksum);
    }
}
