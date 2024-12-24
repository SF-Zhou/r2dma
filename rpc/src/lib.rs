#![feature(return_type_notation)]

mod call_context;
mod error;
mod meta;
mod transport;

pub use call_context::*;
pub use error::{Error, Result};
pub use meta::*;
pub use transport::*;

#[cfg(test)]
mod tests {
    use super::*;
    use derse::{Deserialize, Serialize};

    #[derive(Debug, derse::Serialize, derse::Deserialize)]
    pub struct Req {
        pub data: String,
    }

    #[derive(Debug, derse::Serialize, derse::Deserialize)]
    pub struct Rsp {
        pub data: String,
    }

    pub trait DemoService {
        async fn foo(&self, req: &Req) -> Result<Rsp>;

        fn export_interface(
            self: std::sync::Arc<Self>,
        ) -> std::collections::HashMap<String, Box<dyn Fn(&[u8]) -> Result<()>>>
        where
            Self: 'static + Send + Sync,
            Self::foo(..): Send,
        {
            let mut map =
                std::collections::HashMap::<String, Box<dyn Fn(&[u8]) -> Result<()>>>::default();
            let this = self.clone();
            map.insert(
                "DemoService/foo".into(),
                Box::new(move |bytes| {
                    let req = Req::deserialize(bytes).unwrap();
                    let this = this.clone();
                    tokio::spawn(async move {
                        let _ = this.foo(&req).await;
                    });
                    Ok(())
                }),
            );
            map
        }
    }

    struct DemoImpl;
    impl DemoService for DemoImpl {
        async fn foo(&self, req: &Req) -> Result<Rsp> {
            println!("req is {:#?}", req);
            Ok(Rsp {
                data: req.data.clone(),
            })
        }
    }

    #[tokio::test]
    async fn test_demo_service() {
        let demo = std::sync::Arc::new(DemoImpl);
        let map = demo.export_interface();

        if let Some(func) = map.get("DemoService/foo") {
            let req = Req {
                data: "hello".into(),
            };
            let bytes = req.serialize::<derse::DownwardBytes>().unwrap();
            func(&bytes).unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}
