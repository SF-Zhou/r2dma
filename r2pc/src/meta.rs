use std::io::Write;

use crate::{MSG_HEADER, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Meta {
    pub msg_id: u64,
    pub method: String,
    pub flags: u32,
}

#[derive(Serialize)]
pub struct SerializePackage<'a, P: Serialize> {
    pub meta: &'a Meta,
    pub payload: &'a P,
}

#[derive(Deserialize)]
pub struct DeserializePackage<P> {
    pub meta: Meta,
    pub payload: P,
}

#[repr(transparent)]
struct Writer<'a>(&'a mut Vec<u8>);

impl Write for Writer<'_> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.0
            .try_reserve(buf.len())
            .map_err(|_| std::io::ErrorKind::OutOfMemory)?;
        self.0.extend_from_slice(buf);
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Meta {
    pub fn serialize<P: Serialize>(&self, payload: &P) -> Result<Vec<u8>> {
        const S: usize = std::mem::size_of::<u32>();

        let mut bytes: Vec<u8> = Vec::with_capacity(8);
        bytes.extend(MSG_HEADER.to_be_bytes());
        bytes.extend(0u32.to_be_bytes()); // reserve for length.

        let mut writer = Writer(&mut bytes);
        let package = SerializePackage {
            meta: self,
            payload,
        };
        rmp_serde::encode::write(&mut writer, &package).unwrap();

        let payload_len = (bytes.len() - S * 2) as u32;
        bytes[S..S * 2].copy_from_slice(&payload_len.to_be_bytes());

        Ok(bytes)
    }

    pub fn deserialize<P: for<'c> Deserialize<'c>>(&self, value: rmpv::Value) -> Result<P> {
        Ok(P::deserialize(value)?)
    }
}
