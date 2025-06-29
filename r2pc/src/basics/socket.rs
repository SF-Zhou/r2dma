use super::*;
use bytes::{Bytes, BytesMut};
use serde::Serialize;
use tokio::sync::mpsc;

/// A socket abstraction that can handle both TCP and RDMA sockets.
#[derive(Debug, Clone)]
pub enum Socket {
    TCP(TcpSocket),
    RDMA,
}

impl Socket {
    pub async fn send<P: Serialize>(&self, meta: MsgMeta, payload: &P) -> Result<()> {
        match self {
            Socket::TCP(s) => s.send(meta, payload).await,
            Socket::RDMA => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TcpSocket {
    stream: mpsc::Sender<Bytes>,
}

impl TcpSocket {
    pub fn new(stream: mpsc::Sender<Bytes>) -> Self {
        Self { stream }
    }

    pub async fn send<P: Serialize>(&self, meta: MsgMeta, payload: &P) -> Result<()> {
        let mut bytes = BytesMut::with_capacity(512);
        meta.serialize_to(payload, &mut bytes)?;
        self.stream
            .send(bytes.into())
            .await
            .map_err(|e| Error::new(ErrorKind::TcpSendMsgFailed, e.to_string()))
    }
}

const MSG_HEADER: u32 = u32::from_be_bytes(*b"r2pc");

impl SendMsg for BytesMut {
    fn len(&self) -> usize {
        self.len()
    }

    fn prepare(&mut self) -> Result<()> {
        self.extend(MSG_HEADER.to_be_bytes());
        self.extend([0u8; 8]);
        Ok(())
    }

    fn finish(&mut self, start_offset: usize, meta_len: usize) -> Result<()> {
        let body_len = self.len() - start_offset - 8;
        self[4..8].copy_from_slice(&(body_len as u32).to_be_bytes());
        self[8..12].copy_from_slice(&(meta_len as u32).to_be_bytes());
        Ok(())
    }

    fn writer(&mut self) -> impl std::io::Write {
        #[repr(transparent)]
        struct Writer<'a>(&'a mut BytesMut);

        impl std::io::Write for Writer<'_> {
            #[inline(always)]
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.write_all(buf)?;
                Ok(buf.len())
            }

            #[inline]
            fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
                self.0.extend_from_slice(buf);
                Ok(())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        Writer(self)
    }
}
