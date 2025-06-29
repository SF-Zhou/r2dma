#![feature(return_type_notation)]
use r2pc::*;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct FooReq(u64);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FooRsp(u64);

#[service]
pub trait FooService {
    async fn foo(&self, c: &Context, r: &FooReq) -> Result<FooRsp>;
}

struct FooServiceImpl;

impl FooService for FooServiceImpl {
    async fn foo(&self, c: &Context, r: &FooReq) -> Result<FooRsp> {
        let client = Client {
            timeout: Duration::from_secs(1),
        };
        let rsp = client.bar(c, &BarReq(r.0)).await?;
        Ok(FooRsp(rsp.0 + 1))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BarReq(u64);

#[derive(Debug, Serialize, Deserialize)]
pub struct BarRsp(u64);

#[service]
pub trait BarService {
    async fn bar(&self, c: &Context, r: &BarReq) -> Result<BarRsp>;
}

struct BarServiceImpl;

impl BarService for BarServiceImpl {
    async fn bar(&self, _c: &Context, r: &BarReq) -> Result<BarRsp> {
        Ok(BarRsp(r.0 * 2))
    }
}

#[tokio::test]
async fn test_concurrent_call() {
    let foo = Arc::new(FooServiceImpl);
    let mut server_service_manager = ServiceManager::default();
    server_service_manager.add_methods(foo.clone().rpc_export());
    let server = Server::create(server_service_manager);
    let server = Arc::new(server);
    let addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();
    let (addr, listen_handle) = server.clone().listen(addr).await.unwrap();

    let bar = Arc::new(BarServiceImpl);
    let mut client_service_manager = ServiceManager::default();
    client_service_manager.add_methods(bar.clone().rpc_export());
    let state = Arc::new(State::new(client_service_manager));
    let ctx = state.client_ctx(addr);

    let client = Client::default();
    assert_eq!(client.foo(&ctx, &FooReq(0)).await, Ok(FooRsp(1)));
    assert_eq!(client.foo(&ctx, &FooReq(1)).await, Ok(FooRsp(3)));

    let rsp = client
        .list_methods(&ctx, &Default::default())
        .await
        .unwrap();
    assert_eq!(rsp.len(), 2);

    client.bar(&ctx, &BarReq(0)).await.unwrap_err();

    server.stop();
    let _ = listen_handle.await;
}
