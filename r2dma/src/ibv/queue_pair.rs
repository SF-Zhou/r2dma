use super::Endpoint;
use crate::*;
use r2dma_sys::*;

pub const ACCESS_FLAGS: u32 = ibv_access_flags::IBV_ACCESS_LOCAL_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_READ.0
    | ibv_access_flags::IBV_ACCESS_RELAXED_ORDERING.0;

pub type QueuePair = utils::Wrapper<ibv_qp>;

impl QueuePair {
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
                    dgid: *remote.gid,
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

    fn modify_qp(&self, attr: &mut ibv_qp_attr, mask: ibv_qp_attr_mask) -> Result<()> {
        let ret = unsafe { ibv_modify_qp(self.as_mut_ptr(), attr, mask.0 as _) };
        if ret == 0_i32 {
            Ok(())
        } else {
            Err(Error::with_errno(ErrorKind::IBModifyQPFail))
        }
    }
}

impl utils::Deleter for ibv_qp {
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
