use std::fmt;
use std::str;

// 公共消息错误类型
#[derive(Debug, PartialEq)]
pub enum MessageError {
    InsufficientLength(usize, usize),  // 期望长度, 实际长度
    InvalidUtf8(str::Utf8Error),        // UTF-8 格式错误
    UnknownMessageType(u8),             // 未知消息类型
}

// 公共消息类型，目前根据大疆，有3种
#[derive(Debug, PartialEq)]
pub enum MessageType {
    BaseMessageType = 0,
    PositionVectorMessageType = 1,
    SystemMessageType = 4,
}

impl std::error::Error for MessageError {}
impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageError::InsufficientLength(expected, actual) => 
                write!(f, "数据长度不足: 需要 {} 字节, 实际 {} 字节", expected, actual),
            MessageError::InvalidUtf8(e) => 
                write!(f, "文本格式错误: {}", e),
            MessageError::UnknownMessageType(t) => 
                write!(f, "未知消息类型: 0x{:02X}", t),
        }
    }
}

/// 所有消息类型必须实现的 trait
pub trait Message {
    /// 从字节数组解析消息
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError> where Self: Sized;
    // 从结构体到字节的编码
    fn encode(&self) -> Vec<u8> ;
    /// 打印消息内容
    fn print(&self);
}
