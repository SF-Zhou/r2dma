use super::verbs::*;
use super::Device;
use crate::{Error, Result};

pub type Context = super::Wrapper<ibv_context>;

impl Context {
    pub fn create(device: &Device) -> Result<Self> {
        Ok(Self::new(unsafe {
            let context = ibv_open_device(device.as_mut_ptr());
            if context.is_null() {
                return Err(Error::IBOpenDeviceFail(std::io::Error::last_os_error()));
            }
            context
        }))
    }

    pub fn device(&self) -> &Device {
        unsafe { std::mem::transmute(&self.device) }
    }

    pub fn query_device(&self) -> Result<ibv_device_attr> {
        let mut device_attr = ibv_device_attr::default();
        let ret = unsafe { ibv_query_device(self.as_mut_ptr(), &mut device_attr) };
        if ret == 0 {
            Ok(device_attr)
        } else {
            Err(Error::IBQueryDeviceFail(std::io::Error::last_os_error()))
        }
    }

    pub fn query_gid(&self, port_num: u8, gid_index: u16) -> Result<ibv_gid> {
        let mut gid = ibv_gid::default();
        let ret =
            unsafe { ibv_query_gid(self.as_mut_ptr(), port_num as _, gid_index as _, &mut gid) };
        if ret == 0 && !gid.is_null() {
            Ok(gid)
        } else {
            Err(Error::IBQueryGidFail(std::io::Error::last_os_error()))
        }
    }

    pub fn query_port(&self, port_num: u8) -> Result<ibv_port_attr> {
        let mut port_attr = std::mem::MaybeUninit::<ibv_port_attr>::uninit();
        let ret =
            unsafe { ibv_query_port(self.as_mut_ptr(), port_num, port_attr.as_mut_ptr() as _) };
        if ret == 0 {
            Ok(unsafe { port_attr.assume_init() })
        } else {
            Err(Error::IBQueryPortFail(std::io::Error::last_os_error()))
        }
    }

    #[cfg(test)]
    pub fn create_for_test() -> Self {
        let list = super::DeviceList::available().unwrap();
        let first_device = list.first().unwrap();
        Context::create(first_device).unwrap()
    }
}

impl super::Deleter for ibv_context {
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

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_context() {
        let context = Context::create_for_test();
        println!("context: {:#?}", context);

        let device_attr = context.query_device().unwrap();
        println!("device attr: {:#?}", device_attr);

        for port_num in 1..=device_attr.phys_port_cnt {
            let port_attr = context.query_port(port_num).unwrap();
            println!("port {port_num} attr: {:#?}", port_attr);

            for gid_index in 0..port_attr.gid_tbl_len {
                if let Ok(entry) = context.query_gid(port_num, gid_index as u16) {
                    println!("{gid_index}: {:?}", entry);
                }
            }
        }
    }
}
