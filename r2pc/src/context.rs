use super::{Server, Transport};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Context {
    pub tr: Transport,
    pub server: Option<Arc<Server>>,
}

impl Context {
    pub fn new(tr: Transport) -> Self {
        Self { tr, server: None }
    }
}
