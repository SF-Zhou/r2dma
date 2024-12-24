use super::{Error, Meta, Result};
use derse::Serialize;
use tokio::io::AsyncWriteExt;

pub struct Transport {
    socket: tokio::net::TcpStream,
}
pub type TransportPtr = std::sync::Arc<Transport>;

impl Transport {
    pub async fn send<T: derse::Serialize>(&mut self, meta: &Meta, msg: &T) -> Result<()> {
        let mut bytes = msg.serialize::<derse::DownwardBytes>()?;
        meta.serialize_to(&mut bytes)?;
        let len = bytes.len();
        self.socket
            .write_u64(len as u64)
            .await
            .map_err(|e| Error::SocketError(e.to_string()))?;
        self.socket
            .write_all(&bytes)
            .await
            .map_err(|e| Error::SocketError(e.to_string()))?;
        Ok(())
    }
}
