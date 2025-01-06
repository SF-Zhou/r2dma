use crate::{Context, Meta};
use derse::{Deserialize, Serialize};

pub struct Client;

impl Client {
    pub async fn call<
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
        ctx.tr.send(&bytes).await?;
        let bytes = ctx.tr.recv().await?;
        let mut buf = bytes.as_slice();
        let _ = Meta::deserialize_from(&mut buf).map_err(Into::into)?;
        std::result::Result::<Rsp, Error>::deserialize_from(&mut buf).map_err(Into::into)?
    }
}
