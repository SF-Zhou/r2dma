#![feature(return_type_notation)]
use derse::{Deserialize, Serialize};
use r2pc::{Client, ConnectionPool, Context, Result, Server, Transport};
use std::{
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct CallReq {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallRsp {}

#[r2pc::service]
pub trait DemoService {
    async fn invoke(&self, c: &Context, r: &CallReq) -> Result<CallRsp>;
}

#[derive(Default)]
struct DemoImpl {
    value: AtomicUsize,
}

impl DemoService for DemoImpl {
    async fn invoke(&self, _ctx: &Context, _req: &CallReq) -> Result<CallRsp> {
        self.value.fetch_add(1, Ordering::SeqCst);
        Ok(CallRsp {})
    }
}

#[tokio::test]
async fn test_concurrent_call() {
    let demo = Arc::new(DemoImpl::default());
    let mut server = Server::default();
    server.add_methods(demo.clone().rpc_export());
    let server = Arc::new(server);
    let addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();

    let (addr, listen_handle) = server.clone().listen(addr).await.unwrap();
    let pool = Arc::new(ConnectionPool::new(64));
    let tr = Transport::new_sync(pool, addr);
    let ctx = Context { tr };

    const N: usize = 32;
    const M: usize = 4096;

    let mut tasks = vec![];
    for _ in 0..N {
        let ctx = ctx.clone();
        tasks.push(tokio::spawn(async move {
            for _ in 0..M {
                let req = CallReq {};
                let rsp = Client.invoke(&ctx, &req).await;
                assert!(rsp.is_ok());
            }
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }

    assert_eq!(demo.value.load(Ordering::Acquire), N * M);
    server.stop();
    let _ = listen_handle.await;
}
