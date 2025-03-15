use super::DeviceConfig;
use crate::{ibv, Result};
use std::{borrow::Cow, sync::Arc};

/// Represents an RDMA device with its associated context, protection domain, port attributes, and GID.
#[derive(Debug)]
pub struct Device {
    /// The context associated with the device.
    context: Arc<ibv::Context>,
    /// The attributes of the device,
    device_attr: ibv::ibv_device_attr,
    /// The ports on the device.
    ports: Vec<Port>,
    /// The protection domain associated with the device.
    pd: ibv::ProtectionDomain,
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
    pub fn avaiables(config: &DeviceConfig) -> Result<Vec<Self>> {
        let mut devices = vec![];

        for device in ibv::Device::availables()? {
            let device = Device::open(&device, config)?;
            if !config.device_filter.is_empty()
                && !config.device_filter.contains(device.name().as_ref())
            {
                tracing::debug!(
                    "skip device {} by filter: {:?}",
                    device.name().as_ref(),
                    config.device_filter
                );
                continue;
            }
            devices.push(device);
        }

        Ok(devices)
    }

    pub fn open(device: &ibv::Device, config: &DeviceConfig) -> Result<Self> {
        let context = Arc::new(ibv::Context::create(device)?);
        let pd = ibv::ProtectionDomain::create(context.clone())?;
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
            context,
            device_attr,
            ports,
            pd,
        })
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
        let config = DeviceConfig {
            device_filter: Default::default(),
            gid_type_filter: [ibv::GidType::IB, ibv::GidType::RoCEv2].into(),
            roce_v2_skip_link_local_addr: true,
        };
        let devices = Device::avaiables(&config).unwrap();
        let device = devices.first().unwrap();
        println!("{}: {:#?}", device.name(), device);
    }
}
