use super::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct Client {
    pub timeout: Duration,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(1),
        }
    }
}

impl Client {
    pub async fn rpc_call<Req, Rsp, E>(
        &self,
        ctx: &Context,
        req: &Req,
        method_name: &str,
    ) -> std::result::Result<Rsp, E>
    where
        Req: Serialize,
        Rsp: for<'c> Deserialize<'c>,
        E: std::error::Error + From<crate::Error> + for<'c> Deserialize<'c>,
    {
        let socket = ctx.get_socket().await?;

        let (msg_id, rx) = ctx.state.msg_waiter.alloc();
        let meta = MsgMeta {
            msg_id,
            flags: MsgFlags::IsReq,
            method: method_name.into(),
        };
        socket.send(meta, req).await?;

        match tokio::time::timeout(self.timeout, rx).await {
            Ok(r) => r
                .map_err(|e| Error::new(ErrorKind::WaitMsgFailed, e.to_string()))?
                .and_then(|msg| msg.deserialize_payload())?,
            Err(e) => {
                ctx.state.msg_waiter.timeout(msg_id);
                Err(Error::new(ErrorKind::Timeout, e.to_string()).into())
            }
        }
    }
}
