use super::{CompQueues, Devices, Socket};
use crate::{verbs, Result};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub struct EventLoopState {
    stopping: AtomicBool,
    comp_queues: Arc<CompQueues>,
    // Map wr_id to Socket to handle completion events
    // Arc<Socket> allows shared ownership if Sockets are managed elsewhere too.
    // Mutex for interior mutability across threads.
    socket_map: Mutex<HashMap<u64, Arc<Socket>>>,
}

pub struct EventLoop {
    state: Arc<EventLoopState>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EventLoop {
    pub fn create(devices: &Devices) -> Result<Self> {
        let max_cqe = 32;
        let comp_queues = CompQueues::create(devices, max_cqe)?;
        let state = Arc::new(EventLoopState {
            stopping: AtomicBool::new(false),
            comp_queues,
            socket_map: Mutex::new(HashMap::new()),
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

    // Method to register a socket with a specific wr_id
    // This would typically be called by the Socket when it posts a work request.
    pub fn register_socket(&self, wr_id: u64, socket: Arc<Socket>) {
        let mut map = self.state.socket_map.lock().unwrap();
        map.insert(wr_id, socket);
    }

    // Method to deregister a socket (e.g., when the operation is done or socket is closed)
    // This is important to prevent the map from growing indefinitely.
    pub fn deregister_socket(&self, wr_id: u64) {
        let mut map = self.state.socket_map.lock().unwrap();
        map.remove(&wr_id);
    }

    pub fn run(state: Arc<EventLoopState>) {
        let comp_queues = state.comp_queues.clone();
        let num_entiries = comp_queues.num_entries();
        let mut wcs_vec = vec![verbs::ibv_wc::default(); num_entiries];

        while !state.stopping.load(Ordering::Acquire) {
            // poll for events.
            let polled_wcs = comp_queues.poll_cq(&mut wcs_vec).unwrap();
            if polled_wcs.is_empty() {
                // TODO: Consider a more sophisticated sleep/wakeup mechanism (e.g., condvar)
                // if low-latency is critical and CPU usage for polling is a concern.
                // For now, a short sleep is fine for many applications.
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }

            // handle events.
            for wc in polled_wcs {
                tracing::info!(
                    "CQ Event: wr_id={}, status={:?}, opcode={:?}, byte_len={}",
                    wc.wr_id,
                    wc.status,
                    wc.opcode,
                    wc.byte_len
                );

                // Look up the socket associated with this work completion
                let socket_map = state.socket_map.lock().unwrap();
                if let Some(socket) = socket_map.get(&wc.wr_id) {
                    // Notify the socket
                    socket.handle_completion(wc);

                    // TODO: Decide on a strategy for deregistering sockets.
                    // If a wr_id is only used once, it should be deregistered here.
                    // If a wr_id can be reused (e.g., for persistent RECV requests),
                    // then deregistration happens elsewhere (e.g., when socket is closed).
                    // For simplicity, let's assume for now that wr_ids might be reused
                    // or handled by the socket itself.
                    // Example: if wc.status != rdma_sys::ibv_wc_status::IBV_WC_SUCCESS {
                    //     // On error, perhaps always deregister or let socket decide.
                    // }
                } else {
                    tracing::warn!("No socket found for wr_id: {}", wc.wr_id);
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
        let event_loop = EventLoop::create(&devices).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        drop(event_loop);
    }
}
