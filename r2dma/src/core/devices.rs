use super::{Device, DeviceIndex};
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
    pub fn open() -> Result<Self> {
        let mut devices = vec![];

        let device_list = ibv::DeviceList::available()?;
        for (idx, device) in device_list.iter().enumerate() {
            let device = Device::open(DeviceIndex(idx), device)?;
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
        let devices = Devices::open().unwrap();
        for device in devices.iter() {
            println!("{:#?}", *device);
        }
    }
}
