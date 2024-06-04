use crate::*;
use r2dma_sys::*;

use std::os::{fd::BorrowedFd, raw::c_void};

pub type CompChannel = utils::Wrapper<ibv_comp_channel>;

impl CompChannel {
    pub fn fd(&self) -> BorrowedFd {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }

    pub fn set_nonblock(&self) -> Result<()> {
        let flags = unsafe { libc::fcntl(self.fd, libc::F_GETFL) };
        if flags == -1 {
            return Err(Error::with_errno(ErrorKind::SetNonBlockFail));
        }

        let ret = unsafe { libc::fcntl(self.fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Error::with_errno(ErrorKind::SetNonBlockFail))
        }
    }

    pub fn get_cq_event(&self) -> Result<*mut c_void> {
        let mut comp_queue: *mut ibv_cq = std::ptr::null_mut();
        let mut cq_context: *mut std::ffi::c_void = std::ptr::null_mut();
        let ret = unsafe { ibv_get_cq_event(self.as_mut_ptr(), &mut comp_queue, &mut cq_context) };
        if ret == 0 {
            Ok(cq_context)
        } else if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
            Ok(std::ptr::null_mut())
        } else {
            Err(Error::with_errno(ErrorKind::IBGetCQEventFail))
        }
    }
}

impl utils::Deleter for ibv_comp_channel {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        ibv_destroy_comp_channel(ptr)
    }
}

impl std::fmt::Debug for CompChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompChannel")
            .field("fd", &self.fd)
            .field("refcnt", &self.refcnt)
            .finish()
    }
}
