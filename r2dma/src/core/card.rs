use crate::*;
use r2dma_sys::*;
use std::{borrow::Cow, sync::Arc};

#[derive(Debug)]
pub struct Card {
    pub protection_domain: ibv::ProtectionDomain,
    pub context: ibv::Context,
    pub port_attr: ibv_port_attr,
    pub gid: ibv::Gid,
}

impl Card {
    pub fn open(device: &ibv::Device) -> Result<Arc<Self>> {
        let context = ibv::Context::new(unsafe {
            let context = ibv_open_device(device.as_mut_ptr());
            if context.is_null() {
                return Err(Error::with_errno(ErrorKind::IBOpenDeviceFail));
            }
            context
        });

        let protection_domain = ibv::ProtectionDomain::new(unsafe {
            let protection_domain = ibv_alloc_pd(context.as_mut_ptr());
            if protection_domain.is_null() {
                return Err(Error::with_errno(ErrorKind::IBAllocPDFail));
            }
            protection_domain
        });

        let port_attr = context.query_port(1)?;
        let gid = context.query_gid(1, 1)?;

        Ok(Arc::new(Self {
            protection_domain,
            context,
            port_attr,
            gid,
        }))
    }

    pub fn name(&self) -> Cow<str> {
        self.context.device().name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ib_device() {
        let device_list = ibv::DeviceList::available().unwrap();
        let first_device = device_list.first().unwrap();
        let card = Card::open(first_device).unwrap();
        println!("{}: {:#?}", card.name(), card);

        let context = &card.context;
        let gid = context.query_gid(1, 0).unwrap();
        println!("{:?} {} {}", gid, gid.subnet_prefix(), gid.interface_id());
        assert!(context.query_gid(1, u16::MAX).is_err());

        let port_attr = context.query_port(1).unwrap();
        println!("{:#?}", port_attr);
        assert!(context.query_port(10).is_err());
    }
}
