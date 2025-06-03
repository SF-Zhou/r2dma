#![allow(dead_code)]
#![allow(deref_nullptr)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::missing_safety_doc, clippy::too_many_arguments)]

use std::{net::Ipv6Addr, os::raw::c_int};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[repr(transparent)]
pub struct pthread_mutex_t(pub libc::pthread_mutex_t);

impl std::fmt::Debug for pthread_mutex_t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("pthread_mutex_t").finish()
    }
}

#[repr(transparent)]
pub struct pthread_cond_t(pub libc::pthread_cond_t);

impl std::fmt::Debug for pthread_cond_t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("pthread_cond_t").finish()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum IBV_LINK_LAYER {
    UNSPECIFIED = 0,
    INFINIBAND = 1,
    ETHERNET = 2,
}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[inline(always)]
pub unsafe fn ibv_req_notify_cq(cq: *mut ibv_cq, solicited_only: c_int) -> c_int {
    (*(*cq).context).ops.req_notify_cq.unwrap_unchecked()(cq, solicited_only)
}

#[inline(always)]
pub unsafe fn ibv_poll_cq(cq: *mut ibv_cq, num_entries: c_int, wc: *mut ibv_wc) -> c_int {
    (*(*cq).context).ops.poll_cq.unwrap_unchecked()(cq, num_entries, wc)
}

#[inline(always)]
pub unsafe fn ibv_post_send(
    qp: *mut ibv_qp,
    wr: *mut ibv_send_wr,
    bad_wr: *mut *mut ibv_send_wr,
) -> c_int {
    (*(*qp).context).ops.post_send.unwrap_unchecked()(qp, wr, bad_wr)
}

#[inline(always)]
pub unsafe fn ibv_post_recv(
    qp: *mut ibv_qp,
    wr: *mut ibv_recv_wr,
    bad_wr: *mut *mut ibv_recv_wr,
) -> c_int {
    (*(*qp).context).ops.post_recv.unwrap_unchecked()(qp, wr, bad_wr)
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

impl std::fmt::Debug for ibv_gid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let gid = self
            .as_raw()
            .chunks_exact(2)
            .map(|b| format!("{:02x}{:02x}", b[0], b[1]))
            .reduce(|a, b| format!("{a}:{b}"))
            .unwrap();
        f.write_str(&gid)
    }
}

impl Serialize for ibv_gid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.as_raw())
    }
}

impl<'de> Deserialize<'de> for ibv_gid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut gid = ibv_gid::default();
        gid.raw = <[u8; 16]>::deserialize(deserializer)?;
        Ok(gid)
    }
}

pub const ACCESS_FLAGS: u32 = ibv_access_flags::IBV_ACCESS_LOCAL_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_READ.0
    | ibv_access_flags::IBV_ACCESS_RELAXED_ORDERING.0;
