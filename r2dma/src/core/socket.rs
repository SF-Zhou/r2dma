use crate::{verbs, Buffer, ErrorKind, Result};
use std::sync::Arc;

use super::{Endpoint, QueuePair};

#[derive(Debug, Clone)]
pub struct Socket {
    queue_pair: Arc<QueuePair>,
}

impl Socket {
    pub fn create(queue_pair: Arc<QueuePair>) -> Self {
        Socket { queue_pair }
    }

    pub fn qp_num(&self) -> u32 {
        self.queue_pair.qp_num
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint {
            qp_num: self.queue_pair.qp_num,
            lid: 0,
            gid: self.queue_pair.device().info().ports[0].gids[1].1,
        }
    }

    pub fn init(&self, endpoint: Endpoint) -> Result<()> {
        self.queue_pair.init(1, 0)?;
        self.queue_pair.ready_to_recv(&endpoint)?;
        self.queue_pair.ready_to_send()?;
        Ok(())
    }

    pub fn post_recv(&self, wr_id: u64, buf: Buffer) -> Result<()> {
        let mut recv_sge = verbs::ibv_sge {
            addr: buf.as_ptr() as _,
            length: buf.len() as _,
            lkey: buf.lkey(self.queue_pair.device()),
        };
        let mut recv_wr = verbs::ibv_recv_wr {
            wr_id,
            sg_list: &mut recv_sge as *mut _,
            num_sge: 1,
            next: std::ptr::null_mut(),
        };

        match self.queue_pair.post_recv(&mut recv_wr) {
            0 => Ok(()),
            _ => Err(ErrorKind::IBPostRecvFailed.with_errno()),
        }
    }

    pub fn post_send(&self, wr_id: u64, buf: Buffer) -> Result<()> {
        let mut send_sge = verbs::ibv_sge {
            addr: buf.as_ptr() as _,
            length: buf.len() as _,
            lkey: buf.lkey(self.queue_pair.device()),
        };
        let mut send_wr = verbs::ibv_send_wr {
            wr_id,
            sg_list: &mut send_sge as *mut _,
            num_sge: 1,
            opcode: verbs::ibv_wr_opcode::IBV_WR_SEND,
            send_flags: verbs::ibv_send_flags::IBV_SEND_SIGNALED.0,
            ..Default::default()
        };

        match self.queue_pair.post_send(&mut send_wr) {
            0 => Ok(()),
            _ => Err(ErrorKind::IBPostSendFailed.with_errno()),
        }
    }
}
