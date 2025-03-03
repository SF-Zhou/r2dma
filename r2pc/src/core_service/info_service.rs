use crate::{Context, Result, service};

#[service]
pub trait InfoService {
    async fn list_methods(&self, ctx: &Context, v: &()) -> Result<Vec<String>>;
}

impl InfoService for super::CoreServiceImpl {
    async fn list_methods(&self, ctx: &Context, _: &()) -> Result<Vec<String>> {
        let server = ctx.server.as_ref().unwrap();
        Ok(server.methods.keys().cloned().collect())
    }
}
