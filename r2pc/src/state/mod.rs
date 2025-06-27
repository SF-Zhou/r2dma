mod msg_waiter;
pub use msg_waiter::MsgWaiter;

mod services;
pub use services::{Method, Services};

use crate::*;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct State {
    pub services: Services,
    pub msg_waiter: MsgWaiter,
}

impl State {
    pub fn new(services: Services) -> Arc<Self> {
        Arc::new(Self {
            services,
            msg_waiter: Default::default(),
        })
    }

    pub(crate) fn handle_recv(self: &Arc<Self>, socket: Socket, msg: Msg) -> Result<()> {
        if msg.meta.flags.contains(MsgFlags::IsReq) {
            let ctx = Context {
                socket_getter: SocketGetter::Single(socket),
                state: self.clone(),
            };
            self.services.invoke(ctx, msg);
        } else {
            self.msg_waiter.post(msg.meta.msg_id, Ok(msg));
        }
        Ok(())
    }
}
