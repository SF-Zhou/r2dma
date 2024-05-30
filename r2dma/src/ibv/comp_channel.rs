use crate::*;
use r2dma_sys::*;

use std::{
    os::fd::{AsRawFd, RawFd},
    sync::Arc,
};

pub type CompChannel = Wrapper<ibv_comp_channel>;

impl CompChannel {
    pub fn fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn set_nonblock(&self) -> Result<()> {
        let fd = self.fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags == -1 {
            return Err(Error::with_errno(ErrorKind::SetNonBlockFail));
        }

        let ret = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Error::with_errno(ErrorKind::SetNonBlockFail))
        }
    }

    pub fn wait(&self) -> Result<u32> {
        let mut pollfd = libc::pollfd {
            fd: self.fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pollfd, 1, 100) };
        if ret >= 0 {
            Ok(ret as _)
        } else {
            Err(Error::with_errno(ErrorKind::PollCompChannelFailed))
        }
    }

    pub fn poll(&self) -> Result<Option<Arc<Socket>>> {
        let mut comp_queue: *mut ibv_cq = std::ptr::null_mut();
        let mut cq_context: *mut std::ffi::c_void = std::ptr::null_mut();
        let ret = unsafe { ibv_get_cq_event(self.as_mut_ptr(), &mut comp_queue, &mut cq_context) };
        if ret == 0 {
            Ok(Some(Socket::from_cq_context(cq_context)))
        } else if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(Error::with_errno(ErrorKind::IBGetCQEventFail))
        }
    }
}

impl Deleter for ibv_comp_channel {
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
