use std::net::Ipv6Addr;

use super::verbs::{ibv_gid, ibv_gid_entry, ibv_gid_type};

impl ibv_gid {
    pub fn as_raw(&self) -> &[u8; 16] {
        unsafe { &self.raw }
    }

    pub fn as_bits(&self) -> u128 {
        u128::from_be_bytes(unsafe { self.raw })
    }

    pub fn as_ipv6(&self) -> std::net::Ipv6Addr {
        Ipv6Addr::from_bits(self.as_bits())
    }

    pub fn subnet_prefix(&self) -> u64 {
        u64::from_be(unsafe { self.global.subnet_prefix })
    }

    pub fn interface_id(&self) -> u64 {
        u64::from_be(unsafe { self.global.interface_id })
    }

    pub fn is_null(&self) -> bool {
        self.interface_id() == 0
    }
}

impl std::convert::AsMut<[u8]> for ibv_gid {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { &mut self.raw }
    }
}

impl std::fmt::Debug for ibv_gid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let gid = super::bytes_to_hex_string(self.as_raw());
        f.debug_tuple("Gid").field(&gid).finish()
    }
}

impl derse::Serialize for ibv_gid {
    fn serialize_to<S: derse::Serializer>(&self, serializer: &mut S) -> derse::Result<()> {
        serializer.prepend(self.as_raw())
    }
}

impl<'a> derse::Deserialize<'a> for ibv_gid {
    fn deserialize_from<S: derse::Deserializer<'a>>(buf: &mut S) -> derse::Result<Self>
    where
        Self: Sized,
    {
        let mut gid = ibv_gid::default();
        let data = buf.pop(gid.as_mut().len())?;
        gid.as_mut().copy_from_slice(&data);
        Ok(gid)
    }
}

impl ibv_gid_entry {
    pub fn gid_type(&self) -> ibv_gid_type {
        match self.gid_type {
            0 => ibv_gid_type::IBV_GID_TYPE_IB,
            1 => ibv_gid_type::IBV_GID_TYPE_ROCE_V1,
            2 => ibv_gid_type::IBV_GID_TYPE_ROCE_V2,
            i => panic!("invalid gid type {i}"),
        }
    }
}

impl std::fmt::Debug for ibv_gid_entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.gid_type() == ibv_gid_type::IBV_GID_TYPE_ROCE_V2 {
            let ipv6 = self.gid.as_ipv6();
            f.debug_struct("gid_entry")
                .field("gid", &ipv6)
                .field("type", &self.gid_type())
                .finish()
        } else {
            f.debug_struct("gid_entry")
                .field("gid", &self.gid)
                .field("type", &self.gid_type())
                .finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use derse::{Deserialize, DownwardBytes, Serialize};

    #[test]
    fn test_gid() {
        let mut gid = ibv_gid::default();
        let bytes: DownwardBytes = gid.serialize().unwrap();
        let mut des = ibv_gid::deserialize(bytes.as_ref()).unwrap();
        assert_eq!(gid.as_mut(), des.as_mut());
    }
}
