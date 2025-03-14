use super::*;
use crate::{verbs::*, Error, Result};
use std::{ops::Deref, sync::Arc};

/// A Protection Domain (PD) is a security construct that defines the boundaries within which RDMA
/// operations can be performed. It acts as a permission container that specifies which Memory
/// Regions (MRs) and other resources can be accessed by remote machines.
pub struct ProtectionDomain {
    _context: Arc<Context>,
    ptr: *mut ibv_pd,
}

impl Drop for ProtectionDomain {
    fn drop(&mut self) {
        let _ = unsafe { ibv_dealloc_pd(self.ptr) };
    }
}

impl ProtectionDomain {
    pub fn create(context: Arc<Context>) -> Result<Self> {
        let ptr = unsafe {
            let protection_domain = ibv_alloc_pd(context.as_mut_ptr());
            if protection_domain.is_null() {
                return Err(Error::IBAllocPDFail(std::io::Error::last_os_error()));
            }
            protection_domain
        };
        Ok(Self {
            _context: context,
            ptr,
        })
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut ibv_pd {
        self.ptr
    }
}

impl Deref for ProtectionDomain {
    type Target = ibv_pd;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl std::fmt::Debug for ProtectionDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtectionDomain")
            .field("handle", &self.handle)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pd_create() {
        let devices = Device::availables().unwrap();
        assert!(!devices.is_empty());
        let context = Context::create(devices.first().unwrap()).unwrap();
        let context = Arc::new(context);
        let pd = ProtectionDomain::create(context).unwrap();
        println!("pd: {:#?}", pd);
    }
}
