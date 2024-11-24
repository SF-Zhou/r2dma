use super::verbs::*;
use crate::*;

pub type ProtectionDomain = super::Wrapper<ibv_pd>;

impl ProtectionDomain {
    pub fn create(context: &ibv::Context) -> Result<Self> {
        Ok(ibv::ProtectionDomain::new(unsafe {
            let protection_domain = ibv_alloc_pd(context.as_mut_ptr());
            if protection_domain.is_null() {
                return Err(Error::with_errno(ErrorKind::IBAllocPDFail));
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
