use crate::{ibv::*, Error, Result};
use std::os::fd::BorrowedFd;

/// A completion channel is essentially file descriptor that is used to deliver completion
/// notifications to a userspace process.  When a completion event is generated for a completion
/// queue (CQ), the event is delivered via the completion channel attached to that CQ.
pub type CompChannel = super::Wrapper<ibv_comp_channel>;

impl CompChannel {
    pub fn create(context: &Context) -> Result<Self> {
        let channel = unsafe { ibv_create_comp_channel(context.as_mut_ptr()) };
        if channel.is_null() {
            return Err(Error::IBCreateCompChannelFail(
                std::io::Error::last_os_error(),
            ));
        }
        Ok(Self::new(channel))
    }

    pub fn fd(&self) -> BorrowedFd {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }

    pub fn set_nonblock(&self) -> Result<()> {
        let flags = unsafe { libc::fcntl(self.fd, libc::F_GETFL) };
        if flags == -1 {
            return Err(Error::IBSetCompChannelNonBlockFail(
                std::io::Error::last_os_error(),
            ));
        }

        let ret = unsafe { libc::fcntl(self.fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Error::IBSetCompChannelNonBlockFail(
                std::io::Error::last_os_error(),
            ))
        }
    }

    pub fn get_cq_event<T>(&self) -> Result<Option<&mut T>> {
        let mut comp_queue: *mut ibv_cq = std::ptr::null_mut();
        let mut cq_context: *mut std::ffi::c_void = std::ptr::null_mut();
        let ret = unsafe { ibv_get_cq_event(self.as_mut_ptr(), &mut comp_queue, &mut cq_context) };
        if ret == 0 {
            Ok(Some(unsafe { &mut *(cq_context as *mut _) }))
        } else if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(Error::IBGetCompQueueEventFail(
                std::io::Error::last_os_error(),
            ))
        }
    }
}

impl super::Deleter for ibv_comp_channel {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd};

    #[test]
    fn test_comp_channel() {
        let context = Context::create_for_test();
        let comp_channel = CompChannel::create(&context).unwrap();
        comp_channel.set_nonblock().unwrap();
        assert_ne!(comp_channel.fd().as_raw_fd(), -1);
        println!("{:#?}", comp_channel);

        let value = comp_channel.get_cq_event::<i32>().unwrap();
        assert!(value.is_none());

        unsafe { std::fs::File::from_raw_fd(comp_channel.fd) };
        comp_channel.set_nonblock().unwrap_err();

        unsafe { std::fs::File::from_raw_fd(context.cmd_fd) };
        CompChannel::create(&context).unwrap_err();
    }
}
