#![feature(return_type_notation)]

mod client;
mod connection_pool;
mod constants;
mod context;
mod core_service;
mod error;
mod meta;
mod server;
mod transport;

pub use client::Client;
pub use connection_pool::*;
pub use constants::*;
pub use context::*;
pub use core_service::*;
pub use error::{Error, Result};
pub use meta::*;
pub use server::*;
pub use transport::*;

pub use r2pc_macro::service;
