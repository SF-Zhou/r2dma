use crate::{ibv::*, Error, Result};

/// A Memory Region (MR) is a chunk of local memory that is registered with the RDMA hardware.
/// This allows the remote machine to perform direct memory access operations (like read, write, or
/// atomic operations) to this memory without involving the local CPU.
pub type MemoryRegion = super::Wrapper<ibv_mr>;

impl MemoryRegion {
    pub fn create(pd: &ProtectionDomain, buf: &[u8]) -> Result<Self> {
        Ok(Self::new(unsafe {
            let memory_region = ibv_reg_mr(
                pd.as_mut_ptr(),
                buf.as_ptr() as _,
                buf.len(),
                ACCESS_FLAGS as _,
            );
            if memory_region.is_null() {
                return Err(Error::IBRegMemoryRegionFail(std::io::Error::last_os_error()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_create() {
        let context = Context::create_for_test();
        let pd = ProtectionDomain::create(&context).unwrap();
        let bytes = vec![0u8; 16];
        let mr = MemoryRegion::create(&pd, &bytes);
        println!("{:#?}", mr);
    }
}
