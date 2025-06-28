use crate::{Context, Result, service};

#[service]
pub trait InfoService {
    async fn list_methods(&self, ctx: &Context, v: &()) -> Result<Vec<String>>;
}

impl InfoService for () {
    async fn list_methods(&self, ctx: &Context, _: &()) -> Result<Vec<String>> {
        Ok(ctx.state.services.method_names().cloned().collect())
    }
}
