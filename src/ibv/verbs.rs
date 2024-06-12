#![allow(dead_code)]
#![allow(deref_nullptr)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(clippy::missing_safety_doc, clippy::too_many_arguments)]

use libc::{c_int, pthread_cond_t, pthread_mutex_t};
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
