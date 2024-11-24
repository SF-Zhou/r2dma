use super::{verbs::*, *};
use crate::*;

pub type MemoryRegion = super::Wrapper<ibv_mr>;

impl MemoryRegion {
    pub fn create(pd: &ProtectionDomain, buf: &utils::AlignedBuffer) -> Result<Self> {
        Ok(Self::new(unsafe {
            let memory_region = ibv_reg_mr(
                pd.as_mut_ptr(),
                buf.as_ptr() as _,
                buf.len(),
                ibv::ACCESS_FLAGS as _,
            );
            if memory_region.is_null() {
                return Err(Error::with_errno(ErrorKind::IBRegMRFail));
            }
            memory_region
        }))
    }
}

impl super::Deleter for ibv_mr {
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
