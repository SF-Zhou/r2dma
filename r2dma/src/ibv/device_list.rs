use crate::{ibv::*, Error, Result};
use std::{ffi::c_int, ops::Deref};

pub type DeviceList = super::Wrapper<[Device]>;

impl DeviceList {
    pub fn available() -> Result<Self> {
        let mut num_devices: c_int = 0;
        let arr = unsafe { ibv_get_device_list(&mut num_devices) };
        if arr.is_null() {
            return Err(Error::IBGetDeviceListFail(std::io::Error::last_os_error()));
        }
        if num_devices == 0 {
            return Err(Error::IBDeviceNotFound);
        }

        Ok(Self::new(
            std::ptr::slice_from_raw_parts_mut(arr, num_devices as usize) as _,
        ))
    }
}

impl super::Deleter for [Device] {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        ibv_free_device_list(ptr as _);
        0
    }
}

impl std::fmt::Debug for DeviceList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DeviceList").field(&self.deref()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_list() {
        let list = DeviceList::available().unwrap();
        assert!(!list.is_empty());
        println!("{:#?}", list);
    }
}
