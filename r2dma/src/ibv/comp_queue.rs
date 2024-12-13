use crate::{ibv::*, Error, Result};

/// A Completion Queue is an object which contains the completed work requests which were posted to
/// the Work Queues (WQ). Every completion says that a specific WR was completed (both successfully
/// completed WRs and unsuccessfully completed WRs).A Completion Queue is a mechanism to notify the
/// application about information of ended Work Requests (status, opcode, size, source). CQs have n
/// Completion Queue Entries (CQE). The number of CQEs is specified when the CQ is created. When a
/// CQE is polled it is removed from the CQ. CQ is a FIFO of CQEs. CQ can service send queues,
/// receive queues, or both. Work queues from multiple QPs can be associated with a single CQ.
/// struct ibv_cq is used to implement a CQ.
pub type CompQueue = super::Wrapper<ibv_cq>;

impl CompQueue {
    pub fn create(context: &Context, max_cqe: u32, comp_channel: &CompChannel) -> Result<Self> {
        let comp_queue = unsafe {
            ibv_create_cq(
                context.as_mut_ptr(),
                max_cqe as _,
                std::ptr::null_mut(),
                comp_channel.as_mut_ptr(),
                0,
            )
        };
        if comp_queue.is_null() {
            return Err(Error::IBCreateCompQueueFail(std::io::Error::last_os_error()));
        }
        Ok(Self::new(comp_queue))
    }

    pub fn req_notify(&self) -> Result<()> {
        let ret = unsafe { ibv_req_notify_cq(self.as_mut_ptr(), 0) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Error::IBReqNotifyCompQueueFail(
                std::io::Error::last_os_error(),
            ))
        }
    }

    pub fn ack_cq_events(&self, unack_events_count: u32) {
        unsafe { ibv_ack_cq_events(self.as_mut_ptr(), unack_events_count) }
    }

    pub fn set_cq_context(&self, ptr: *mut std::ffi::c_void) {
        let this = unsafe { &mut *self.as_mut_ptr() };
        this.cq_context = ptr;
    }

    pub fn poll_cq<'a>(&self, wc: &'a mut [ibv_wc]) -> Result<&'a mut [ibv_wc]> {
        let num_entries = wc.len() as i32;
        let num = unsafe { ibv_poll_cq(self.as_mut_ptr(), num_entries, wc.as_mut_ptr() as _) };
        if num >= 0 {
            Ok(&mut wc[..num as usize])
        } else {
            Err(Error::IBPollCompQueueFail(std::io::Error::last_os_error()))
        }
    }
}

impl super::Deleter for ibv_cq {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        ibv_destroy_cq(ptr)
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
        let context = Context::create_for_test();
        let comp_channel = CompChannel::create(&context).unwrap();
        let comp_queue = CompQueue::create(&context, 64, &comp_channel).unwrap();
        println!("{:#?}", comp_queue);

        comp_queue.req_notify().unwrap();
        comp_queue.ack_cq_events(0);
        comp_queue.set_cq_context(std::ptr::null_mut());

        let mut wcs: Vec<ibv_wc> = vec![];
        wcs.resize(8, ibv_wc::default());
        let finished = comp_queue.poll_cq(&mut wcs).unwrap();
        assert!(finished.is_empty());

        drop(context);
        drop(comp_channel);
        comp_queue.req_notify().unwrap_err();
    }
}
