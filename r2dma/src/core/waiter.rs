use tokio::sync::oneshot;

pub struct WaitState {
    pub sender: oneshot::Sender<()>,
}

#[derive(Debug, Default)]
pub struct Waiter {
    lockmap: lockmap::LockMap<usize, WaitState>,
}

impl Waiter {
    pub fn register(&self, id: usize, sender: oneshot::Sender<()>) {
        self.lockmap.insert(id, WaitState { sender });
    }

    pub fn notify(&self, id: usize) {
        if let Some(wait_state) = self.lockmap.remove(&id) {
            let _ = wait_state.sender.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn test_waiter() {
        let waiter = Arc::new(Waiter::default());
        let (sender, receiver) = oneshot::channel();
        let id = 1;

        waiter.register(id, sender);

        tokio::spawn({
            let waiter = Arc::clone(&waiter);
            async move {
                // Simulate some work before notifying
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                waiter.notify(id);
            }
        });

        assert!(receiver.await.is_ok());
    }
}
