use std::str;
use tracing::info;
use serde::{Serialize, Deserialize};

use crate::message::message::MessageType;

use super::message::{Message, MessageError};

/// 基本类型，主要包含了RID的字符串
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseMessage {
    pub id_type: u8,          // 高位 4 位 (7-4 位)
    pub ua_type: u8,          // 低位 4 位 (3-0 位)
    pub uas_id: String,       // UAS 识别身份信息（字符串类型）
    #[serde(default)]
    pub reserved: [u8; 3],    // 3 字节预留空间
}

impl BaseMessage {
    pub const MESSAGE_TYPE: u8 = 0x00;
    const EXPECTED_LENGTH: usize = 24;
}

impl Message for BaseMessage {
    /// 从 u8 数组解析为结构化数据
    ///
    /// # 参数
    /// - `data`: 至少包含 24 字节的输入数据
    ///
    /// # 错误
    /// - 当输入数据长度不足时返回 ParseError::InsufficientLength
    /// - 当 UAS ID 不是有效的 UTF-8 时返回 ParseError::InvalidUtf8
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError> {
        if data.len() < Self::EXPECTED_LENGTH{
            return Err(MessageError::InsufficientLength(
                Self::EXPECTED_LENGTH, 
                data.len()
            ));
        }

        // 解析第一个字节 (起始字节 1)
        let byte0 = data[0];
        let id_type = (byte0 >> 4) & 0x0F;  // 提取高4位 (7-4位)
        let ua_type = byte0 & 0x0F;         // 提取低4位 (3-0位)
        info!("id type={}, ua_type={}", id_type, ua_type);
        // 解析 UAS ID (起始字节 2，长度 20)
        let uas_id_start = 1;
        let uas_id_bytes = &data[uas_id_start..uas_id_start + 20];
        
        // 转换为 String，移除尾部的空字符(\0)和空白字符
        let uas_id = match str::from_utf8(uas_id_bytes) {
            Ok(s) => {
                // 移除尾部的空字符和空白字符
                s.trim_end_matches('\0')
                 .trim_end()
                 .to_string()
            },
            Err(e) => {
                info!("base message utf8 error.");
                return Err(MessageError::InvalidUtf8(e))
            }
        };

        // 解析预留字段 (起始字节 22)
        //let reserved_start = 21;  // 起始索引 = 起始字节 - 1
        let reserved = [0,0,0];
            

        Ok(Self {
            id_type,
            ua_type,
            uas_id,
            reserved,
        })
    }

    fn encode(&self) -> Vec<u8> {
        let mut bytes:Vec<u8> = Vec::new();
        
        let message_type = MessageType::BaseMessageType as u8;
        let message_protocol = (message_type << 4) | 0x01;
        bytes.push(message_protocol);
        // 编码第一个字节：id_type（高4位） + ua_type（低4位）
        let type_byte = (self.id_type << 4) | (self.ua_type & 0x0F);
        bytes.push(type_byte);
        
        // 编码UAS ID（最多20字节）
        let uas_bytes = self.uas_id.as_bytes().to_vec();
        bytes.extend_from_slice(&uas_bytes);
        
        //不足的位置写0
        let id_len = uas_bytes.len();
        let reserved = vec![0u8; 23-id_len];
        bytes.extend(&reserved);
        
        bytes
    }


    fn print(&self) {
        println!("=== BaseMessage ===");
        println!("ID 类型: 0x{:X}", self.id_type);
        println!("UA 类型: 0x{:X}", self.ua_type);
        println!("UAS ID: '{}'", self.uas_id);
        println!("预留字段: {:02X?}", self.reserved);
    }
}

