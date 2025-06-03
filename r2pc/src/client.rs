use crate::{Context, DeserializePackage, Meta};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct Client {
    timeout: Duration,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(1),
        }
    }
}

impl Client {
    pub async fn rpc_call<Req, Rsp, Error>(
        &self,
        ctx: &Context,
        req: &Req,
        method_name: &str,
    ) -> std::result::Result<Rsp, Error>
    where
        Req: Serialize,
        Rsp: for<'c> Deserialize<'c>,
        Error: std::error::Error + From<crate::Error> + for<'c> Deserialize<'c>,
    {
        let meta = Meta {
            msg_id: Default::default(),
            method: method_name.into(),
            flags: Default::default(),
        };
        let bytes = meta.serialize(req)?;
        let bytes = match tokio::time::timeout(self.timeout, ctx.tr.request(&bytes)).await {
            Ok(r) => r?,
            Err(e) => return Err(crate::Error::Timeout(e.to_string()).into()),
        };
        let buf = bytes.as_slice();
        let package: DeserializePackage<std::result::Result<Rsp, Error>> =
            rmp_serde::from_slice(buf).map_err(crate::Error::from)?;
        package.payload
    }
}
