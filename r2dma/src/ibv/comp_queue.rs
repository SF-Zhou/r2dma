use super::*;
use crate::{Error, Result};
use std::{ops::Deref, sync::Arc};

/// A Completion Queue is an object which contains the completed work requests which were posted to
/// the Work Queues (WQ). Every completion says that a specific WR was completed (both successfully
/// completed WRs and unsuccessfully completed WRs).A Completion Queue is a mechanism to notify the
/// application about information of ended Work Requests (status, opcode, size, source). CQs have n
/// Completion Queue Entries (CQE). The number of CQEs is specified when the CQ is created. When a
/// CQE is polled it is removed from the CQ. CQ is a FIFO of CQEs. CQ can service send queues,
/// receive queues, or both. Work queues from multiple QPs can be associated with a single CQ.
/// struct ibv_cq is used to implement a CQ.
pub struct CompQueue {
    _context: Arc<Context>,
    comp_channel: Option<Arc<CompChannel>>,
    ptr: *mut ibv_cq,
}

impl Drop for CompQueue {
    fn drop(&mut self) {
        let _ = unsafe { ibv_destroy_cq(self.ptr) };
    }
}

unsafe impl Send for CompQueue {}
unsafe impl Sync for CompQueue {}

impl CompQueue {
    pub fn create(
        context: &Arc<Context>,
        max_cqe: u32,
        comp_channel: Option<&Arc<CompChannel>>,
    ) -> Result<Arc<Self>> {
        let ptr = unsafe {
            ibv_create_cq(
                context.as_mut_ptr(),
                max_cqe as _,
                std::ptr::null_mut(),
                comp_channel
                    .map(|c| c.as_mut_ptr())
                    .unwrap_or(std::ptr::null_mut()),
                0,
            )
        };
        if ptr.is_null() {
            return Err(Error::IBCreateCompQueueFail(std::io::Error::last_os_error()));
        }
        Ok(Arc::new(Self {
            _context: context.clone(),
            comp_channel: comp_channel.cloned(),
            ptr,
        }))
    }

    pub fn req_notify(&self) -> Result<()> {
        if self.comp_channel.is_none() {
            return Err(Error::IBReqNotifyCompQueueFail(
                std::io::Error::from_raw_os_error(libc::EINVAL),
            ));
        }
        let ret = unsafe { ibv_req_notify_cq(self.ptr, 0) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Error::IBReqNotifyCompQueueFail(
                std::io::Error::last_os_error(),
            ))
        }
    }

    pub fn ack_cq_events(&self, unack_events_count: u32) {
        unsafe { ibv_ack_cq_events(self.ptr, unack_events_count) }
    }

    pub fn set_cq_context(&self, ptr: *mut std::ffi::c_void) {
        let this = unsafe { &mut *self.ptr };
        this.cq_context = ptr;
    }

    pub fn poll_cq<'a>(&self, wc: &'a mut [ibv_wc]) -> Result<&'a mut [ibv_wc]> {
        let num_entries = wc.len() as i32;
        let num = unsafe { ibv_poll_cq(self.ptr, num_entries, wc.as_mut_ptr() as _) };
        if num >= 0 {
            Ok(&mut wc[..num as usize])
        } else {
            Err(Error::IBPollCompQueueFail(std::io::Error::last_os_error()))
        }
    }

    #[allow(unused)]
    pub(crate) fn as_mut_ptr(&self) -> *mut ibv_cq {
        self.ptr
    }
}

impl Deref for CompQueue {
    type Target = ibv_cq;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl std::fmt::Debug for CompQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompQueue")
            .field("handle", &self.handle)
            .field("cqe", &self.cqe)
            .field("comp_events_completiond", &self.comp_events_completed)
            .field("async_events_completiond", &self.async_events_completed)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comp_queue() {
        let devices = Device::availables().unwrap();
        let context = Context::create(devices.first().unwrap()).unwrap();
        let comp_queue = CompQueue::create(&context, 64, None).unwrap();
        println!("{:#?}", comp_queue);

        comp_queue.req_notify().unwrap_err();
        comp_queue.ack_cq_events(0);
        comp_queue.set_cq_context(std::ptr::null_mut());

        let mut wcs: Vec<ibv_wc> = vec![];
        wcs.resize(8, ibv_wc::default());
        let finished = comp_queue.poll_cq(&mut wcs).unwrap();
        assert!(finished.is_empty());

        let comp_channel = CompChannel::create(&context).unwrap();
        let comp_queue = CompQueue::create(&context, 64, Some(&comp_channel)).unwrap();
        comp_queue.req_notify().unwrap();
    }
}
