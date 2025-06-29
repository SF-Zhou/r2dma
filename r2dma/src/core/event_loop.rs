use super::{CompQueues, Devices};
use crate::{verbs, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct EventLoopState {
    stopping: AtomicBool,
    comp_queues: Arc<CompQueues>,
}

pub struct EventLoop {
    state: Arc<EventLoopState>,
    handle: Option<std::thread::JoinHandle<()>>,
}

/// An event loop that continuously polls for completion events from the RDMA devices.
/// It runs in a separate thread and processes completion events until stopped.
impl EventLoop {
    pub fn create(devices: &Devices, max_cqe: u32) -> Result<Self> {
        let comp_queues = CompQueues::create(devices, max_cqe)?;
        let state = Arc::new(EventLoopState {
            stopping: AtomicBool::new(false),
            comp_queues,
        });

        let handle = std::thread::spawn({
            let state = state.clone();
            move || EventLoop::run(state)
        });

        Ok(EventLoop {
            state,
            handle: Some(handle),
        })
    }

    pub fn stop_and_join(&mut self) {
        self.state.stopping.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }

    pub fn run(state: Arc<EventLoopState>) {
        let comp_queues = state.comp_queues.clone();
        let num_entiries = comp_queues.num_entries();
        let mut wcs = vec![verbs::ibv_wc::default(); num_entiries];

        while !state.stopping.load(Ordering::Acquire) {
            // poll for events.
            let wcs = comp_queues.poll_cq(&mut wcs).unwrap();
            if wcs.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }

            // handle events.
            for wc in wcs {
                if wc.is_recv() {
                    tracing::info!(
                        "wc is recv id {}, result {}, status {:?}",
                        wc.wr_id,
                        wc.byte_len,
                        wc.status
                    );
                } else {
                    tracing::info!(
                        "wc is send id {}, result {}, status {:?}",
                        wc.wr_id,
                        wc.byte_len,
                        wc.status
                    );
                }
            }
        }
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_loop() {
        let devices = Devices::availables().unwrap();
        let event_loop = EventLoop::create(&devices, 32).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        drop(event_loop);
    }
}
