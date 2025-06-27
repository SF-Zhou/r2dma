use serde::Serialize;

use super::*;
use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub enum SocketGetter {
    Single(Socket),
    FromPool(Arc<TcpSocketPool>, SocketAddr),
}

impl SocketGetter {
    pub async fn get_socket(&self) -> Result<Socket> {
        match self {
            SocketGetter::Single(socket) => Ok(socket.clone()),
            SocketGetter::FromPool(tcp_socket_pool, socket_addr) => {
                tcp_socket_pool.acquire(socket_addr).await
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    pub socket_getter: SocketGetter,
    pub state: Arc<State>,
}

impl Context {
    pub async fn get_socket(&self) -> Result<Socket> {
        self.socket_getter.get_socket().await
    }

    pub async fn send_rsp<Rsp, E>(&self, mut meta: MsgMeta, rsp: std::result::Result<Rsp, E>)
    where
        Rsp: Serialize,
        E: std::error::Error + From<crate::Error> + Serialize,
    {
        meta.flags.remove(MsgFlags::IsReq);
        match Msg::serialize(meta, &rsp) {
            Ok(bytes) => match &self.socket_getter {
                SocketGetter::Single(socket) => {
                    if let Err(e) = socket.send(bytes).await {
                        tracing::error!("send rsp failed: {e}");
                    }
                }
                SocketGetter::FromPool(_, _) => {
                    tracing::error!("send rsp failed: invalid socket");
                }
            },
            Err(e) => {
                tracing::error!("serialize rsp failed: {e}");
            }
        }
    }
}
