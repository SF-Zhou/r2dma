use super::{Device, DeviceConfig, DeviceIndex};
use crate::{ibv, Result};

/// Represents a collection of InfiniBand devices and their associated device list.
#[derive(Debug)]
pub struct Devices {
    /// A vector of `Device` instances.
    devices: Vec<Device>,
    /// The list of available InfiniBand devices.
    _device_list: ibv::DeviceList,
}

impl Devices {
    pub fn open(config: &DeviceConfig) -> Result<Self> {
        let mut devices = vec![];

        let device_list = ibv::DeviceList::available()?;
        for (idx, device) in device_list.iter().enumerate() {
            let device = Device::open(DeviceIndex(idx), device, config)?;
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

        Ok(Self {
            devices,
            _device_list: device_list,
        })
    }

    pub fn iter(&self) -> std::slice::Iter<Device> {
        self.devices.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devices() {
        let mut config = DeviceConfig::default();
        let devices = Devices::open(&config).unwrap();
        let first_device = devices.iter().next().unwrap().name().to_string();
        for device in devices.iter() {
            println!("{:#?}", *device);
        }

        config.device_filter = [first_device].into();
        let devices = Devices::open(&config).unwrap();
        assert!(devices.iter().next().is_some());

        config.device_filter = ["invalid".to_owned()].into();
        let devices = Devices::open(&config).unwrap();
        assert!(devices.iter().next().is_none());
    }
}
