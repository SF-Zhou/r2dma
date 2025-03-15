use super::*;
use crate::{Error, Result};
use std::{
    borrow::Cow,
    ffi::{c_int, CStr, OsStr},
    ops::Deref,
    os::unix::ffi::OsStrExt,
    path::Path,
    sync::Arc,
};

struct RawDeviceList {
    ptr: *mut *mut ibv_device,
    num_devices: usize,
}

impl RawDeviceList {
    fn available() -> Result<Self> {
        let mut num_devices: c_int = 0;
        let ptr = unsafe { ibv_get_device_list(&mut num_devices) };
        if ptr.is_null() {
            return Err(Error::IBGetDeviceListFail(std::io::Error::last_os_error()));
        }
        if num_devices == 0 {
            return Err(Error::IBDeviceNotFound);
        }
        Ok(Self {
            ptr,
            num_devices: num_devices as usize,
        })
    }
}

impl Drop for RawDeviceList {
    fn drop(&mut self) {
        unsafe { ibv_free_device_list(self.ptr) };
    }
}

unsafe impl Send for RawDeviceList {}
unsafe impl Sync for RawDeviceList {}

#[derive(Clone)]
pub struct Device {
    list: Arc<RawDeviceList>,
    index: usize,
}

impl Device {
    pub fn availables() -> Result<Vec<Self>> {
        let list = Arc::new(RawDeviceList::available()?);
        let out = (0..list.num_devices)
            .map(|index| Self {
                list: list.clone(),
                index,
            })
            .collect();
        Ok(out)
    }

    pub fn name(&self) -> Cow<str> {
        unsafe { CStr::from_ptr(self.name.as_ptr()) }.to_string_lossy()
    }

    pub fn guid(&self) -> u64 {
        u64::from_be(unsafe { ibv_get_device_guid(self.as_mut_ptr()) })
    }

    pub fn ibdev_path(&self) -> &Path {
        let str = unsafe { CStr::from_ptr(self.ibdev_path.as_ptr()) };
        Path::new(OsStr::from_bytes(str.to_bytes()))
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut ibv_device {
        unsafe { *self.list.ptr.add(self.index) }
    }
}

impl Deref for Device {
    type Target = ibv_device;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_mut_ptr() }
    }
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        let guid = format!("{:016x}", self.guid());
        let ibdev_path = self.ibdev_path();
        f.debug_struct("ibv_device")
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
    use super::*;

    #[test]
    fn test_devices() {
        let devices = Device::availables().unwrap();
        assert!(!devices.is_empty());
        println!("{:#?}", devices);
    }
}
