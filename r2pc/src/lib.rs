#![feature(return_type_notation)]

mod error;
pub use error::{Error, ErrorKind, Result};

mod msg;
pub use msg::{Msg, MsgFlags, MsgMeta};

mod socket;
pub use socket::{Socket, TcpSocket};

mod msg_waiter;
pub use msg_waiter::MsgWaiter;

mod services;
pub use services::{Method, Services};

mod core_state;
pub use core_state::CoreState;

mod socket_pool;
pub use socket_pool::{SocketPool, TcpSocketPool};

mod context;
pub use context::{Context, SocketGetter};

mod server;
pub use server::Server;

mod client;
pub use client::Client;

mod core_service;
pub use core_service::*;

pub use r2pc_macro::service;
