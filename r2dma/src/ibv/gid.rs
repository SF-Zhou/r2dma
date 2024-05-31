use r2dma_sys::ibv_gid;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Gid(ibv_gid);

impl Gid {
    pub fn as_raw(&self) -> &[u8; 16] {
        unsafe { &self.0.raw }
    }

    pub fn as_mut(&mut self) -> &mut [u8] {
        unsafe { &mut self.0.raw }
    }

    pub fn subnet_prefix(&self) -> u64 {
        u64::from_be(unsafe { self.0.global.subnet_prefix })
    }

    pub fn interface_id(&self) -> u64 {
        u64::from_be(unsafe { self.0.global.interface_id })
    }
}

impl std::fmt::Debug for Gid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let gid = crate::utils::bytes_to_hex_string(self.as_raw());
        f.debug_tuple("Gid").field(&gid).finish()
    }
}

impl std::ops::Deref for Gid {
    type Target = ibv_gid;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ibv_gid> for Gid {
    fn from(value: ibv_gid) -> Self {
        Gid(value)
    }
}

impl Default for Gid {
    fn default() -> Self {
        Self(unsafe { std::mem::zeroed() })
    }
}

impl<'a> derse::Serialization<'a> for Gid {
    fn serialize_to<S: derse::Serializer>(&self, serializer: &mut S) -> derse::Result<()> {
        serializer.prepend(self.as_raw())
    }

    fn deserialize_from<S: derse::Deserializer<'a>>(buf: &mut S) -> derse::Result<Self>
    where
        Self: Sized,
    {
        let data = buf.pop(16)?;
        let mut gid = Gid::default();
        gid.as_mut().copy_from_slice(&data);
        Ok(gid)
    }
}

#[cfg(test)]
mod tests {
    use super::Gid;
    use derse::{DownwardBytes, Serialization};

    #[test]
    fn test_gid() {
        let mut gid = Gid::default();
        let bytes: DownwardBytes = gid.serialize().unwrap();
        let mut des = Gid::deserialize(bytes.as_ref()).unwrap();
        assert_eq!(gid.as_mut(), des.as_mut());
    }
}
