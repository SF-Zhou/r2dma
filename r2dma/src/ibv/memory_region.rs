use crate::*;
use r2dma_sys::*;

pub type MemoryRegion = utils::Wrapper<ibv_mr>;

impl utils::Deleter for ibv_mr {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        ibv_dereg_mr(ptr)
    }
}

impl std::fmt::Debug for MemoryRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryRegion")
            .field("addr", &self.addr)
            .field("length", &self.length)
            .field("handle", &self.handle)
            .field("lkey", &self.lkey)
            .field("rkey", &self.rkey)
            .finish()
    }
}
