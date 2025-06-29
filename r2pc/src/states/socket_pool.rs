use crate::*;
use bytes::{Buf, BytesMut};
use foldhash::fast::RandomState;
use std::{io::IoSlice, net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::mpsc,
};

pub trait SocketPool {
    async fn acquire(&self, addr: &SocketAddr, state: &Arc<State>) -> Result<Socket>;
}

#[derive(Default)]
pub struct TcpSocketPool {
    socket_map: dashmap::DashMap<SocketAddr, Socket, RandomState>,
}

const MSG_HEADER: u32 = u32::from_be_bytes(*b"r2pc");
const MAX_MSG_SIZE: usize = 64 << 20;

impl TcpSocketPool {
    pub fn add_socket(
        &self,
        addr: SocketAddr,
        stream: tokio::net::TcpStream,
        state: &Arc<State>,
    ) -> Result<Socket> {
        let (recv_stream, send_stream) = stream.into_split();
        let (sender, receiver) = mpsc::channel(1024);
        tokio::spawn(async move {
            if let Err(_e) = Self::start_send_loop(send_stream, receiver).await {}
        });
        let send_socket = Socket::TCP(TcpSocket::new(sender));
        let send_clone = send_socket.clone();
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::start_recv_loop(recv_stream, send_clone, &state).await {
                tracing::error!("recv loop for {addr} failed: {e}");
                state.socket_pool.socket_map.remove(&addr);
            }
        });
        Ok(send_socket)
    }

    fn parse_message(buffer: &mut BytesMut) -> Result<Option<BytesMut>> {
        const S: usize = std::mem::size_of::<u64>();
        if buffer.len() < S {
            return Ok(None);
        }
        let header = u64::from_be_bytes(buffer[..S].try_into().unwrap());
        if (header >> 32) as u32 != MSG_HEADER {
            return Err(Error::new(
                ErrorKind::TcpParseMsgFailed,
                format!("invalid header: {header:08X}"),
            ));
        }

        let len = header as u32 as usize;
        if len >= MAX_MSG_SIZE {
            return Err(Error::new(
                ErrorKind::TcpParseMsgFailed,
                format!("msg is too long: {len}"),
            ));
        }

        if buffer.len() < S + len {
            Ok(None)
        } else {
            buffer.advance(S);
            Ok(Some(buffer.split_to(len)))
        }
    }

    async fn start_recv_loop(
        mut recv_stream: OwnedReadHalf,
        send_socket: Socket,
        state: &Arc<State>,
    ) -> Result<()> {
        let mut buffer = bytes::BytesMut::with_capacity(1 << 20);
        loop {
            match Self::parse_message(&mut buffer)? {
                Some(bytes) => {
                    let msg = Msg::deserialize_meta(bytes.into())?;
                    state.handle_recv(send_socket.clone(), msg)?;
                }
                None => {
                    let n = recv_stream
                        .read_buf(&mut buffer)
                        .await
                        .map_err(|e| Error::new(ErrorKind::TcpRecvFailed, e.to_string()))?;
                    if n == 0 {
                        return Err(Error::new(
                            ErrorKind::TcpRecvFailed,
                            "socket eof".to_string(),
                        ));
                    }
                }
            }
        }
    }

    async fn start_send_loop(
        mut send_stream: OwnedWriteHalf,
        mut receiver: mpsc::Receiver<Msg>,
    ) -> Result<()> {
        const LIMIT: usize = 64;
        let mut msgs = Vec::with_capacity(LIMIT);
        loop {
            let mut headers = [[0u8; 8]; LIMIT];
            let mut bufs = [IoSlice::new(&[]); LIMIT * 2];

            let n = receiver.recv_many(&mut msgs, LIMIT).await;
            if n == 0 {
                return Ok(());
            }

            for (msg, h) in msgs.iter().zip(headers.iter_mut()) {
                let header = (MSG_HEADER as u64) << 32 | (msg.as_slice().len() as u64);
                *h = header.to_be_bytes();
            }
            let mut offset = 0;
            for (header, msg) in headers.iter().zip(&msgs) {
                bufs[offset] = IoSlice::new(header);
                offset += 1;
                bufs[offset] = IoSlice::new(msg.as_slice());
                offset += 1;
            }

            let mut slices = &mut bufs[..offset];
            while !slices.is_empty() {
                match send_stream.write_vectored(slices).await {
                    Ok(n) => {
                        IoSlice::advance_slices(&mut slices, n);
                    }
                    Err(e) => {
                        return Err(Error::new(ErrorKind::TcpSendFailed, e.to_string()));
                    }
                }
            }
            msgs.clear();
        }
    }
}

impl SocketPool for TcpSocketPool {
    async fn acquire(&self, addr: &SocketAddr, state: &Arc<State>) -> Result<Socket> {
        // Check if the socket is already in the socket map.
        if let Some(socket) = self.socket_map.get(addr) {
            return Ok(socket.clone());
        }
        // If not, create a new socket and insert it into the socket map.
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| Error::new(ErrorKind::TcpConnectFailed, e.to_string()))?;

        let send_socket = self.add_socket(*addr, stream, state).map_err(|e| {
            Error::new(
                ErrorKind::TcpAddSocketFailed,
                format!("failed to add socket for {addr}: {e}"),
            )
        })?;

        self.socket_map.insert(*addr, send_socket.clone());
        Ok(send_socket)
    }
}

impl std::fmt::Debug for TcpSocketPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpSocketPool").finish()
    }
}
