use crate::protocol::Packet;
use crate::transport::{Transport, TransportError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct TcpClientTransport{
    input_sender:mpsc::Sender<Packet>,
    input_receiver: mpsc::Receiver<Packet>,
    main_handle: JoinHandle<()>,
}

impl TcpClientTransport{
    pub async fn connect(addr: impl ToSocketAddrs) -> Result<Self, TransportError> {
        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(TransportError::Io)?;
        let (input_sender, mut output_receiver) = mpsc::channel(100);
        let (output_sender, input_receiver) = mpsc::channel(100);
        let main_handle = tokio::spawn(async move{
            let (mut read_half, mut write_half) = stream.into_split();
            // 用于接收信息并外送
            tokio::spawn(async move{
                while let Some(packet) = output_receiver.recv().await {
                    if let Err(e) = write_half.write_all(&packet.to_bytes()).await {
                        eprintln!("Write error: {}", e);
                        break;
                    }
                }
            });

            // 读取任务
            loop {
                match Self::read_packet(&mut read_half).await {
                    Ok(packet) => {
                        let uuid = packet.header.session_id;
                        if output_sender.send((uuid, packet)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Read error: {:?}", e);
                        break;
                    }
                }
            }
        });
        Ok(TcpClientTransport {
            input_sender,
            input_receiver,
            main_handle,
        })
    }

    async fn read_packet(stream: &mut OwnedReadHalf) -> Result<Packet, TransportError> {
        let mut header = [0u8; 64];
        stream.read_exact(&mut header).await.map_err(|_| TransportError::ReceiveError)?;

        let payload_len = u32::from_le_bytes(header[16..20].try_into().unwrap()) as usize;
        let mut packet_data = Vec::with_capacity(64 + payload_len);
        packet_data.extend_from_slice(&header);

        packet_data.resize(64 + payload_len, 0);
        stream.read_exact(&mut packet_data[64..]).await.map_err(|_| TransportError::ReceiveError)?;

        Packet::from_bytes(&packet_data).map_err(|_| TransportError::MsgError)
    }
}

impl Transport for TcpClientTransport{
    async fn send(&self, uuid: Uuid, packet: Packet) -> Result<(), TransportError> {
        self.input_sender.send(packet)
            .await
            .map_err(|_| TransportError::SendError)?;
        Ok(())
    }

    async fn receive(&mut self) -> impl Future<Output=Option<(Uuid, Packet)>> {
        self.input_receiver.recv()
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        // 关闭主任务
        if let Some(handle) = self.main_handle.take() {
            handle.abort();
        }
    }
}