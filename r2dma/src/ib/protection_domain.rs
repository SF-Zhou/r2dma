use super::*;
use crate::{verbs::*, Error, Result};

/// A Protection Domain (PD) is a security construct that defines the boundaries within which RDMA
/// operations can be performed. It acts as a permission container that specifies which Memory
/// Regions (MRs) and other resources can be accessed by remote machines.
pub type ProtectionDomain = super::Wrapper<ibv_pd>;
pub struct ProtectionDomain {
    context: Context,
}

impl ProtectionDomain {
    pub fn create(context: &Context) -> Result<Self> {
        Ok(ProtectionDomain::new(unsafe {
            let protection_domain = ibv_alloc_pd(context.as_mut_ptr());
            if protection_domain.is_null() {
                return Err(Error::IBAllocPDFail(std::io::Error::last_os_error()));
            }
            protection_domain
        }))
    }
}

impl super::Deleter for ibv_pd {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pd_create() {
        let context = Context::create_for_test();
        let pd = ProtectionDomain::create(&context).unwrap();
        println!("pd: {:#?}", pd);
    }
}
