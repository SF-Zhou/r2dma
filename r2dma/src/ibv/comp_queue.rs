use super::verbs::*;
use crate::*;
pub type CompQueue = super::Wrapper<ibv_cq>;

impl CompQueue {
    pub fn create(
        context: &ibv::Context,
        max_cqe: u32,
        comp_channel: &ibv::CompChannel,
    ) -> Result<Self> {
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
            return Err(Error::IBCreateCQFail);
        }
        Ok(Self::new(comp_queue))
    }

    pub fn req_notify(&self) -> Result<()> {
        let ret = unsafe { ibv_req_notify_cq(self.as_mut_ptr(), 0) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Error::IBReqNotifyCQFail)
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
            Err(Error::IBPollCQFail)
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
    use super::super::*;

    #[test]
    fn test_comp_queue() {
        let context = Context::create_for_test();
        let comp_channel = CompChannel::create(&context).unwrap();
        let comp_queue = CompQueue::create(&context, 64, &comp_channel).unwrap();
        println!("{:#?}", comp_queue);

        comp_queue.req_notify().unwrap();
        comp_queue.ack_cq_events(0);
    }
}
