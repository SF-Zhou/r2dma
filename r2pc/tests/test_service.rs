#![feature(return_type_notation)]
use derse::{Deserialize, Serialize};
use r2pc::{Client, ConnectionPool, Context, Error, Server, Transport};
use std::{str::FromStr, sync::Arc};

#[derive(Debug, Serialize, Deserialize)]
pub struct FooReq {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FooRsp {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BarReq {
    pub data: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BarRsp {
    pub data: u64,
}

#[derive(thiserror::Error, derse::Serialize, derse::Deserialize)]
#[error("bar error: {0}")]
struct DemoError(pub String);

impl std::fmt::Debug for DemoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl From<Error> for DemoError {
    fn from(e: Error) -> Self {
        Self(e.to_string())
    }
}

type DemoResult<T> = std::result::Result<T, DemoError>;

#[r2pc::service]
pub trait DemoService {
    async fn foo(&self, ctx: &Context, req: &FooReq) -> DemoResult<FooRsp>;
    async fn bar(&self, ctx: &Context, req: &BarReq) -> DemoResult<BarRsp>;
}

struct DemoImpl;
impl DemoService for DemoImpl {
    async fn foo(&self, ctx: &Context, req: &FooReq) -> DemoResult<FooRsp> {
        tracing::info!("foo: ctx: {:?}, req: {:?}", ctx, req);
        Ok(FooRsp {
            data: req.data.clone(),
        })
    }

    async fn bar(&self, ctx: &Context, req: &BarReq) -> DemoResult<BarRsp> {
        tracing::info!("bar: ctx: {:?}, req: {:?}", ctx, req);
        Ok(BarRsp { data: req.data + 1 })
    }
}

#[tokio::test]
async fn test_demo_service() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let demo = Arc::new(DemoImpl);
    let mut server = Server::default();
    server.add_methods(demo.rpc_export());
    let server = Arc::new(server);
    let addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();

    let (addr, listen_handle) = server.clone().listen(addr).await.unwrap();
    let pool = Arc::new(ConnectionPool::new(16));
    let tr = Transport::new_sync(pool, addr);
    let ctx = Context { tr };

    let req = FooReq { data: "foo".into() };
    let rsp = Client.foo(&ctx, &req).await;
    match rsp {
        Ok(r) => assert_eq!(r.data, "foo"),
        Err(e) => assert_eq!(e.to_string(), ""),
    }

    let req = BarReq { data: 233 };
    let rsp = Client.bar(&ctx, &req).await;
    match rsp {
        Ok(r) => assert_eq!(r.data, 234),
        Err(e) => assert_eq!(e.to_string(), ""),
    }

    server.stop();
    let _ = listen_handle.await;
}

#[test]
fn test_demo_service_sync() {
    let demo = Arc::new(DemoImpl);
    let mut server = Server::default();
    server.add_methods(demo.rpc_export());
    let server = Arc::new(server);
    let addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (addr, listen_handle) = runtime.block_on(server.clone().listen(addr)).unwrap();

    let pool = Arc::new(ConnectionPool::new(16));
    let tr = Transport::new_sync(pool, addr);
    let ctx = Context { tr };

    let req = FooReq { data: "foo".into() };
    let current = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let rsp = current.block_on(Client.foo(&ctx, &req));
    match rsp {
        Ok(r) => assert_eq!(r.data, "foo"),
        Err(e) => assert_eq!(e.to_string(), ""),
    }

    server.stop();
    let _ = runtime.block_on(listen_handle);
}
