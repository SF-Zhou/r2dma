use super::*;
use tokio::sync::mpsc;

/// A socket abstraction that can handle both TCP and RDMA sockets.
#[derive(Debug, Clone)]
pub enum Socket {
    TCP(TcpSocket),
    RDMA,
}

impl Socket {
    pub async fn send(&self, msg: Msg) -> Result<()> {
        match self {
            Socket::TCP(s) => s.send(msg).await,
            Socket::RDMA => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TcpSocket {
    stream: mpsc::Sender<Msg>,
}

impl TcpSocket {
    pub fn new(stream: mpsc::Sender<Msg>) -> Self {
        Self { stream }
    }

    pub async fn send(&self, msg: Msg) -> Result<()> {
        self.stream
            .send(msg)
            .await
            .map_err(|e| Error::new(ErrorKind::TcpSendMsgFailed, e.to_string()))
    }
}
