use super::*;
use crate::*;
use derse::{Deserialize, Serialize};
use std::ffi::c_int;

pub const ACCESS_FLAGS: u32 = ibv_access_flags::IBV_ACCESS_LOCAL_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_READ.0
    | ibv_access_flags::IBV_ACCESS_RELAXED_ORDERING.0;

#[derive(Debug, Deserialize, Serialize)]
pub struct Endpoint {
    pub qp_num: u32,
    pub lid: u16,
    pub gid: ibv_gid,
}

pub type QueuePair = super::Wrapper<ibv_qp>;

impl QueuePair {
    pub fn create(
        pd: &ibv::ProtectionDomain,
        comp_queue: &CompQueue,
        cap: ibv_qp_cap,
    ) -> Result<Self> {
        let mut attr = ibv_qp_init_attr {
            qp_context: std::ptr::null_mut(),
            send_cq: comp_queue.as_mut_ptr(),
            recv_cq: comp_queue.as_mut_ptr(),
            srq: std::ptr::null_mut(),
            cap,
            qp_type: ibv_qp_type::IBV_QPT_RC,
            sq_sig_all: 0,
        };
        let queue_pair = unsafe { ibv_create_qp(pd.as_mut_ptr(), &mut attr) };
        if queue_pair.is_null() {
            return Err(Error::IBCreateQueuePairFail(std::io::Error::last_os_error()));
        }
        Ok(Self::new(queue_pair))
    }

    pub fn init(&mut self, port_num: u8, pkey_index: u16) -> Result<()> {
        let mut attr = ibv_qp_attr {
            qp_state: ibv_qp_state::IBV_QPS_INIT,
            pkey_index,
            port_num,
            qp_access_flags: ACCESS_FLAGS,
            ..Default::default()
        };

        let mask = ibv_qp_attr_mask::IBV_QP_PKEY_INDEX
            | ibv_qp_attr_mask::IBV_QP_STATE
            | ibv_qp_attr_mask::IBV_QP_PORT
            | ibv_qp_attr_mask::IBV_QP_ACCESS_FLAGS;

        self.modify_qp(&mut attr, mask)
    }

    pub fn ready_to_recv(&self, remote: &Endpoint) -> Result<()> {
        let mut attr = ibv_qp_attr {
            qp_state: ibv_qp_state::IBV_QPS_RTR,
            path_mtu: ibv_mtu::IBV_MTU_512,
            dest_qp_num: remote.qp_num,
            rq_psn: 0,
            max_dest_rd_atomic: 1,
            min_rnr_timer: 0x12,
            ah_attr: ibv_ah_attr {
                grh: ibv_global_route {
                    dgid: remote.gid,
                    flow_label: 0,
                    sgid_index: 1,
                    hop_limit: 0xff,
                    traffic_class: 0,
                },
                dlid: remote.lid,
                sl: 0,
                src_path_bits: 0,
                static_rate: 0,
                is_global: 1,
                port_num: 1,
            },
            ..Default::default()
        };

        let mask = ibv_qp_attr_mask::IBV_QP_STATE
            | ibv_qp_attr_mask::IBV_QP_AV
            | ibv_qp_attr_mask::IBV_QP_PATH_MTU
            | ibv_qp_attr_mask::IBV_QP_DEST_QPN
            | ibv_qp_attr_mask::IBV_QP_RQ_PSN
            | ibv_qp_attr_mask::IBV_QP_MAX_DEST_RD_ATOMIC
            | ibv_qp_attr_mask::IBV_QP_MIN_RNR_TIMER;

        self.modify_qp(&mut attr, mask)
    }

    pub fn ready_to_send(&self) -> Result<()> {
        let mut attr = ibv_qp_attr {
            qp_state: ibv_qp_state::IBV_QPS_RTS,
            timeout: 0x12,
            retry_cnt: 6,
            rnr_retry: 6,
            sq_psn: 0,
            max_rd_atomic: 1,
            ..Default::default()
        };

        let mask = ibv_qp_attr_mask::IBV_QP_STATE
            | ibv_qp_attr_mask::IBV_QP_TIMEOUT
            | ibv_qp_attr_mask::IBV_QP_RETRY_CNT
            | ibv_qp_attr_mask::IBV_QP_RNR_RETRY
            | ibv_qp_attr_mask::IBV_QP_SQ_PSN
            | ibv_qp_attr_mask::IBV_QP_MAX_QP_RD_ATOMIC;

        self.modify_qp(&mut attr, mask)
    }

    pub fn set_error(&self) {
        let mut attr = ibv_qp_attr {
            qp_state: ibv_qp_state::IBV_QPS_ERR,
            ..Default::default()
        };

        let mask = ibv_qp_attr_mask::IBV_QP_STATE;

        // assuming this operation succeeds.
        self.modify_qp(&mut attr, mask).unwrap()
    }

    pub fn post_send(&self, wr: &mut ibv_send_wr) -> c_int {
        let mut bad_wr = std::ptr::null_mut();
        unsafe { ibv_post_send(self.as_mut_ptr(), wr, &mut bad_wr) }
    }

    pub fn post_recv(&self, wr: &mut ibv_recv_wr) -> c_int {
        let mut bad_wr = std::ptr::null_mut();
        unsafe { ibv_post_recv(self.as_mut_ptr(), wr, &mut bad_wr) }
    }

    fn modify_qp(&self, attr: &mut ibv_qp_attr, mask: ibv_qp_attr_mask) -> Result<()> {
        let ret = unsafe { ibv_modify_qp(self.as_mut_ptr(), attr, mask.0 as _) };
        if ret == 0_i32 {
            Ok(())
        } else {
            Err(Error::IBModifyQueuePairFail(std::io::Error::last_os_error()))
        }
    }
}

impl super::Deleter for ibv_qp {
    unsafe fn delete(ptr: *mut Self) -> i32 {
        ibv_destroy_qp(ptr)
    }
}

impl std::fmt::Debug for QueuePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuePair")
            .field("handle", &self.handle)
            .field("qp_num", &self.qp_num)
            .field("state", &self.state)
            .field("qp_type", &self.qp_type)
            .field("events_completiond", &self.events_completed)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_queue_pair_create() {
        let context = Context::create_for_test();
        let comp_channel = CompChannel::create(&context).unwrap();
        let comp_queue = CompQueue::create(&context, 128, &comp_channel).unwrap();
        let pd = ProtectionDomain::create(&context).unwrap();
        let cap = ibv_qp_cap {
            max_send_wr: 64,
            max_recv_wr: 64,
            max_send_sge: 1,
            max_recv_sge: 1,
            max_inline_data: 0,
        };
        let mut queue_pair = QueuePair::create(&pd, &comp_queue, cap).unwrap();
        println!("{:#?}", queue_pair);
        println!("{:#?}", unsafe { &*queue_pair.as_mut_ptr() });

        queue_pair.init(1, 0).unwrap();
        queue_pair.set_error();
    }
}
