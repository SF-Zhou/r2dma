use clap::Parser;
use r2pc::{Client, ConnectionPool, Context, Transport};
use r2pc_demo::{EchoService, GreetService};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Listen address.
    #[arg(default_value = "127.0.0.1:8000")]
    pub addr: std::net::SocketAddr,

    /// Request value.
    #[arg(short, long, default_value = "alice")]
    pub value: String,

    /// Enable stress testing.
    #[arg(long, default_value_t = false)]
    pub stress: bool,

    /// Stress testing duration.
    #[arg(long, default_value = "60")]
    pub secs: u64,

    /// The number of coroutines.
    #[arg(long, default_value = "32")]
    pub coroutines: usize,
}

async fn stress_test(args: Args) {
    let counter = Arc::new(AtomicU64::new(0));
    let start_time = std::time::Instant::now();
    let pool = Arc::new(ConnectionPool::new(64));
    let tr = Transport::new_sync(pool, args.addr);
    let ctx = Context { tr };
    let mut tasks = vec![];
    for _ in 0..args.coroutines {
        let value = args.value.clone();
        let counter = counter.clone();
        let ctx = ctx.clone();
        tasks.push(tokio::spawn(async move {
            while std::time::Instant::now()
                .duration_since(start_time)
                .as_secs()
                < args.secs
            {
                for _ in 0..4096 {
                    let rsp = Client.echo(&ctx, &value).await;
                    assert!(rsp.is_ok());
                    counter.fetch_add(1, Ordering::AcqRel);
                }
            }
        }));
    }
    tokio::select! {
        _ = async {
            for task in tasks {
                task.await.unwrap();
            }
        } => {
        }
        _ = async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                tracing::info!("QPS: {}/s", counter.swap(0, Ordering::SeqCst));
            }
        } => {
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    if args.stress {
        stress_test(args).await;
    } else {
        let pool = Arc::new(ConnectionPool::new(4));
        let tr = Transport::new_sync(pool, args.addr);
        let ctx = Context { tr };
        let rsp = Client.echo(&ctx, &args.value).await;
        tracing::info!("echo rsp: {:?}", rsp);

        let rsp = Client.greet(&ctx, &args.value).await;
        tracing::info!("greet rsp: {:?}", rsp);
    }
}
