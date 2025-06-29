#![forbid(unsafe_code)]
#![feature(return_type_notation)]

mod basics;
pub use basics::*;

mod services;
pub use services::*;

mod states;
pub use states::*;

mod server;
pub use server::Server;

mod client;
pub use client::Client;

pub use r2pc_macro::service;
