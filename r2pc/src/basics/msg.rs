use super::{Error, ErrorKind, Result};
use bitflags::bitflags;
use bytes::Bytes;
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

pub trait SendMsg {
    fn len(&self) -> usize;

    fn prepare(&mut self) -> Result<()>;

    fn finish(&mut self, start_offset: usize, meta_len: usize) -> Result<()>;

    fn writer(&mut self) -> impl std::io::Write;
}

impl MsgMeta {
    pub fn serialize_to<M: SendMsg, P: Serialize>(&self, payload: &P, msg: &mut M) -> Result<()> {
        let msg_start_offset = msg.len();
        msg.prepare()?;

        let meta_start_offset = msg.len();
        rmp_serde::encode::write(&mut msg.writer(), self)
            .map_err(|e| Error::new(ErrorKind::SerializeFailed, e.to_string()))?;
        let meta_len = msg.len() - meta_start_offset;

        if self.flags.contains(MsgFlags::IsJson) {
            serde_json::to_writer(msg.writer(), payload)
                .map_err(|e| Error::new(ErrorKind::SerializeFailed, e.to_string()))?;
        } else {
            rmp_serde::encode::write(&mut msg.writer(), &payload)
                .map_err(|e| Error::new(ErrorKind::SerializeFailed, e.to_string()))?;
        }

        msg.finish(msg_start_offset, meta_len)?;

        Ok(())
    }
}

/*
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
*/
