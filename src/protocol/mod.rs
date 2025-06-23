const HEADER_SIZE: usize = 64;
const MAGIC: &[u8; 4] = b"rum3";

// 消息类型错误
#[derive(Debug)]
pub enum MsgError {
    InvalidHeader,
    InvalidPayload,
    ChecksumMismatch,
    UnsupportedVersion,
    InvalidMagic,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum MsgType{
    Call = 0u8,
    Reply = 1u8,
    Error = 2u8,
    Auth = 3u8,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketHeader {
    pub magic: [u8; 4],        // 固定魔数 b"rum3"
    pub version: u8,           // 协议版本号
    pub msg_type: MsgType,     // 消息类型
    pub reserved: [u8; 10],    // 预留字段
    pub payload_len: u32,      // 消息体长度（单位：字节）
    pub session_id: u64,       // 会话 ID
    pub timestamp: u64,        // 时间戳（用于超时、认证）
    pub checksum: u32,         // 可选：对 payload 做 CRC32 校验
    pub _padding: [u8; 24],    // 填充到 64 字节
}

impl PacketHeader {
    pub fn from_payload(payload: &[u8], session_id: u64) -> Self{
        let payload_len = payload.len() as u32;
        let checksum = crc32fast::hash(payload);
        PacketHeader {
            magic: *MAGIC,
            version: 1, // 假设当前版本为 1
            msg_type: MsgType::Call, // 默认消息类型为 Call
            reserved: [0; 10], // 预留字段初始化为 0
            payload_len,
            session_id,
            timestamp: chrono::Utc::now().timestamp_millis() as u64, // 当前时间戳
            checksum,
            _padding: [0; 24], // 填充到 64 字节
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_SIZE);
        bytes.extend_from_slice(&self.magic);
        bytes.push(self.version);
        bytes.push(self.msg_type as u8);
        bytes.extend_from_slice(&self.reserved);
        bytes.extend_from_slice(&self.payload_len.to_le_bytes());
        bytes.extend_from_slice(&self.session_id.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes.extend_from_slice(&self._padding);
        bytes
    }

    pub fn from_bytes(buf:&[u8]) -> Result<PacketHeader, MsgError> {
        // 缓冲区长度不足
        if buf.len() < HEADER_SIZE {
            return Err(MsgError::InvalidHeader)
        }

        let magic = buf[0..4].try_into().unwrap();
        let version = buf[4];
        let msg_type = match buf[5] {
            0 => MsgType::Call,
            1 => MsgType::Reply,
            2 => MsgType::Error,
            3 => MsgType::Auth,
            _ => return Err(MsgError::InvalidHeader), // 无效的消息类型
        };
        let reserved = buf[6..16].try_into().unwrap();
        let payload_len = u32::from_le_bytes(buf[16..20].try_into().unwrap());
        let session_id = u64::from_le_bytes(buf[20..28].try_into().unwrap());
        let timestamp = u64::from_le_bytes(buf[28..36].try_into().unwrap());
        let checksum = u32::from_le_bytes(buf[36..40].try_into().unwrap());
        let _padding = buf[40..64].try_into().unwrap();

        Ok(PacketHeader {
            magic,
            version,
            msg_type,
            reserved,
            payload_len,
            session_id,
            timestamp,
            checksum,
            _padding,
        })
    }
}

pub struct Packet {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn new(header: PacketHeader, payload: Vec<u8>) -> Self {
        Packet { header, payload }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, MsgError> {
        let header = PacketHeader::from_bytes(buf)?;
        // 检查 payload 长度是否足够
        if buf.len() < HEADER_SIZE + header.payload_len as usize {
            return Err(MsgError::InvalidPayload); // 缓冲区长度不足
        }
        // 提取 payload
        let payload = buf[HEADER_SIZE..HEADER_SIZE + header.payload_len as usize].to_vec();
        // 检查校验和
        if crc32fast::hash(&payload) != header.checksum {
            return Err(MsgError::ChecksumMismatch); // 校验和不匹配
        }
        // 检查魔数是否正确
        if header.magic != *MAGIC {
            return Err(MsgError::InvalidMagic); // 魔数不匹配
        }
        // 检查版本号是否支持
        if header.version != 1 {
            return Err(MsgError::UnsupportedVersion); // 不支持的版本号
        }
        // 返回 MsgBody 实例
        Ok(Packet { header, payload })
    }
}

