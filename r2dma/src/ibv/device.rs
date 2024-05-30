use crate::*;
use r2dma_sys::*;
use std::borrow::Cow;
use std::ffi::CStr;

pub type Device = Wrapper<ibv_device>;

impl Device {
    pub fn name(&self) -> Cow<str> {
        unsafe { CStr::from_ptr(self.name.as_ptr()).to_string_lossy() }
    }

    pub fn guid(&self) -> u64 {
        u64::from_be(unsafe { ibv_get_device_guid(self.as_mut_ptr()) })
    }
}

impl Deleter for ibv_device {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        unreachable!("invalid deletion to Device {ptr:?}!")
    }
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        let guid = crate::utils::bytes_to_hex_string(&self.guid().to_be_bytes());
        f.debug_struct("Device")
            .field("name", &name)
            .field("guid", &guid)
            .field("node_type", &self.node_type)
            .field("transport_type", &self.transport_type)
            .finish()
    }
}
