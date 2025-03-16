use super::*;
use crate::{Error, Result};
use derse::{Deserialize, Serialize};
use std::{ffi::c_int, ops::Deref, sync::Arc};

#[derive(Debug, Deserialize, Serialize)]
pub struct Endpoint {
    pub qp_num: u32,
    pub lid: u16,
    pub gid: ibv_gid,
}

pub struct QueuePair {
    _pd: Arc<ProtectionDomain>,
    _comp_queue: Arc<CompQueue>,
    ptr: *mut ibv_qp,
}

impl Drop for QueuePair {
    fn drop(&mut self) {
        let _ = unsafe { ibv_destroy_qp(self.ptr) };
    }
}

unsafe impl Send for QueuePair {}
unsafe impl Sync for QueuePair {}

impl QueuePair {
    pub fn create(
        pd: &Arc<ProtectionDomain>,
        comp_queue: &Arc<CompQueue>,
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
        let ptr = unsafe { ibv_create_qp(pd.as_mut_ptr(), &mut attr) };
        if ptr.is_null() {
            return Err(Error::IBCreateQueuePairFail(std::io::Error::last_os_error()));
        }
        Ok(Self {
            _pd: pd.clone(),
            _comp_queue: comp_queue.clone(),
            ptr,
        })
    }

    pub fn init(&mut self, port_num: u8, pkey_index: u16) -> Result<()> {
        let mut attr = ibv_qp_attr {
            qp_state: ibv_qp_state::IBV_QPS_INIT,
            pkey_index,
            port_num,
            qp_access_flags: ACCESS_FLAGS,
            ..Default::default()
        };

        const MASK: ibv_qp_attr_mask = ibv_qp_attr_mask(
            ibv_qp_attr_mask::IBV_QP_PKEY_INDEX.0
                | ibv_qp_attr_mask::IBV_QP_STATE.0
                | ibv_qp_attr_mask::IBV_QP_PORT.0
                | ibv_qp_attr_mask::IBV_QP_ACCESS_FLAGS.0,
        );

        self.modify_qp(&mut attr, MASK)
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

        const MASK: ibv_qp_attr_mask = ibv_qp_attr_mask(
            ibv_qp_attr_mask::IBV_QP_STATE.0
                | ibv_qp_attr_mask::IBV_QP_AV.0
                | ibv_qp_attr_mask::IBV_QP_PATH_MTU.0
                | ibv_qp_attr_mask::IBV_QP_DEST_QPN.0
                | ibv_qp_attr_mask::IBV_QP_RQ_PSN.0
                | ibv_qp_attr_mask::IBV_QP_MAX_DEST_RD_ATOMIC.0
                | ibv_qp_attr_mask::IBV_QP_MIN_RNR_TIMER.0,
        );

        self.modify_qp(&mut attr, MASK)
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

        const MASK: ibv_qp_attr_mask = ibv_qp_attr_mask(
            ibv_qp_attr_mask::IBV_QP_STATE.0
                | ibv_qp_attr_mask::IBV_QP_TIMEOUT.0
                | ibv_qp_attr_mask::IBV_QP_RETRY_CNT.0
                | ibv_qp_attr_mask::IBV_QP_RNR_RETRY.0
                | ibv_qp_attr_mask::IBV_QP_SQ_PSN.0
                | ibv_qp_attr_mask::IBV_QP_MAX_QP_RD_ATOMIC.0,
        );

        self.modify_qp(&mut attr, MASK)
    }

    pub fn set_error(&self) {
        let mut attr = ibv_qp_attr {
            qp_state: ibv_qp_state::IBV_QPS_ERR,
            ..Default::default()
        };

        const MASK: ibv_qp_attr_mask = ibv_qp_attr_mask::IBV_QP_STATE;

        // assuming this operation succeeds.
        self.modify_qp(&mut attr, MASK).unwrap()
    }

    pub fn post_send(&self, wr: &mut ibv_send_wr) -> c_int {
        let mut bad_wr = std::ptr::null_mut();
        unsafe { ibv_post_send(self.ptr, wr, &mut bad_wr) }
    }

    pub fn post_recv(&self, wr: &mut ibv_recv_wr) -> c_int {
        let mut bad_wr = std::ptr::null_mut();
        unsafe { ibv_post_recv(self.ptr, wr, &mut bad_wr) }
    }

    fn modify_qp(&self, attr: &mut ibv_qp_attr, mask: ibv_qp_attr_mask) -> Result<()> {
        let ret = unsafe { ibv_modify_qp(self.ptr, attr, mask.0 as _) };
        if ret == 0_i32 {
            Ok(())
        } else {
            Err(Error::IBModifyQueuePairFail(std::io::Error::last_os_error()))
        }
    }

    #[allow(unused)]
    pub fn as_mut_ptr(&self) -> *mut ibv_qp {
        self.ptr
    }
}

impl Deref for QueuePair {
    type Target = ibv_qp;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
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
    use super::*;

    #[test]
    fn test_queue_pair_create() {
        let devices = Device::availables().unwrap();
        let context = Context::create(devices.first().unwrap()).unwrap();
        let comp_queue = CompQueue::create(&context, 128, None).unwrap();
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

        queue_pair.init(1, 0).unwrap();
        queue_pair.set_error();
    }

    #[test]
    fn test_queue_pair_send_recv() {
        let devices = Device::availables().unwrap();
        let device = devices.first().unwrap();
        let context = Context::create(device).unwrap();
        let pd = ProtectionDomain::create(&context).unwrap();
        let cap = ibv_qp_cap {
            max_send_wr: 64,
            max_recv_wr: 64,
            max_send_sge: 1,
            max_recv_sge: 1,
            max_inline_data: 0,
        };

        let comp_queue_a = CompQueue::create(&context, 128, None).unwrap();
        let mut queue_pair_a = QueuePair::create(&pd, &comp_queue_a, cap).unwrap();
        let comp_queue_b = CompQueue::create(&context, 128, None).unwrap();
        let mut queue_pair_b = QueuePair::create(&pd, &comp_queue_b, cap).unwrap();

        queue_pair_a.init(1, 0).unwrap();
        queue_pair_b.init(1, 0).unwrap();

        let recv_buf = vec![0u8; 1024];
        let recv_mr = MemoryRegion::create(&pd, &recv_buf).unwrap();
        let mut recv_sge = ibv_sge {
            addr: recv_mr.addr as _,
            length: recv_mr.length as _,
            lkey: recv_mr.lkey,
        };
        let mut recv_wr = ibv_recv_wr {
            wr_id: 1,
            sg_list: &mut recv_sge as *mut _,
            num_sge: 1,
            next: std::ptr::null_mut(),
        };
        assert_eq!(queue_pair_b.post_recv(&mut recv_wr), 0);

        let gid = context.query_gid(1, 1).unwrap();
        queue_pair_a
            .ready_to_recv(&Endpoint {
                qp_num: queue_pair_b.qp_num,
                lid: 0,
                gid,
            })
            .unwrap();
        queue_pair_b
            .ready_to_recv(&Endpoint {
                qp_num: queue_pair_a.qp_num,
                lid: 0,
                gid,
            })
            .unwrap();

        queue_pair_a.ready_to_send().unwrap();
        queue_pair_b.ready_to_send().unwrap();

        let mut wcs_b = vec![ibv_wc::default(); 128];
        assert!(comp_queue_b.poll_cq(&mut wcs_b).unwrap().is_empty());

        let send_buf = vec![1u8; 233];
        let send_mr = MemoryRegion::create(&pd, &send_buf).unwrap();
        let mut send_sge = ibv_sge {
            addr: send_mr.addr as _,
            length: send_mr.length as _,
            lkey: send_mr.lkey,
        };
        let mut send_wr = ibv_send_wr {
            wr_id: 2,
            sg_list: &mut send_sge as *mut _,
            num_sge: 1,
            opcode: ibv_wr_opcode::IBV_WR_SEND,
            send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
            ..Default::default()
        };
        assert_eq!(queue_pair_a.post_send(&mut send_wr), 0);

        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut wcs_a = vec![ibv_wc::default(); 128];
        let comp_a = comp_queue_a.poll_cq(&mut wcs_a).unwrap();
        assert_eq!(comp_a.len(), 1);
        assert_eq!(comp_a[0].wr_id, 2);
        assert_eq!(comp_a[0].qp_num, queue_pair_a.qp_num);
        assert_eq!(comp_a[0].status, ibv_wc_status::IBV_WC_SUCCESS);

        let comp_b = comp_queue_b.poll_cq(&mut wcs_b).unwrap();
        assert_eq!(comp_b.len(), 1);
        assert_eq!(comp_b[0].wr_id, 1);
        assert_eq!(comp_b[0].qp_num, queue_pair_b.qp_num);
        assert_eq!(comp_b[0].status, ibv_wc_status::IBV_WC_SUCCESS);
        assert_eq!(comp_b[0].byte_len, send_buf.len() as u32);
        assert_eq!(recv_buf[..send_buf.len()], send_buf[..]);
    }
}
