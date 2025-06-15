use super::*;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct CoreState {
    pub services: Services,
    pub msg_waiter: MsgWaiter,
}

impl CoreState {
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
                core_state: self.clone(),
            };
            self.services.invoke(ctx, msg);
        } else {
            self.msg_waiter.post(msg.meta.msg_id, Ok(msg));
        }
        Ok(())
    }
}
