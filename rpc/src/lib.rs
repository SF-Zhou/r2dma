#![feature(return_type_notation)]

mod client;
mod context;
mod error;
mod meta;
mod transport;

pub use client::Client;
pub use context::*;
pub use error::{Error, Result};
pub use meta::*;
pub use transport::*;

pub use rpc_macro::service;

#[cfg(test)]
mod tests {
    use super::*;
    use derse::{Deserialize, Serialize};

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

    #[rpc_macro::service]
    pub trait DemoService {
        async fn foo(&self, ctx: &Context, req: &FooReq) -> DemoResult<FooRsp>;
        async fn bar(&self, ctx: &Context, req: &BarReq) -> DemoResult<BarRsp>;
    }

    struct DemoImpl;
    impl DemoService for DemoImpl {
        async fn foo(&self, ctx: &Context, req: &FooReq) -> DemoResult<FooRsp> {
            println!("foo: ctx: {:?}, req: {:?}", ctx, req);
            Ok(FooRsp {
                data: req.data.clone(),
            })
        }

        async fn bar(&self, ctx: &Context, req: &BarReq) -> DemoResult<BarRsp> {
            println!("bar: ctx: {:?}, req: {:?}", ctx, req);
            Ok(BarRsp { data: req.data })
        }
    }

    #[tokio::test]
    async fn test_demo_service() {
        let demo = std::sync::Arc::new(DemoImpl);
        let map = demo.export_interface();
        assert_eq!(map.len(), 2);

        let ctx = Context {
            tr: Transport::create_for_test().await,
        };

        if let Some(func) = map.get("DemoService/foo") {
            let req = FooReq {
                data: "hello".into(),
            };
            let meta = Meta {
                msg_id: 0,
                method: "DemoService/foo".into(),
                flags: 0,
            };
            let bytes = req.serialize::<derse::DownwardBytes>().unwrap();
            func(&ctx, meta, &bytes).unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        if let Some(func) = map.get("DemoService/bar") {
            let req = BarReq { data: 233 };
            let meta = Meta {
                msg_id: 0,
                method: "DemoService/bar".into(),
                flags: 0,
            };
            let bytes = req.serialize::<derse::DownwardBytes>().unwrap();
            func(&ctx, meta, &bytes).unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}
