mod tcp_server;
mod hub;
mod tcp_client;

use async_trait::async_trait;
use uuid::Uuid;
use crate::protocol::Packet;

// 用于抽象传输层
#[async_trait]
pub trait Transport{
    async fn send(&self, uuid: Uuid, packet: Packet) -> Result<(), TransportError>;
    async fn receive(&mut self) -> impl Future<Output = Option<(Uuid,Packet)>>;
    async fn close(&mut self) -> Result<(), TransportError>;
}

// 传输错误类型
#[derive(Debug)]
pub enum TransportError {
    MsgError,
    CloseError,
    Io(std::io::Error),
    ConnectionNotFound,
    SendError,
    ReceiveError,
}