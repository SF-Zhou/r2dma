use crate::ibv;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct Config {
    pub device: DeviceConfig,
}

#[derive(Debug, Default)]
pub struct DeviceConfig {
    pub device_filter: HashSet<String>,
    pub gid_type_filter: HashSet<ibv::GidType>,
    pub roce_v2_skip_link_local_addr: bool,
}
