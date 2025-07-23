
pub mod message;
pub mod base_message;
pub mod position_vector_message;
pub mod system_message;
pub mod packet_message;
use tracing::info;

use crate::message::message::Message;

pub enum AnyMessage {
    Base(base_message::BaseMessage),
    PositionVector(position_vector_message::PositionVectorMessage),
    System(system_message::SystemMessage)
}

impl AnyMessage {
    /// 工厂方法 - 根据首字节的消息类型创建具体的消息实例
    pub fn from_bytes(data: &[u8]) -> Result<Self, message::MessageError> {

        if data.is_empty() {
            return Err(message::MessageError::InsufficientLength(1, 0));
        }
        let message_type = (data[0] >> 4) & 0x0f;
        let content = &data[1..];
        info!("message type = {}", message_type);
        match message_type {
            base_message::BaseMessage::MESSAGE_TYPE => {
                base_message::BaseMessage::from_bytes(content).map(AnyMessage::Base)
            },
            position_vector_message::PositionVectorMessage::MESSAGE_TYPE => {
                position_vector_message::PositionVectorMessage::from_bytes(content).map(AnyMessage::PositionVector)
            },
            system_message::SystemMessage::MESSAGE_TYPE => {
                system_message::SystemMessage::from_bytes(content).map(AnyMessage::System)
            },
            t => Err(message::MessageError::UnknownMessageType(t)),
        }
    }
    
    pub fn print(&self) {
        match self {
            AnyMessage::Base(msg) => msg.print(),
            AnyMessage::PositionVector(msg) => msg.print(),
            AnyMessage::System(msg) => msg.print(),
        }
    }
}