use super::{Error, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

#[derive(Debug)]
pub struct Transport {
    socket: Mutex<tokio::net::TcpStream>,
}
pub type TransportPtr = std::sync::Arc<Transport>;

impl Transport {
    pub async fn send(&self, bytes: &[u8]) -> Result<()> {
        let mut socket = self.socket.lock().await;
        socket
            .write_all(bytes)
            .await
            .map_err(|e| Error::SocketError(e.to_string()))
    }

    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut socket = self.socket.lock().await;
        let len = socket
            .read_u64()
            .await
            .map_err(|e| Error::SocketError(e.to_string()))?;

        let mut bytes = vec![0u8; len as usize];
        socket
            .read_exact(&mut bytes)
            .await
            .map_err(|e| Error::SocketError(e.to_string()))?;
        Ok(bytes)
    }

    #[cfg(test)]
    pub async fn create_for_test() -> TransportPtr {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let socket = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        std::sync::Arc::new(Transport {
            socket: Mutex::new(socket),
        })
    }
}
