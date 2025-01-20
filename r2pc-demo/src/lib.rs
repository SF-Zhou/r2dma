#![feature(return_type_notation)]
use r2pc::{Context, Result};

#[r2pc::service]
pub trait EchoService {
    async fn echo(&self, c: &Context, r: &String) -> Result<String>;
}

#[r2pc::service]
pub trait GreetService {
    async fn greet(&self, c: &Context, r: &String) -> Result<String>;
}
