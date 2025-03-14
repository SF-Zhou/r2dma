use super::*;
use crate::{verbs::*, Error, Result};
use std::{ops::Deref, sync::Arc};

/// A Memory Region (MR) is a chunk of local memory that is registered with the RDMA hardware.
/// This allows the remote machine to perform direct memory access operations (like read, write, or
/// atomic operations) to this memory without involving the local CPU.
pub struct MemoryRegion {
    _pd: Arc<ProtectionDomain>,
    ptr: *mut ibv_mr,
}

impl Drop for MemoryRegion {
    fn drop(&mut self) {
        let _ = unsafe { ibv_dereg_mr(self.ptr) };
    }
}

impl MemoryRegion {
    pub fn create(pd: &Arc<ProtectionDomain>, buf: &[u8]) -> Result<Self> {
        let ptr = unsafe {
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
        };

        Ok(Self {
            _pd: pd.clone(),
            ptr,
        })
    }
}

impl Deref for MemoryRegion {
    type Target = ibv_mr;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
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
        let devices = Device::availables().unwrap();
        let context = Arc::new(Context::create(devices.first().unwrap()).unwrap());
        let pd = Arc::new(ProtectionDomain::create(context.clone()).unwrap());
        let bytes = vec![0u8; 16];
        let mr = MemoryRegion::create(&pd, &bytes);
        println!("{:#?}", mr);
    }
}
