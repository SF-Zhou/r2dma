use super::{Meta, TransportPtr};

pub struct CallContext {
    pub meta: Meta,
    pub tr: TransportPtr,
}
