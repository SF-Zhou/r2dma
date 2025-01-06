use crate::Result;
use derse::{Deserialize, DownwardBytes, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Meta {
    pub msg_id: u64,
    pub method: String,
    pub flags: u32,
}

impl Meta {
    pub fn serialize<P: Serialize>(&self, payload: &P) -> Result<DownwardBytes> {
        let mut bytes: DownwardBytes = payload.serialize()?;
        self.serialize_to(&mut bytes)?;
        let len = bytes.len();
        bytes.prepend(len.to_be_bytes());
        Ok(bytes)
    }
}
