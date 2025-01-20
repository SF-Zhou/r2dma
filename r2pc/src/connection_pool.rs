use crate::{Error, Result};
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub struct ConnectionPool {
    max_connection_num: usize,
    map: lockmap::LockMap<SocketAddr, Vec<TcpStream>>,
}

impl ConnectionPool {
    pub fn new(max_connection_num: usize) -> Self {
        Self {
            max_connection_num,
            map: Default::default(),
        }
    }

    pub async fn acquire(&self, addr: SocketAddr) -> Result<TcpStream> {
        let mut entry = self.map.entry(addr);
        if let Some(conns) = entry.get_mut() {
            if let Some(conn) = conns.pop() {
                return Ok(conn);
            }
        }
        drop(entry);

        self.connect(addr).await
    }

    pub fn restore(&self, addr: SocketAddr, stream: TcpStream) {
        let mut entry = self.map.entry(addr);
        if let Some(conns) = entry.get_mut() {
            if conns.len() < self.max_connection_num {
                conns.push(stream);
            }
        } else {
            entry.insert(vec![stream]);
        }
    }

    async fn connect(&self, addr: SocketAddr) -> Result<TcpStream> {
        match tokio::time::timeout(std::time::Duration::from_secs(1), TcpStream::connect(&addr))
            .await
        {
            Ok(r) => r.map_err(|e| Error::SocketError(e.to_string())),
            Err(e) => Err(Error::SocketError(e.to_string())),
        }
    }
}

impl std::fmt::Debug for ConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionPool")
            .field("max_connection_num", &self.max_connection_num)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    pub async fn test_connection_pool() {
        let listener = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let pool = ConnectionPool::new(2);
        let stream = pool.acquire(addr).await.unwrap();
        pool.restore(addr, stream);

        let stream1 = pool.acquire(addr).await.unwrap();
        let stream2 = pool.acquire(addr).await.unwrap();
        pool.restore(addr, stream1);
        pool.restore(addr, stream2);
    }
}
