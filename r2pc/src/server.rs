use super::*;
use std::{net::SocketAddr, sync::Arc};

pub struct Server {
    state: Arc<State>,
    stop_token: tokio_util::sync::CancellationToken,
}

impl Server {
    pub fn create(service_manager: ServiceManager) -> Self {
        let state = State::new(service_manager);

        Self {
            state,
            stop_token: tokio_util::sync::CancellationToken::new(),
        }
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
                    while let Ok((stream, addr)) = listener.accept().await {
                        if let Err(e) = self.state.socket_pool.add_socket(addr, stream, &self.state) {
                            tracing::error!("failed to add socket {addr}: {e}");
                        } else {
                            tracing::info!("accepted connection from {addr}");
                        }
                    }
                } => {}
            }
        });

        Ok((listener_addr, listen_routine))
    }
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_server_creation() {
        let mut service_manager = ServiceManager::default();
        service_manager.add_methods(InfoService::rpc_export(Arc::new(())));
        let server = Server::create(service_manager);
        let server = Arc::new(server);

        let addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();
        let (_addr, listen_handle) = server.clone().listen(addr).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        server.stop();
        let _ = listen_handle.await;
    }
}
