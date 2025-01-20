use crate::*;
use derse::Deserialize;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::TcpStream;

pub type Method = Box<dyn Fn(&Context, Meta, &[u8]) -> Result<()> + Send + Sync>;

#[derive(Default)]
pub struct Server {
    stop_token: tokio_util::sync::CancellationToken,
    methods: HashMap<String, Method>,
}

impl Server {
    pub fn add_methods(&mut self, methods: HashMap<String, Method>) {
        self.methods.extend(methods);
    }

    pub fn stop(&self) {
        self.stop_token.cancel();
    }

    pub async fn listen(
        self: Arc<Self>,
        addr: SocketAddr,
    ) -> std::io::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let listener_addr = listener.local_addr()?;
        let stop_token = self.stop_token.clone();

        let listen_routine = tokio::spawn(async move {
            tokio::select! {
                _ = stop_token.cancelled() => {
                    tracing::info!("stop accept loop");
                }
                _ = async {
                    while let Ok((socket, addr)) = listener.accept().await {
                        let clone = self.clone();
                        tokio::spawn(async move {
                            tracing::info!("socket {addr} established");
                            match clone.handle(socket).await {
                                Ok(_) => tracing::info!("socket {addr} closed"),
                                Err(err) => tracing::info!("socket {addr} closed with error {err}"),
                            }
                        });
                    }
                } => {}
            }
        });

        Ok((listener_addr, listen_routine))
    }

    pub async fn handle(self: Arc<Self>, socket: TcpStream) -> Result<()> {
        let recv_stream = socket.into_std().unwrap();
        let send_stream = recv_stream.try_clone().unwrap();
        let recv_tr = Transport::new_async(TcpStream::from_std(recv_stream).unwrap());
        let send_tr = Transport::new_async(TcpStream::from_std(send_stream).unwrap());

        loop {
            let bytes = recv_tr.recv().await?;

            let mut buf = bytes.as_slice();
            let meta = Meta::deserialize_from(&mut buf)?;
            if let Some(func) = self.methods.get(&meta.method) {
                let ctx = Context {
                    tr: send_tr.clone(),
                };
                let _ = func(&ctx, meta, buf);
            }
        }
    }
}
