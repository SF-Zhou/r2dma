use crate::*;
use r2dma_sys::*;

pub type CompQueue = utils::Wrapper<ibv_cq>;

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
