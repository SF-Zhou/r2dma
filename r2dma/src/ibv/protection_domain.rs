use crate::*;
use r2dma_sys::*;

pub type ProtectionDomain = utils::Wrapper<ibv_pd>;

impl ProtectionDomain {}

impl utils::Deleter for ibv_pd {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        ibv_dealloc_pd(ptr)
    }
}

impl std::fmt::Debug for ProtectionDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtectionDomain")
            .field("handle", &self.handle)
            .finish()
    }
}
