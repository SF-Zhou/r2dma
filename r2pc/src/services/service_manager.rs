use crate::*;
use foldhash::fast::RandomState;
use std::{collections::HashMap, sync::Arc};

pub type Method = Box<dyn Fn(Context, Msg) -> Result<()> + Send + Sync>;

pub struct ServiceManager {
    methods: HashMap<String, Method, RandomState>,
}

impl Default for ServiceManager {
    fn default() -> Self {
        let mut this = Self {
            methods: Default::default(),
        };
        let dummy = Arc::new(());
        this.add_methods(InfoService::rpc_export(dummy.clone()));
        this
    }
}

impl ServiceManager {
    pub fn add_methods(&mut self, methods: HashMap<String, Method>) {
        self.methods.extend(methods);
    }

    pub fn method_names(&self) -> impl Iterator<Item = &String> {
        self.methods.keys()
    }

    pub fn invoke(&self, ctx: Context, msg: Msg) {
        if let Some(func) = self.methods.get(&msg.meta.method) {
            let _ = func(ctx, msg);
        } else {
            tokio::spawn(async move {
                let m = format!("method not found: {}", msg.meta.method);
                tracing::error!(m);
                ctx.send_rsp::<(), Error>(msg.meta, Err(Error::new(ErrorKind::InvalidArgument, m)))
                    .await;
            });
        }
    }
}

impl std::fmt::Debug for ServiceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceManager")
            .field("methods", &self.methods.keys())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_services() {
        let manager = ServiceManager::default();
        assert!(
            manager
                .method_names()
                .any(|m| m == "InfoService/list_methods")
        );
        println!("{:?}", manager);
    }
}
