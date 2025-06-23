pub mod utils;
mod auth;

pub enum AuthType{
    ClientHello = 0,
    ServerHello = 1,
    ClientAck = 2,
    ServerAck = 3,
}

impl AuthType {
    pub fn to_u8(&self) -> u8 {
        match self {
            AuthType::ClientHello => 0,
            AuthType::ServerHello => 1,
            AuthType::ClientAck => 2,
            AuthType::ServerAck => 3,
        }
    }

    pub fn from_u8(value: u8) -> Option<AuthType> {
        match value {
            0 => Some(AuthType::ClientHello),
            1 => Some(AuthType::ServerHello),
            2 => Some(AuthType::ClientAck),
            3 => Some(AuthType::ServerAck),
            _ => None,
        }
    }
}

pub struct AuthBody{
    auth_type: AuthType,
    data: Vec<u8>,
}

impl AuthBody{
    pub fn new(auth_type: AuthType, data: Vec<u8>) -> AuthBody {
        AuthBody {
            auth_type,
            data,
        }
    }
    pub fn to_u8(&self) -> Vec<u8> {
        let mut result = vec![self.auth_type.to_u8()];
        result.extend_from_slice(&self.data);
        result
    }

    pub fn from_u8(data: &[u8]) -> Option<AuthBody> {
        if data.is_empty() {
            return None;
        }

        let auth_type = AuthType::from_u8(data[0])?;
        let data_slice = &data[1..];

        Some(AuthBody {
            auth_type,
            data: data_slice.to_vec(),
        })
    }
}

