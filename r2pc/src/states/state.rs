use super::*;
use crate::*;
use std::{net::SocketAddr, sync::Arc};

#[derive(Default, Debug)]
pub struct State {
    pub service_manager: ServiceManager,
    pub msg_waiter: MsgWaiter,
    pub socket_pool: TcpSocketPool,
}

impl State {
    pub fn new(service_manager: ServiceManager) -> Arc<Self> {
        Arc::new(Self {
            service_manager,
            msg_waiter: Default::default(),
            socket_pool: Default::default(),
        })
    }

    pub fn client_ctx(self: &Arc<Self>, peer_addr: SocketAddr) -> Context {
        Context::client_ctx(self, peer_addr)
    }

    pub(crate) fn handle_recv(self: &Arc<Self>, socket: Socket, msg: Msg) -> Result<()> {
        if msg.meta.flags.contains(MsgFlags::IsReq) {
            let ctx = Context::server_ctx(self, socket);
            self.service_manager.invoke(ctx, msg);
        } else {
            self.msg_waiter.post(msg.meta.msg_id, Ok(msg));
        }
        Ok(())
    }
}
