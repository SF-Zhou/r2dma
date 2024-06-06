use crate::*;
use r2dma_sys::*;
use std::sync::Arc;

pub struct Socket {
    pub(super) queue_pair: ibv::QueuePair,
    pub(super) comp_queue: ibv::CompQueue,
    pub(super) channel: Arc<Channel>,
    pub(super) state: State,
}

impl Socket {
    pub fn endpoint(&self) -> Endpoint {
        Endpoint {
            qp_num: self.queue_pair.qp_num,
            lid: self.channel.card.port_attr.lid,
            gid: self.channel.card.gid,
        }
    }

    pub fn ready(&self, remote: &Endpoint) -> Result<()> {
        self.queue_pair.ready_to_recv(remote)?;
        self.queue_pair.ready_to_send()
    }

    pub(super) fn submit_work_request_id(&self, wr_id: WorkRequestId, sge: &mut ibv_sge) -> i32 {
        // 1. create sge.
        let (sg_list, num_sge) = match sge.addr {
            0 => (std::ptr::null_mut(), 0),
            _ => (sge as _, 1),
        };

        // 2. create work request and post.
        match wr_id {
            WorkRequestId::Empty => {
                0 // do nothing.
            }
            WorkRequestId::SendData(_)
            | WorkRequestId::AsyncSendData(_)
            | WorkRequestId::SendMsg(_, _) => {
                let mut wr = ibv_send_wr {
                    wr_id: wr_id.into(),
                    sg_list,
                    num_sge,
                    opcode: ibv_wr_opcode::IBV_WR_SEND,
                    send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
                    ..Default::default()
                };
                let mut bad_wr = std::ptr::null_mut();
                unsafe { ibv_post_send(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
            }
            WorkRequestId::SendImm(imm) | WorkRequestId::AsyncSendImm(imm) => {
                let mut wr = ibv_send_wr {
                    wr_id: wr_id.into(),
                    opcode: ibv_wr_opcode::IBV_WR_SEND_WITH_IMM,
                    send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
                    __bindgen_anon_1: ibv_send_wr__bindgen_ty_1 { imm_data: imm.to_be() },
                    ..Default::default()
                };
                let mut bad_wr = std::ptr::null_mut();
                unsafe { ibv_post_send(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
            }
            WorkRequestId::RecvData(_) => {
                let mut wr = ibv_recv_wr {
                    wr_id: wr_id.into(),
                    next: std::ptr::null_mut(),
                    sg_list,
                    num_sge,
                };
                let mut bad_wr = std::ptr::null_mut();
                unsafe { ibv_post_recv(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
            }
        }
    }

    pub(super) fn submit_work_unchecked<W: Submittable>(&self, work: W) -> Result<()> {
        let wr_id = work.wr_id();
        let mut sge = work.sge();
        let ret = self.submit_work_request_id(wr_id, &mut sge);
        if ret == 0 {
            work.release();
            Ok(())
        } else {
            Err(Error::with_errno(ErrorKind::IBPostSendFail))
        }
    }

    pub fn submit_send_work<W: Submittable>(&self, work: W) -> Result<()> {
        match self.state.apply_send(16) {
            core::state::ApplyResult::Succ => self.submit_work_unchecked(work),
            core::state::ApplyResult::Async => {
                let work = AsyncWork(work);
                self.submit_work_unchecked(work)
            }
            core::state::ApplyResult::Error => Err(Error::new(ErrorKind::IBSocketError)),
        }
    }

    pub fn submit_recv_work(&self, mut work: WorkRef) -> Result<()> {
        work.recv = true;
        match self.state.apply_recv() {
            core::state::ApplyResult::Succ => self.submit_work_unchecked(work),
            core::state::ApplyResult::Error => Err(Error::new(ErrorKind::IBSocketError)),
            _ => unreachable!(),
        }
    }

    pub fn set_to_error(&self) -> Result<()> {
        let need_send_empty_wr = self.state.set_error();
        let result = self.queue_pair.set_to_error();
        if need_send_empty_wr {
            let mut sge = ibv_sge::default();
            let ret = self
                .submit_work_request_id(WorkRequestId::send_msg(WorkRequestId::Empty), &mut sge);
            if ret != 0 {
                // TODO(SF): notify event loop.
            }
        }
        result
    }
}

impl std::fmt::Debug for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket")
            .field("queue_pair", &self.queue_pair)
            .field("comp_queue", &self.comp_queue)
            .field("state", &self.state)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use derse::Serialization;

    use super::*;

    #[tokio::test]
    async fn test_ib_socket() {
        let config = Config::default();
        let manager = Manager::init(&config).unwrap();

        let send_socket = manager.create_socket().unwrap();
        println!("send socket: {:#?}", send_socket);
        let recv_socket = manager.create_socket().unwrap();
        println!("recv socket: {:#?}", recv_socket);

        let send_endpoint = send_socket.endpoint();
        println!("send endpoint: {:#?}", send_endpoint);
        let _ = send_endpoint.serialize::<derse::DownwardBytes>();
        let recv_endpoint = recv_socket.endpoint();
        println!("recv endpoint: {:#?}", recv_endpoint);

        send_socket.ready(&recv_endpoint).unwrap();
        recv_socket.ready(&send_endpoint).unwrap();

        let mut buf = manager.allocate_buffer().unwrap();
        println!("send memory: {:#?}", buf);
        buf.as_mut().fill(0x23);

        // 1. send a buffer.
        let mut work = manager.allocate_work().unwrap();
        work.buf = Some(buf);
        send_socket.submit_send_work(work).unwrap();

        // 2. send a imm.
        let work = manager.allocate_work().unwrap();
        send_socket.submit_send_work(work).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 3. multi-send.
        for _ in 0..200 {
            let buf = manager.allocate_buffer().unwrap();
            let mut work = manager.allocate_work().unwrap();
            work.buf = Some(buf);
            send_socket.submit_send_work(work).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        send_socket.set_to_error().unwrap();
        recv_socket.set_to_error().unwrap();

        drop(send_socket);
        drop(recv_socket);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
