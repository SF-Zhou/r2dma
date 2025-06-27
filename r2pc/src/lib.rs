#![feature(return_type_notation)]

mod error;
pub use error::{Error, ErrorKind, Result};

mod msg;
pub use msg::{Msg, MsgFlags, MsgMeta};

mod socket;
pub use socket::{Socket, TcpSocket};

mod state;
pub use state::{Method, MsgWaiter, Services, State};

mod socket_pool;
pub use socket_pool::{SocketPool, TcpSocketPool};

mod context;
pub use context::{Context, SocketGetter};

mod server;
pub use server::Server;

mod client;
pub use client::Client;

mod core_services;
pub use core_services::*;

pub use r2pc_macro::service;
