use super::{Error, ErrorKind, Result};
use bitflags::bitflags;
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Clone, Copy)]
#[repr(transparent)]
#[serde(transparent)]
pub struct MsgFlags(u8);

bitflags! {
    impl MsgFlags: u8 {
        const IsReq = 1;
        const IsJson = 2;
        const IsCompressed = 4;
    }
}

#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Clone)]
pub struct MsgMeta {
    pub msg_id: u64,
    pub flags: MsgFlags,
    pub method: String,
}

/// Represents a message in the R2PC protocol.
/// The message is serialized with metadata at the beginning,
/// followed by the payload. The metadata includes the message ID, flags,
/// and method name. The payload can be in JSON or binary format,
/// depending on the flags set in the metadata.
#[derive(Default)]
pub struct Msg {
    /// The metadata of the message, including ID, flags, and method.
    pub meta: MsgMeta,
    /// The serialized message bytes, including the metadata and payload.
    bytes: Bytes,
    /// The offset in the bytes where the payload starts.
    offset: usize,
}

const META_LEN_SIZE: usize = std::mem::size_of::<u32>();

impl Msg {
    pub fn serialize<P: Serialize>(meta: MsgMeta, payload: &P) -> Result<Self> {
        let mut bytes = BytesMut::with_capacity(1024);
        bytes.extend([0u8; META_LEN_SIZE]); // reserve space for the metadata length.

        let mut writer = Writer(&mut bytes);
        rmp_serde::encode::write(&mut writer, &meta)
            .map_err(|e| Error::new(ErrorKind::SerializeFailed, e.to_string()))?;

        // after writing the metadata, we need to update the length of the metadata in the bytes.
        let meta_len = bytes.len() - META_LEN_SIZE;
        bytes[..META_LEN_SIZE].copy_from_slice(&(meta_len as u32).to_be_bytes());

        // now we can serialize the payload.
        if meta.flags.contains(MsgFlags::IsJson) {
            let writer = Writer(&mut bytes);
            serde_json::to_writer(writer, payload)
                .map_err(|e| Error::new(ErrorKind::SerializeFailed, e.to_string()))?;
        } else {
            let mut writer = Writer(&mut bytes);
            rmp_serde::encode::write(&mut writer, &payload)
                .map_err(|e| Error::new(ErrorKind::SerializeFailed, e.to_string()))?;
        }

        Ok(Self {
            meta,
            bytes: bytes.into(),
            offset: META_LEN_SIZE + meta_len,
        })
    }

    pub fn deserialize_meta(bytes: Bytes) -> Result<Self> {
        let len = bytes.len();
        if len < META_LEN_SIZE {
            return Err(Error::new(
                ErrorKind::DeserializeFailed,
                format!("invalid msg length: {len}"),
            ));
        }

        let meta_len = u32::from_be_bytes(bytes[..META_LEN_SIZE].try_into().unwrap()) as usize;
        let offset = META_LEN_SIZE + meta_len;
        if offset > len {
            return Err(Error::new(
                ErrorKind::DeserializeFailed,
                format!("invalid meta length: {meta_len}, msg length: {len}"),
            ));
        }

        let meta: MsgMeta = rmp_serde::from_slice(&bytes[META_LEN_SIZE..offset]).map_err(|e| {
            Error::new(
                ErrorKind::DeserializeFailed,
                format!("failed to deserialize msg meta: {e}"),
            )
        })?;

        Ok(Msg {
            meta,
            bytes,
            offset,
        })
    }

    pub fn payload(&self) -> &[u8] {
        &self.bytes[self.offset..]
    }

    pub fn deserialize_payload<P: for<'c> Deserialize<'c>>(&self) -> Result<P> {
        if self.meta.flags.contains(MsgFlags::IsJson) {
            serde_json::from_slice(self.payload()).map_err(|e| {
                Error::new(
                    ErrorKind::DeserializeFailed,
                    format!("failed to deserialize json payload: {e}"),
                )
            })
        } else {
            rmp_serde::from_slice(self.payload()).map_err(|e| {
                Error::new(
                    ErrorKind::DeserializeFailed,
                    format!("failed to deserialize msg payload: {e}"),
                )
            })
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let payload_len = self.bytes.len() - self.offset;
        f.debug_struct("Msg")
            .field("meta", &self.meta)
            .field("payload_len", &payload_len)
            .finish()
    }
}

#[repr(transparent)]
struct Writer<'a>(&'a mut BytesMut);

impl std::io::Write for Writer<'_> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.0.extend_from_slice(buf);
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_msg_serialize_deserialize() {
        let meta = MsgMeta {
            msg_id: 1,
            flags: MsgFlags::IsReq | MsgFlags::IsJson,
            method: "test_method".to_string(),
        };
        let payload = json!({
            "key1": "value1",
            "key2": 42,
            "key3": [1, 2, 3],
        });

        let msg = Msg::serialize(meta.clone(), &payload).unwrap();
        assert_eq!(msg.meta, meta);
        assert!(msg.bytes.len() > 0);
        println!("serialized message: {:?}", msg);
        println!("payload string: {}", String::from_utf8_lossy(msg.payload()));

        let de = Msg::deserialize_meta(msg.bytes).unwrap();
        assert_eq!(de.meta, meta);
        assert_eq!(
            de.deserialize_payload::<serde_json::Value>().unwrap(),
            payload
        );
    }
}
