use super::*;
use crate::{verbs, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::{ffi::c_int, ops::Deref, sync::Arc};

#[derive(Debug, Deserialize, Serialize)]
pub struct Endpoint {
    pub qp_num: u32,
    pub lid: u16,
    pub gid: verbs::ibv_gid,
}

struct RawQueuePair(*mut verbs::ibv_qp);
impl Drop for RawQueuePair {
    fn drop(&mut self) {
        let _ = unsafe { verbs::ibv_destroy_qp(self.0) };
    }
}
unsafe impl Send for RawQueuePair {}
unsafe impl Sync for RawQueuePair {}

/// Represents a queue pair in RDMA communication.
/// A queue pair consists of a send queue and a receive queue, which are used to send and receive messages.
pub struct QueuePair {
    queue_pair: RawQueuePair,
    _comp_queues: Arc<CompQueues>,
    _device_index: usize,
    _devices: Devices,
}

impl QueuePair {
    pub fn create(
        devices: &Devices,
        device_index: usize,
        comp_queues: &Arc<CompQueues>,
        cap: verbs::ibv_qp_cap,
    ) -> Result<Self> {
        let mut attr = verbs::ibv_qp_init_attr {
            qp_context: std::ptr::null_mut(),
            send_cq: comp_queues.comp_queue_ptr(device_index),
            recv_cq: comp_queues.comp_queue_ptr(device_index),
            srq: std::ptr::null_mut(),
            cap,
            qp_type: verbs::ibv_qp_type::IBV_QPT_RC,
            sq_sig_all: 0,
        };
        let ptr = unsafe { verbs::ibv_create_qp(devices[device_index].pd_ptr(), &mut attr) };
        if ptr.is_null() {
            return Err(ErrorKind::IBCreateQueuePairFail.with_errno());
        }
        Ok(Self {
            queue_pair: RawQueuePair(ptr),
            _comp_queues: comp_queues.clone(),
            _device_index: device_index,
            _devices: devices.clone(),
        })
    }

    pub fn device(&self) -> &Device {
        &self._devices[self._device_index]
    }

    pub fn init(&self, port_num: u8, pkey_index: u16) -> Result<()> {
        let mut attr = verbs::ibv_qp_attr {
            qp_state: verbs::ibv_qp_state::IBV_QPS_INIT,
            pkey_index,
            port_num,
            qp_access_flags: verbs::ACCESS_FLAGS,
            ..Default::default()
        };

        const MASK: verbs::ibv_qp_attr_mask = verbs::ibv_qp_attr_mask(
            verbs::ibv_qp_attr_mask::IBV_QP_PKEY_INDEX.0
                | verbs::ibv_qp_attr_mask::IBV_QP_STATE.0
                | verbs::ibv_qp_attr_mask::IBV_QP_PORT.0
                | verbs::ibv_qp_attr_mask::IBV_QP_ACCESS_FLAGS.0,
        );

        self.modify_qp(&mut attr, MASK)
    }

    pub fn ready_to_recv(&self, remote: &Endpoint) -> Result<()> {
        let mut attr = verbs::ibv_qp_attr {
            qp_state: verbs::ibv_qp_state::IBV_QPS_RTR,
            path_mtu: verbs::ibv_mtu::IBV_MTU_512,
            dest_qp_num: remote.qp_num,
            rq_psn: 0,
            max_dest_rd_atomic: 1,
            min_rnr_timer: 0x12,
            ah_attr: verbs::ibv_ah_attr {
                grh: verbs::ibv_global_route {
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

        const MASK: verbs::ibv_qp_attr_mask = verbs::ibv_qp_attr_mask(
            verbs::ibv_qp_attr_mask::IBV_QP_STATE.0
                | verbs::ibv_qp_attr_mask::IBV_QP_AV.0
                | verbs::ibv_qp_attr_mask::IBV_QP_PATH_MTU.0
                | verbs::ibv_qp_attr_mask::IBV_QP_DEST_QPN.0
                | verbs::ibv_qp_attr_mask::IBV_QP_RQ_PSN.0
                | verbs::ibv_qp_attr_mask::IBV_QP_MAX_DEST_RD_ATOMIC.0
                | verbs::ibv_qp_attr_mask::IBV_QP_MIN_RNR_TIMER.0,
        );

        self.modify_qp(&mut attr, MASK)
    }

    pub fn ready_to_send(&self) -> Result<()> {
        let mut attr = verbs::ibv_qp_attr {
            qp_state: verbs::ibv_qp_state::IBV_QPS_RTS,
            timeout: 0x12,
            retry_cnt: 6,
            rnr_retry: 6,
            sq_psn: 0,
            max_rd_atomic: 1,
            ..Default::default()
        };

        const MASK: verbs::ibv_qp_attr_mask = verbs::ibv_qp_attr_mask(
            verbs::ibv_qp_attr_mask::IBV_QP_STATE.0
                | verbs::ibv_qp_attr_mask::IBV_QP_TIMEOUT.0
                | verbs::ibv_qp_attr_mask::IBV_QP_RETRY_CNT.0
                | verbs::ibv_qp_attr_mask::IBV_QP_RNR_RETRY.0
                | verbs::ibv_qp_attr_mask::IBV_QP_SQ_PSN.0
                | verbs::ibv_qp_attr_mask::IBV_QP_MAX_QP_RD_ATOMIC.0,
        );

        self.modify_qp(&mut attr, MASK)
    }

    pub fn set_error(&self) {
        let mut attr = verbs::ibv_qp_attr {
            qp_state: verbs::ibv_qp_state::IBV_QPS_ERR,
            ..Default::default()
        };

        const MASK: verbs::ibv_qp_attr_mask = verbs::ibv_qp_attr_mask::IBV_QP_STATE;

        // assuming this operation succeeds.
        self.modify_qp(&mut attr, MASK).unwrap()
    }

    pub fn post_send(&self, wr: &mut verbs::ibv_send_wr) -> c_int {
        let mut bad_wr = std::ptr::null_mut();
        unsafe { verbs::ibv_post_send(self.queue_pair.0, wr, &mut bad_wr) }
    }

    pub fn post_recv(&self, wr: &mut verbs::ibv_recv_wr) -> c_int {
        let mut bad_wr = std::ptr::null_mut();
        unsafe { verbs::ibv_post_recv(self.queue_pair.0, wr, &mut bad_wr) }
    }

    fn modify_qp(
        &self,
        attr: &mut verbs::ibv_qp_attr,
        mask: verbs::ibv_qp_attr_mask,
    ) -> Result<()> {
        let ret = unsafe { verbs::ibv_modify_qp(self.queue_pair.0, attr, mask.0 as _) };
        if ret == 0_i32 {
            Ok(())
        } else {
            Err(ErrorKind::IBModifyQueuePairFail.with_errno())
        }
    }
}

impl Deref for QueuePair {
    type Target = verbs::ibv_qp;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.queue_pair.0 }
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
    use crate::*;

    #[test]
    fn test_queue_pair_create() {
        let devices = Devices::availables().unwrap();
        let comp_queues = CompQueues::create(&devices, 128).unwrap();
        let cap = verbs::ibv_qp_cap {
            max_send_wr: 64,
            max_recv_wr: 64,
            max_send_sge: 1,
            max_recv_sge: 1,
            max_inline_data: 0,
        };
        let queue_pair = QueuePair::create(&devices, 0, &comp_queues, cap).unwrap();
        println!("{:#?}", queue_pair);

        queue_pair.init(1, 0).unwrap();
        queue_pair.set_error();
    }

    #[test]
    fn test_queue_pair_send_recv() {
        // 1. list all available devices.
        let devices = Devices::availables().unwrap();

        // 2. create two queue pairs.
        let cap = verbs::ibv_qp_cap {
            max_send_wr: 64,
            max_recv_wr: 64,
            max_send_sge: 1,
            max_recv_sge: 1,
            max_inline_data: 0,
        };

        let comp_queues_a = CompQueues::create(&devices, 128).unwrap();
        let queue_pair_a = QueuePair::create(&devices, 0, &comp_queues_a, cap).unwrap();
        let socket_a = Socket::create(Arc::new(queue_pair_a));
        let comp_queues_b = CompQueues::create(&devices, 128).unwrap();
        let queue_pair_b = QueuePair::create(&devices, 0, &comp_queues_b, cap).unwrap();
        let socket_b = Socket::create(Arc::new(queue_pair_b));

        // 3. init all queue pairs.
        socket_a.init(socket_b.endpoint()).unwrap();
        socket_b.init(socket_a.endpoint()).unwrap();

        // 4. post recv wr.
        const LEN: usize = 1 << 20;
        let buffer_pool = BufferPool::create(LEN, 32, &devices).unwrap();

        let mut recv_buf = buffer_pool.allocate().unwrap();
        recv_buf.fill(0);
        let recv_slice: &[u8] = unsafe { std::mem::transmute(&*recv_buf) };
        socket_b.post_recv(1, recv_buf).unwrap();

        // 5. try to poll cq.
        let mut wcs_b = vec![verbs::ibv_wc::default(); 128];
        assert!(comp_queues_b.poll_cq(&mut wcs_b).unwrap().is_empty());

        // 6. post send wr.
        let mut send_buf = buffer_pool.allocate().unwrap();
        send_buf.fill(1);
        let send_slice: &[u8] = unsafe { std::mem::transmute(&*send_buf) };
        let send_len = send_buf.len();
        socket_a.post_send(2, send_buf).unwrap();

        // 7. poll cq.
        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut wcs_a = vec![verbs::ibv_wc::default(); 128];
        let comp_a = comp_queues_a.poll_cq(&mut wcs_a).unwrap();
        assert_eq!(comp_a.len(), 1);
        assert_eq!(comp_a[0].wr_id, 2);
        assert_eq!(comp_a[0].qp_num, socket_a.qp_num());
        assert_eq!(comp_a[0].status, verbs::ibv_wc_status::IBV_WC_SUCCESS);

        let comp_b = comp_queues_b.poll_cq(&mut wcs_b).unwrap();
        assert_eq!(comp_b.len(), 1);
        assert_eq!(comp_b[0].wr_id, 1);
        assert_eq!(comp_b[0].qp_num, socket_b.qp_num());
        assert_eq!(comp_b[0].status, verbs::ibv_wc_status::IBV_WC_SUCCESS);
        assert_eq!(comp_b[0].byte_len, send_len as u32);
        assert!(&recv_slice[..send_len] == send_slice);
    }
}
