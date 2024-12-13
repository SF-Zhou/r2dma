use super::*;
use std::borrow::Cow;
use std::ffi::CStr;

pub type Device = super::Wrapper<ibv_device>;

impl Device {
    pub fn name(&self) -> Cow<str> {
        unsafe { CStr::from_ptr(self.name.as_ptr()).to_string_lossy() }
    }

    pub fn guid(&self) -> u64 {
        u64::from_be(unsafe { ibv_get_device_guid(self.as_mut_ptr()) })
    }

    pub fn ibdev_path(&self) -> Cow<str> {
        unsafe { CStr::from_ptr(self.ibdev_path.as_ptr()).to_string_lossy() }
    }
}

impl super::Deleter for ibv_device {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        unreachable!("invalid deletion to Device {ptr:?}!")
    }
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        let guid = format!("{:016x}", self.guid());
        let ibdev_path = self.ibdev_path();
        f.debug_struct("Device")
            .field("name", &name)
            .field("guid", &guid)
            .field("node_type", &self.node_type)
            .field("transport_type", &self.transport_type)
            .field("ibdev_path", &ibdev_path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[should_panic]
    fn test_device() {
        let _ = super::Device::new(std::ptr::null_mut());
    }
}
