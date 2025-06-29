use serde::Serialize;

use super::*;
use crate::*;
use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub enum SocketWrapper {
    Single(Socket),
    PeerAddr(SocketAddr),
}

#[derive(Clone, Debug)]
pub struct Context {
    pub state: Arc<State>,
    pub socket: SocketWrapper,
}

impl Context {
    pub fn client_ctx(state: &Arc<State>, peer_addr: SocketAddr) -> Context {
        Context {
            state: state.clone(),
            socket: SocketWrapper::PeerAddr(peer_addr),
        }
    }

    pub fn server_ctx(state: &Arc<State>, socket: Socket) -> Context {
        Context {
            state: state.clone(),
            socket: SocketWrapper::Single(socket),
        }
    }

    pub async fn get_socket(&self) -> Result<Socket> {
        match &self.socket {
            SocketWrapper::Single(socket) => Ok(socket.clone()),
            SocketWrapper::PeerAddr(addr) => {
                self.state.socket_pool.acquire(addr, &self.state).await
            }
        }
    }

    pub async fn send_rsp<Rsp, E>(&self, mut meta: MsgMeta, rsp: std::result::Result<Rsp, E>)
    where
        Rsp: Serialize,
        E: std::error::Error + From<crate::Error> + Serialize,
    {
        meta.flags.remove(MsgFlags::IsReq);
        match &self.socket {
            SocketWrapper::Single(socket) => {
                if let Err(e) = socket.send(meta, &rsp).await {
                    tracing::error!("send rsp failed: {e}");
                }
            }
            _ => {
                tracing::error!("send rsp failed: invalid socket");
            }
        }
    }
}
