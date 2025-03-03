use super::*;
use std::net::SocketAddr;

pub enum SocketGetter {
    TcpSocket(TcpSocket),
    TcpSocketGetter(TcpSocketManager, SocketAddr),
    RdmaSocket(RdmaSocket),
    RdmaSocketGetter(RdmaSocketManager, SocketAddr),
}
