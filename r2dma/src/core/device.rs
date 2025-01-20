use super::DeviceConfig;
use crate::{ibv, Result};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceIndex(pub usize);

/// Represents an RDMA device with its associated context, protection domain, port attributes, and GID.
#[derive(Debug)]
pub struct Device {
    // The index of device in list.
    index: DeviceIndex,
    /// The protection domain associated with the device.
    pd: ibv::ProtectionDomain,
    /// The context associated with the device.
    context: ibv::Context,
    /// The attributes of the device,
    device_attr: ibv::ibv_device_attr,
    /// The ports on the device.
    ports: Vec<Port>,
}

#[derive(Debug)]
pub struct Port {
    pub port_num: u8,
    /// The attributes of the port.
    pub port_attr: ibv::ibv_port_attr,
    /// The GID (Global Identifier) list of the port.
    pub gids: Vec<(u16, ibv::ibv_gid, ibv::GidType)>,
}

impl Device {
    pub fn open(index: DeviceIndex, device: &ibv::Device, config: &DeviceConfig) -> Result<Self> {
        let context = ibv::Context::create(device)?;
        let pd = ibv::ProtectionDomain::create(&context)?;
        let device_attr = context.query_device()?;

        let mut ports = vec![];
        for port_num in 1..=device_attr.phys_port_cnt {
            let port_attr = context.query_port(port_num)?;
            let mut gids = vec![];

            for gid_index in 0..port_attr.gid_tbl_len as u16 {
                if let Ok(gid) = context.query_gid(port_num, gid_index) {
                    let gid_type = context.query_gid_type(port_num, gid_index)?;
                    if !config.gid_type_filter.is_empty()
                        && !config.gid_type_filter.contains(&gid_type)
                    {
                        continue;
                    }

                    if config.roce_v2_skip_link_local_addr && gid_type == ibv::GidType::RoCEv2 {
                        let ip = gid.as_ipv6();
                        if ip.is_unicast_link_local() {
                            continue;
                        }
                    }

                    gids.push((gid_index, gid, gid_type))
                }
            }

            ports.push(Port {
                port_num,
                port_attr,
                gids,
            });
        }

        Ok(Self {
            index,
            pd,
            context,
            device_attr,
            ports,
        })
    }

    pub fn index(&self) -> DeviceIndex {
        self.index
    }

    pub fn pd(&self) -> &ibv::ProtectionDomain {
        &self.pd
    }

    pub fn context(&self) -> &ibv::Context {
        &self.context
    }

    pub fn guid(&self) -> u64 {
        self.context.device().guid()
    }

    pub fn name(&self) -> Cow<str> {
        self.context.device().name()
    }

    pub fn device_attr(&self) -> &ibv::ibv_device_attr {
        &self.device_attr
    }

    pub fn ports(&self) -> &Vec<Port> {
        &self.ports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ib_device() {
        let device_list = ibv::DeviceList::available().unwrap();
        let first_device = device_list.first().unwrap();
        let mut config = DeviceConfig::default();
        let device = Device::open(DeviceIndex(0), first_device, &config).unwrap();
        println!("{}: {:#?}", device.name(), device);
        assert_eq!(device.index().0, 0);

        config.gid_type_filter = [ibv::GidType::IB, ibv::GidType::RoCEv2].into();
        config.roce_v2_skip_link_local_addr = true;
        let device = Device::open(DeviceIndex(0), first_device, &config).unwrap();
        println!("{}: {:#?}", device.name(), device);
    }
}
