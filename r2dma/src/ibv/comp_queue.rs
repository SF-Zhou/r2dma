use super::WorkCompletion;
use crate::*;
use r2dma_sys::*;

pub type CompQueue = utils::Wrapper<ibv_cq>;

impl CompQueue {
    pub fn poll<'a>(&self, wc: &'a mut [WorkCompletion]) -> Result<&'a [WorkCompletion]> {
        let num_entries = wc.len() as i32;
        let num = unsafe { ibv_poll_cq(self.as_mut_ptr(), num_entries, wc.as_mut_ptr() as _) };
        if num >= 0 {
            Ok(&wc[..num as usize])
        } else {
            Err(Error::with_errno(ErrorKind::IBPollCQFail))
        }
    }
}

impl utils::Deleter for ibv_cq {
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
