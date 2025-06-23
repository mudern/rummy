use std::collections::HashMap;
use std::sync::Arc;
use crate::protocol::Packet;
use crate::transport::{Transport, TransportError};
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use uuid::{Uuid};

pub struct TcpServerTransport {
    listener: Arc<Mutex<TcpListener>>,
    connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Packet>>>>,
    output_receiver: mpsc::Receiver<(Uuid, Packet)>,
    main_handle: Option<JoinHandle<()>>,
    output_sender: mpsc::Sender<(Uuid, Packet)>,
}

impl TcpServerTransport {
    pub async fn new(addr: impl ToSocketAddrs) -> Result<Self, TransportError> {
        log::info!("Starting TCP server on {:?}", addr);
        let listener = TcpListener::bind(addr)
            .await
            .map_err(TransportError::Io)?;

        let listener = Arc::new(Mutex::new(listener));

        let (output_sender, output_receiver) = mpsc::channel(100);

        Ok(TcpServerTransport {
            listener,
            connections: Arc::new(Mutex::new(HashMap::new())),
            output_receiver,
            main_handle: None,
            output_sender,
        })
    }

    pub fn run(&mut self) {
        let listener = Arc::clone(&self.listener);
        let connections = Arc::clone(&self.connections);
        let output_sender = self.output_sender.clone();

        self.main_handle = Some(tokio::spawn(async move {
            log::info!("TCP server main loop started");
            loop {
                let stream = {
                    let mut locked = listener.lock().await;
                    locked.accept().await
                };

                match stream {
                    Ok((stream, peer_addr)) => {
                        let uuid = Uuid::new_v4();
                        log::info!("New connection accepted: {} - assigned UUID {}", peer_addr, uuid);
                        let (write_sender, write_receiver) = mpsc::channel(100);
                        connections.lock().await.insert(uuid, write_sender);

                        Self::handle_connection(
                            stream,
                            uuid,
                            output_sender.clone(),
                            write_receiver,
                            Arc::clone(&connections),
                        );
                    }
                    Err(e) => {
                        log::error!("Accept error: {}", e);
                        break;
                    }
                }
            }
            log::warn!("TCP server main loop exited");
        }));
    }

    fn handle_connection(
        stream: TcpStream,
        uuid: Uuid,
        output_sender: mpsc::Sender<(Uuid, Packet)>,
        mut write_receiver: mpsc::Receiver<Packet>,
        connections: Arc<Mutex<HashMap<Uuid, mpsc::Sender<Packet>>>>
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            log::info!("Connection handler started for UUID {}", uuid);

            let (mut read_half, mut write_half) = stream.into_split();

            // 写入任务
            let write_handle = tokio::spawn(async move {
                while let Some(packet) = write_receiver.recv().await {
                    if let Err(e) = write_half.write_all(&packet.to_bytes()).await {
                        log::error!("Write error: {}", e);
                        break;
                    }
                }
                log::info!("Write task ended for connection {}", uuid);
                // 关闭连接时移除
                connections.lock().await.remove(&uuid);
                log::info!("Connection {} removed from active connections", uuid);
            });

            // 读取任务
            loop {
                match Self::read_packet(&mut read_half).await {
                    Ok(packet) => {
                        if output_sender.send((uuid, packet)).await.is_err() {
                            log::warn!("Output receiver closed, stopping read for connection {}", uuid);
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Read error on connection {}: {:?}", uuid, e);
                        break;
                    }
                }
            }

            write_handle.abort();
            log::info!("Connection handler ended for UUID {}", uuid);
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

#[async_trait]
impl Transport for TcpServerTransport {
    async fn send(&self, uuid: Uuid, packet: Packet) -> Result<(), TransportError> {
        log::info!("Sending packet to UUID {}", uuid);
        let guard = self.connections.lock().await;
        guard.get(&uuid)
            .ok_or_else(|| {
                log::warn!("Attempted to send to non-existing connection UUID {}", uuid);
                TransportError::ConnectionNotFound
            })?
            .send(packet)
            .await
            .map_err(|_| {
                log::error!("Failed to send packet to UUID {}", uuid);
                TransportError::SendError
            })
    }

    async fn receive(&mut self) -> impl Future<Output = Option<(Uuid,Packet)>> {
        self.output_receiver.recv()
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        log::info!("Closing TcpServerTransport");
        // 关闭主监听任务
        if let Some(handle) = self.main_handle.take() {
            handle.abort();
            log::info!("Main listener task aborted");
        }

        // 关闭所有连接
        let mut connections = self.connections.lock().await;
        for (uuid, sender) in connections.drain() {
            log::info!("Closing connection {}", uuid);
            drop(sender); // 关闭发送端会终止写入任务
        }
        Ok(())
    }
}