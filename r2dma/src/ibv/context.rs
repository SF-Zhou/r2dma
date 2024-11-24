use super::verbs::*;
use super::Device;
use crate::{Error, Result};

pub type Context = super::Wrapper<ibv_context>;

impl Context {
    pub fn create(device: &Device) -> Result<Self> {
        Ok(Self::new(unsafe {
            let context = ibv_open_device(device.as_mut_ptr());
            if context.is_null() {
                return Err(Error::IBOpenDeviceFail);
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
            Err(Error::IBQueryGidFail)
        }
    }

    pub fn query_gid(&self, port_num: u8, gid_index: u16) -> Result<ibv_gid_entry> {
        let mut entry = ibv_gid_entry::default();
        let ret = unsafe {
            _ibv_query_gid_ex(
                self.as_mut_ptr(),
                port_num as _,
                gid_index as _,
                &mut entry,
                0,
                std::mem::size_of::<ibv_gid_entry>(),
            )
        };
        if ret == 0 {
            Ok(entry)
        } else {
            Err(Error::IBQueryGidFail)
        }
    }

    pub fn query_port(&self, port_num: u8) -> Result<ibv_port_attr> {
        let mut port_attr = std::mem::MaybeUninit::<ibv_port_attr>::uninit();
        let ret =
            unsafe { ibv_query_port(self.as_mut_ptr(), port_num, port_attr.as_mut_ptr() as _) };
        if ret == 0 {
            Ok(unsafe { port_attr.assume_init() })
        } else {
            Err(Error::IBQueryPortFail)
        }
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
        let list = DeviceList::available().unwrap();
        let first_device = list.first().unwrap();
        let context = Context::create(first_device).unwrap();

        let device_attr = context.query_device().unwrap();
        println!("device attr: {:#?}", device_attr);

        for port_num in 1..=device_attr.phys_port_cnt {
            let port_attr = context.query_port(port_num);
            println!("port {port_num} attr: {:#?}", port_attr);
        }
    }
}
