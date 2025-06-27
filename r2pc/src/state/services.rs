use crate::*;
use foldhash::fast::RandomState;
use std::{collections::HashMap, sync::Arc};

pub type Method = Box<dyn Fn(Context, Msg) -> Result<()> + Send + Sync>;

pub struct Services {
    methods: HashMap<String, Method, RandomState>,
}

impl Default for Services {
    fn default() -> Self {
        let mut this = Self {
            methods: Default::default(),
        };
        let core_service = Arc::new(CoreServiceImpl);
        this.add_methods(InfoService::rpc_export(core_service.clone()));
        this
    }
}

impl Services {
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
            todo!("Method not found: {}", msg.meta.method);
        }
    }
}

impl std::fmt::Debug for Services {
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
        let manager = Services::default();
        assert!(
            manager
                .method_names()
                .any(|m| m == "InfoService/list_methods")
        );
        println!("{:?}", manager);
    }
}
