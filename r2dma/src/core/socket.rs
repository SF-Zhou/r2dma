use crate::*;
use ibv::WorkCompletion;
use r2dma_sys::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct Socket {
    pub(super) queue_pair: ibv::QueuePair,
    pub(super) comp_queue: ibv::CompQueue,
    pub(super) channel: Arc<Channel>,
    pub(super) unack_events_count: AtomicU32,
    pub(super) state: State,
}

impl Socket {
    pub fn notify(&self) -> Result<()> {
        match unsafe { ibv_req_notify_cq(self.comp_queue.as_mut_ptr(), 0) } {
            0 => Ok(()),
            _ => Err(Error::with_errno(ErrorKind::IBReqNotifyCQFail)),
        }
    }

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

    pub fn from_cq_context<'a>(cq_context: *mut std::ffi::c_void) -> &'a Self {
        let socket = unsafe { &*(cq_context as *const Socket) };
        socket.unack_events_count.fetch_add(1, Ordering::Relaxed);
        socket
    }

    pub fn poll_completions<'a>(
        &self,
        wc: &'a mut [ibv::WorkCompletion],
    ) -> Result<&'a [ibv::WorkCompletion]> {
        let num_entries = wc.len() as i32;
        let num = unsafe {
            ibv_poll_cq(
                self.comp_queue.as_mut_ptr(),
                num_entries,
                wc.as_mut_ptr() as _,
            )
        };
        if num >= 0 {
            Ok(&wc[..num as usize])
        } else {
            Err(Error::with_errno(ErrorKind::IBPollCQFail))
        }
    }

    pub fn submit_work(&self, work: WorkRef) -> Result<()> {
        // 1. prepare to submit.
        let ok = self.state.prepare_submit();
        if !ok {
            self.rollback_submit_and_try_to_remove();
            return Err(Error::new(ErrorKind::IBSocketError));
        }

        // 2. create sge.
        let mut sge = work.buf.as_ref().map(|buf| ibv_sge {
            addr: buf.as_ref().as_ptr() as u64,
            length: buf.as_ref().len() as u32,
            lkey: buf.lkey(),
        });
        let (sg_list, num_sge) = match &mut sge {
            Some(sge) => (sge as _, 1),
            None => (std::ptr::null_mut(), 0),
        };

        // 3. create work request and post.
        let ret = match work.ty {
            WorkType::Send => {
                let mut wr = ibv_send_wr {
                    wr_id: work.ptr() as _,
                    sg_list,
                    num_sge,
                    opcode: match work.imm {
                        Some(_) => ibv_wr_opcode::IBV_WR_SEND_WITH_IMM,
                        None => ibv_wr_opcode::IBV_WR_SEND,
                    },
                    send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
                    __bindgen_anon_1: ibv_send_wr__bindgen_ty_1 {
                        imm_data: work.imm.map_or(0, |n| n.get()).to_be(),
                    },
                    ..Default::default()
                };
                let mut bad_wr = std::ptr::null_mut();
                unsafe { ibv_post_send(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
            }
            WorkType::Recv => {
                let mut wr = ibv_recv_wr {
                    wr_id: work.ptr() as _,
                    next: std::ptr::null_mut(),
                    sg_list,
                    num_sge,
                };
                let mut bad_wr = std::ptr::null_mut();
                unsafe { ibv_post_recv(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
            }
        };

        // 4. check return code.
        if ret == 0 {
            work.release();
            Ok(())
        } else {
            self.rollback_submit_and_try_to_remove();
            Err(Error::with_errno(ErrorKind::IBPostSendFail))
        }
    }

    fn rollback_submit_and_try_to_remove(&self) {
        let need_remove = self.state.rollback_submit_and_try_to_remove();
        if need_remove {
            // TODO(SF): remove this socket.
        }
    }

    pub fn set_to_error(&self) -> Result<()> {
        let need_remove = self.state.set_error_and_try_to_remove();
        let result = self.queue_pair.set_to_error();
        if need_remove {
            // TODO(SF): remove this socket in event loop.
        }
        result
    }

    pub fn on_send(&self, wc: &WorkCompletion, work: WorkRef) -> Result<()> {
        tracing::info!("on send: {wc:#?}, work: {work:?}");

        Ok(())
    }

    pub fn on_recv(&self, wc: &WorkCompletion, work: WorkRef) -> Result<()> {
        tracing::info!("on send: {wc:#?}, work: {work:?}");

        self.submit_work(work) // re-submit it.
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            ibv_ack_cq_events(
                self.comp_queue.as_mut_ptr(),
                self.unack_events_count.load(Ordering::Acquire),
            );
        }
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
    use std::num::NonZeroU32;

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
        work.ty = WorkType::Send;
        work.buf = Some(buf);
        send_socket.submit_work(work).unwrap();

        // 2. send a imm.
        let mut work = manager.allocate_work().unwrap();
        work.ty = WorkType::Send;
        work.imm = Some(NonZeroU32::new(2333).unwrap());
        send_socket.submit_work(work).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 3. multi-send.
        for _ in 0..100 {
            let buf = manager.allocate_buffer().unwrap();
            let mut work = manager.allocate_work().unwrap();
            work.ty = WorkType::Send;
            work.buf = Some(buf);
            send_socket.submit_work(work).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        send_socket.set_to_error().unwrap();
        recv_socket.set_to_error().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
