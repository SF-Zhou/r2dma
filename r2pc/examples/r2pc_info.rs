use clap::Parser;
use r2pc::*;
use std::sync::Arc;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Server address.
    #[arg(default_value = "127.0.0.1:8000")]
    pub addr: std::net::SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    let state = Arc::new(State::default());
    let ctx = state.client_ctx(args.addr);

    let client = Client::default();
    let rsp = client.list_methods(&ctx, &()).await?;
    if !rsp.is_empty() {
        tracing::info!(
            "The address {} provides the following RPC methods: {:#?}",
            args.addr,
            rsp
        );
    }
    Ok(())
}
