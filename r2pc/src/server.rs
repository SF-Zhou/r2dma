use super::*;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::TcpStream;

pub type Method = Box<dyn Fn(&Context, Meta, rmpv::Value) -> Result<()> + Send + Sync>;

pub struct Server {
    stop_token: tokio_util::sync::CancellationToken,
    pub methods: HashMap<String, Method>,
}

impl Default for Server {
    fn default() -> Self {
        let mut this = Self {
            stop_token: Default::default(),
            methods: Default::default(),
        };

        let core_service = Arc::new(CoreServiceImpl);
        this.add_methods(InfoService::rpc_export(core_service.clone()));
        this
    }
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
            let buf = bytes.as_slice();
            let package: DeserializePackage<rmpv::Value> = rmp_serde::from_slice(buf)?;
            let DeserializePackage { meta, payload } = package;
            if let Some(func) = self.methods.get(&meta.method) {
                let ctx = Context {
                    tr: send_tr.clone(),
                    server: Some(self.clone()),
                };
                let _ = func(&ctx, meta, payload);
            }
        }
    }
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server").finish()
    }
}
