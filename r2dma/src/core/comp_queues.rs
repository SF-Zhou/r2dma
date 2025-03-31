use super::Devices;
use crate::{verbs, Error, Result};
use std::sync::Arc;

struct RawCompQueue(*mut verbs::ibv_cq);
impl std::ops::Deref for RawCompQueue {
    type Target = verbs::ibv_cq;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
impl Drop for RawCompQueue {
    fn drop(&mut self) {
        let _ = unsafe { verbs::ibv_destroy_cq(self.0) };
    }
}
unsafe impl Send for RawCompQueue {}
unsafe impl Sync for RawCompQueue {}

pub struct CompQueues {
    comp_queues: Vec<RawCompQueue>,
    pub cqe: usize,
    _devices: Devices,
}

impl CompQueues {
    pub fn create(devices: &Devices, max_cqe: u32) -> Result<Arc<Self>> {
        let mut comp_queues = Vec::with_capacity(devices.len());
        for device in devices {
            let ptr = unsafe {
                verbs::ibv_create_cq(
                    device.context_ptr(),
                    max_cqe as _,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                )
            };
            if ptr.is_null() {
                return Err(Error::IBCreateCompQueueFail(std::io::Error::last_os_error()));
            }
            comp_queues.push(RawCompQueue(ptr));
        }
        let cqe = comp_queues.first().unwrap().cqe as usize;

        let this = Self {
            comp_queues,
            cqe,
            _devices: devices.clone(),
        };
        Ok(Arc::new(this))
    }

    pub(crate) fn comp_queue_ptr(&self, device_index: usize) -> *mut verbs::ibv_cq {
        self.comp_queues[device_index].0
    }

    pub fn poll_cq<'a>(&self, wcs: &'a mut [verbs::ibv_wc]) -> Result<&'a mut [verbs::ibv_wc]> {
        assert!(wcs.len() >= self.comp_queues.len());
        let mut offset = 0usize;
        let num_entries = (wcs.len() / 4) as _;
        for comp_queue in &self.comp_queues {
            let num = unsafe {
                verbs::ibv_poll_cq(comp_queue.0, num_entries, wcs.as_mut_ptr().add(offset) as _)
            };
            if num >= 0 {
                offset += num as usize;
            } else {
                tracing::error!(
                    "poll comp queue failed: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
        Ok(&mut wcs[..offset])
    }
}

impl std::fmt::Debug for CompQueues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompQueue")
            .field("cqe", &self.cqe)
            .field("num_cqs", &self.comp_queues.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comp_queue() {
        let max_cqe = 1024;
        let devices = Devices::availables().unwrap();
        let comp_queues = CompQueues::create(&devices, max_cqe).unwrap();
        println!("{:#?}", comp_queues);
    }
}
