#![feature(return_type_notation)]
use r2pc::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request(pub String);

#[r2pc::service]
pub trait EchoService {
    async fn echo(&self, c: &Context, r: &Request) -> Result<String>;
}

#[r2pc::service]
pub trait GreetService {
    async fn greet(&self, c: &Context, r: &Request) -> Result<String>;
}
