use crate::{MSG_HEADER, Result};
use derse::{Deserialize, DownwardBytes, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Meta {
    pub msg_id: u64,
    pub method: String,
    pub flags: u32,
}

impl Meta {
    pub fn serialize<P: Serialize>(&self, payload: &P) -> Result<DownwardBytes> {
        let mut bytes: DownwardBytes = payload.serialize()?;
        self.serialize_to(&mut bytes)?;
        let len = bytes.len() as u32;
        bytes.prepend(len.to_be_bytes());
        bytes.prepend(MSG_HEADER.to_be_bytes());
        Ok(bytes)
    }

    pub fn deserialize<'a, P: Deserialize<'a>>(&self, bytes: &'a [u8]) -> Result<P> {
        let payload = Deserialize::deserialize(bytes)?;
        Ok(payload)
    }
}
