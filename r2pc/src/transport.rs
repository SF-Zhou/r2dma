use super::{Error, Result};
use crate::ConnectionPool;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

#[derive(Clone, Debug)]
pub enum Transport {
    SyncTcpStream {
        pool: Arc<ConnectionPool>,
        addr: SocketAddr,
    },
    AsyncTcpStream(Arc<Mutex<tokio::net::TcpStream>>),
}

impl Transport {
    pub fn new_sync(pool: Arc<ConnectionPool>, addr: SocketAddr) -> Self {
        Self::SyncTcpStream { pool, addr }
    }

    pub fn new_async(stream: tokio::net::TcpStream) -> Self {
        Self::AsyncTcpStream(Arc::new(Mutex::new(stream)))
    }

    pub async fn request(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match self {
            Transport::SyncTcpStream { pool, addr } => {
                let mut stream = pool.acquire(*addr).await?;
                stream
                    .write_all(bytes)
                    .await
                    .map_err(|e| Error::SocketError(e.to_string()))?;
                let len = stream
                    .read_u64()
                    .await
                    .map_err(|e| Error::SocketError(e.to_string()))?;

                let mut bytes = vec![0u8; len as usize];
                stream
                    .read_exact(&mut bytes)
                    .await
                    .map_err(|e| Error::SocketError(e.to_string()))?;
                pool.restore(*addr, stream);
                Ok(bytes)
            }
            Transport::AsyncTcpStream(_) => todo!(),
        }
    }

    pub async fn send(&self, bytes: &[u8]) -> Result<()> {
        match self {
            Transport::SyncTcpStream { pool: _, addr: _ } => {
                Err(Error::SocketError("invalid op!".into()))
            }
            Transport::AsyncTcpStream(tcp_stream) => {
                let mut socket = tcp_stream.lock().await;
                socket
                    .write_all(bytes)
                    .await
                    .map_err(|e| Error::SocketError(e.to_string()))
            }
        }
    }

    pub async fn recv(&self) -> Result<Vec<u8>> {
        match self {
            Transport::SyncTcpStream { pool: _, addr: _ } => {
                Err(Error::SocketError("invalid op!".into()))
            }
            Transport::AsyncTcpStream(tcp_stream) => {
                let mut socket = tcp_stream.lock().await;
                let len = socket
                    .read_u64()
                    .await
                    .map_err(|e| Error::SocketError(e.to_string()))?;

                let mut bytes = vec![0u8; len as usize];
                socket
                    .read_exact(&mut bytes)
                    .await
                    .map_err(|e| Error::SocketError(e.to_string()))?;

                drop(socket);
                Ok(bytes)
            }
        }
    }
}
