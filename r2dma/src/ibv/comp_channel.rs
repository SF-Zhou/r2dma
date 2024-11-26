use super::verbs::*;
use crate::*;

use std::os::fd::BorrowedFd;

pub type CompChannel = super::Wrapper<ibv_comp_channel>;

impl CompChannel {
    pub fn create(context: &ibv::Context) -> Result<Self> {
        let channel = unsafe { ibv_create_comp_channel(context.as_mut_ptr()) };
        if channel.is_null() {
            return Err(Error::IBCreateCompChannelFail);
        }
        Ok(Self::new(channel))
    }

    pub fn fd(&self) -> BorrowedFd {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }

    pub fn set_nonblock(&self) -> Result<()> {
        let flags = unsafe { libc::fcntl(self.fd, libc::F_GETFL) };
        if flags == -1 {
            return Err(Error::SetNonBlockFail);
        }

        let ret = unsafe { libc::fcntl(self.fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if ret == 0 {
            Ok(())
        } else {
            Err(Error::SetNonBlockFail)
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
            Err(Error::IBGetCQEventFail)
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
    use super::super::*;

    #[test]
    fn test_comp_channel() {
        let context = Context::create_for_test();
        let comp_channel = CompChannel::create(&context).unwrap();
        comp_channel.set_nonblock().unwrap();
        println!("{:#?}", comp_channel);
    }
}
