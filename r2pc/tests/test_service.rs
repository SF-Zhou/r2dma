#![feature(return_type_notation)]
use r2pc::*;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize)]
struct DemoError(pub String);

impl std::error::Error for DemoError {}

impl std::fmt::Display for DemoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "demo error: {}", self.0)
    }
}

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
    async fn timeout(&self, ctx: &Context, req: &FooReq) -> DemoResult<FooRsp>;
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

    async fn timeout(&self, _ctx: &Context, req: &FooReq) -> DemoResult<FooRsp> {
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Ok(FooRsp {
            data: req.data.clone(),
        })
    }
}

#[tokio::test]
async fn test_demo_service() {
    let demo = Arc::new(DemoImpl);
    let mut service_manager = ServiceManager::default();
    service_manager.add_methods(demo.clone().rpc_export());
    let server = Server::create(service_manager);
    let server = Arc::new(server);
    let addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();
    let (addr, listen_handle) = server.clone().listen(addr).await.unwrap();

    let state = Arc::new(State::default());
    let ctx = state.client_ctx(addr);

    let client = Client::default();
    let req = FooReq { data: "foo".into() };
    let rsp = client.foo(&ctx, &req).await;
    match rsp {
        Ok(r) => assert_eq!(r.data, "foo"),
        Err(e) => assert_eq!(e.to_string(), ""),
    }

    let req = BarReq { data: 233 };
    let rsp = client.bar(&ctx, &req).await;
    match rsp {
        Ok(r) => assert_eq!(r.data, 234),
        Err(e) => assert_eq!(e.to_string(), ""),
    }

    let req = FooReq { data: "foo".into() };
    let rsp = client.timeout(&ctx, &req).await;
    tracing::info!("{rsp:?}");
    assert!(rsp.is_err());

    server.stop();
    let _ = listen_handle.await;
}

#[test]
fn test_demo_service_sync() {
    let demo = Arc::new(DemoImpl);
    let mut service_manager = ServiceManager::default();
    service_manager.add_methods(demo.clone().rpc_export());
    let server = Server::create(service_manager);
    let server = Arc::new(server);
    let addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let (addr, listen_handle) = runtime.block_on(server.clone().listen(addr)).unwrap();

    let state = Arc::new(State::default());
    let ctx = state.client_ctx(addr);

    let req = FooReq { data: "foo".into() };
    let current = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let client = Client::default();
    let rsp = current.block_on(client.foo(&ctx, &req));
    match rsp {
        Ok(r) => assert_eq!(r.data, "foo"),
        Err(e) => assert_eq!(e.to_string(), ""),
    }

    server.stop();
    let _ = runtime.block_on(listen_handle);
}
