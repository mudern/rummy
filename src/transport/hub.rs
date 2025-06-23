use crate::protocol::Packet;
use crate::transport::TransportError;

type PacketMessage = (Result<Packet,TransportError>, u8);

struct Agent {
    id: u16,

}

struct Hub{
    agents: Vec<Agent>,
}