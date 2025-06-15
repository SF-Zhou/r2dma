use clap::Parser;
use r2pc::{Context, Result, Server, Services};
use r2pc_demo::{EchoService, GreetService, Request};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Listen address.
    #[arg(default_value = "0.0.0.0:8000")]
    pub addr: std::net::SocketAddr,
}

#[derive(Default)]
struct DemoImpl {
    idx: AtomicU64,
}

impl EchoService for DemoImpl {
    async fn echo(&self, _c: &Context, r: &Request) -> Result<String> {
        self.idx.fetch_add(1, Ordering::AcqRel);
        Ok(r.0.clone())
    }
}

impl GreetService for DemoImpl {
    async fn greet(&self, _c: &Context, r: &Request) -> Result<String> {
        let val = self.idx.fetch_add(1, Ordering::AcqRel);
        Ok(format!("hello {}({})!", r.0, val))
    }
}

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    let demo = Arc::new(DemoImpl::default());
    let mut services = Services::default();
    services.add_methods(EchoService::rpc_export(demo.clone()));
    services.add_methods(GreetService::rpc_export(demo.clone()));
    let server = Server::create(services);

    let server = Arc::new(server);
    let (addr, listen_handle) = server.listen(args.addr).await.unwrap();
    tracing::info!(
        "Serving {:?} on {}...",
        [
            <DemoImpl as EchoService>::NAME,
            <DemoImpl as GreetService>::NAME
        ],
        addr.to_string()
    );
    listen_handle.await.unwrap();
}
