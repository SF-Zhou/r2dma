use super::*;
use std::net::Ipv6Addr;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum GidType {
    IB,
    RoCEv1,
    RoCEv2,
    Other(String),
}

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
        let gid = self
            .as_raw()
            .chunks_exact(2)
            .map(|b| format!("{:02x}{:02x}", b[0], b[1]))
            .reduce(|a, b| format!("{}:{}", a, b))
            .unwrap();
        f.write_str(&gid)
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

#[cfg(test)]
mod tests {
    use super::*;
    use derse::{Deserialize, DownwardBytes, Serialize};

    #[test]
    fn test_gid() {
        let mut gid = ibv_gid::default();
        assert_eq!(gid.subnet_prefix(), 0);
        assert_eq!(gid.interface_id(), 0);
        assert_eq!(gid.as_ipv6().to_string(), "::");
        let bytes: DownwardBytes = gid.serialize().unwrap();
        let mut des = ibv_gid::deserialize(bytes.as_ref()).unwrap();
        assert_eq!(gid.as_mut(), des.as_mut());
    }
}
