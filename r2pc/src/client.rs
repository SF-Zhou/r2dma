use crate::{Context, Meta};
use derse::{Deserialize, Serialize};

pub struct Client;

impl Client {
    pub async fn rpc_call<
        Req: Serialize,
        Rsp: for<'c> Deserialize<'c>,
        Error: std::error::Error + From<crate::Error> + for<'c> Deserialize<'c>,
    >(
        &self,
        ctx: &Context,
        req: &Req,
        method_name: &str,
    ) -> std::result::Result<Rsp, Error> {
        let meta = Meta {
            msg_id: Default::default(),
            method: method_name.into(),
            flags: Default::default(),
        };
        let bytes = meta.serialize(req)?;
        let timeout = std::time::Duration::from_secs(1);
        let bytes = match tokio::time::timeout(timeout, ctx.tr.request(&bytes)).await {
            Ok(r) => r?,
            Err(e) => return Err(crate::Error::Timeout(e.to_string()).into()),
        };
        let mut buf = bytes.as_slice();
        let _ = Meta::deserialize_from(&mut buf).map_err(Into::into)?;
        std::result::Result::<Rsp, Error>::deserialize_from(&mut buf).map_err(Into::into)?
    }
}
