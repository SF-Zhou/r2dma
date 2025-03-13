use super::*;
use crate::{verbs::*, Error, Result};
use std::ops::Deref;

/// A verb context that can be used for future operations on the device.
pub struct Context {
    device: Device,
    ptr: *mut ibv_context,
}

impl Drop for Context {
    fn drop(&mut self) {
        let _ = unsafe { ibv_close_device(self.ptr) };
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    pub fn create(device: &Device) -> Result<Self> {
        let ptr = unsafe {
            let context = ibv_open_device(device.as_mut_ptr());
            if context.is_null() {
                return Err(Error::IBOpenDeviceFail(std::io::Error::last_os_error()));
            }
            context
        };
        Ok(Self {
            device: device.clone(),
            ptr,
        })
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn query_device(&self) -> Result<ibv_device_attr> {
        let mut device_attr = ibv_device_attr::default();
        let ret = unsafe { ibv_query_device(self.ptr, &mut device_attr) };
        if ret == 0 {
            Ok(device_attr)
        } else {
            Err(Error::IBQueryDeviceFail(std::io::Error::last_os_error()))
        }
    }

    pub fn query_gid(&self, port_num: u8, gid_index: u16) -> Result<ibv_gid> {
        let mut gid = ibv_gid::default();
        let ret = unsafe { ibv_query_gid(self.ptr, port_num as _, gid_index as _, &mut gid) };
        if ret == 0 && !gid.is_null() {
            Ok(gid)
        } else {
            Err(Error::IBQueryGidFail(std::io::Error::last_os_error()))
        }
    }

    pub fn query_gid_type(&self, port_num: u8, gid_index: u16) -> Result<GidType> {
        let path = self
            .device()
            .ibdev_path()
            .join(format!("ports/{}/gid_attrs/types/{}", port_num, gid_index));
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if content == "IB/RoCE v1\n" {
                    let port_attr = self.query_port(port_num)?;
                    if port_attr.link_layer == IBV_LINK_LAYER::INFINIBAND as u8 {
                        Ok(GidType::IB)
                    } else {
                        Ok(GidType::RoCEv1)
                    }
                } else if content == "RoCE v2\n" {
                    Ok(GidType::RoCEv2)
                } else {
                    Ok(GidType::Other(content.trim().to_string()))
                }
            }
            Err(err) => Err(Error::IBQueryGidTypeFail(err)),
        }
    }

    pub fn query_port(&self, port_num: u8) -> Result<ibv_port_attr> {
        let mut port_attr = std::mem::MaybeUninit::<ibv_port_attr>::uninit();
        let ret = unsafe { ibv_query_port(self.ptr, port_num, port_attr.as_mut_ptr() as _) };
        if ret == 0 {
            Ok(unsafe { port_attr.assume_init() })
        } else {
            Err(Error::IBQueryPortFail(std::io::Error::last_os_error()))
        }
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut ibv_context {
        self.ptr
    }
}

impl Deref for Context {
    type Target = ibv_context;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_mut_ptr() }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context() {
        let devices = Device::availables().unwrap();
        assert!(!devices.is_empty());
        let context = Context::create(devices.first().unwrap()).unwrap();
        println!("context: {:#?}", context);

        let device_attr = context.query_device().unwrap();
        println!("device attr: {:#?}", device_attr);

        for port_num in 1..=device_attr.phys_port_cnt {
            let port_attr = context.query_port(port_num).unwrap();
            println!("port {port_num} attr: {:#?}", port_attr);

            for gid_index in 0..port_attr.gid_tbl_len {
                if let Ok(gid) = context.query_gid(port_num, gid_index as u16) {
                    let gid_type = context.query_gid_type(port_num, gid_index as _).unwrap();
                    if gid_type == GidType::RoCEv2 {
                        println!("{gid_index}: {}", gid.as_ipv6());
                    } else {
                        println!("{gid_index}: {:?}", gid);
                    }
                }
            }
        }
    }
}
