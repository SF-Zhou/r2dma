use super::{Device, Gid};
use crate::*;
use r2dma_sys::*;

pub type Context = Wrapper<ibv_context>;

impl Context {
    pub fn device(&self) -> &Device {
        unsafe { std::mem::transmute(&self.device) }
    }

    pub fn query_gid(&self, port_num: u8, gid_index: u16) -> Result<Gid> {
        let mut gid = std::mem::MaybeUninit::<ibv_gid>::uninit();
        let ret = unsafe {
            ibv_query_gid(
                self.as_mut_ptr(),
                port_num,
                gid_index as _,
                gid.as_mut_ptr(),
            )
        };
        if ret == 0 {
            Ok(Gid::from(unsafe { gid.assume_init() }))
        } else {
            Err(Error::with_errno(ErrorKind::IBQueryGidFail))
        }
    }

    pub fn query_port(&self, port_num: u8) -> Result<ibv_port_attr> {
        let mut port_attr = std::mem::MaybeUninit::<ibv_port_attr>::uninit();
        let ret =
            unsafe { ibv_query_port(self.as_mut_ptr(), port_num, port_attr.as_mut_ptr() as _) };
        if ret == 0 {
            Ok(unsafe { port_attr.assume_init() })
        } else {
            Err(Error::with_errno(ErrorKind::IBQueryPortFail))
        }
    }
}

impl Deleter for ibv_context {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        ibv_close_device(ptr)
    }
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let device = self.device();
        f.debug_struct("Context")
            .field("device", &device)
            .field("cmd_fd", &self.cmd_fd)
            .field("async_fd", &self.async_fd)
            .field("num_comp_vectors", &self.num_comp_vectors)
            .field("abi_compat", &self.abi_compat)
            .finish()
    }
}
