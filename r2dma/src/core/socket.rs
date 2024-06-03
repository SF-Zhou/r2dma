use crate::*;
use r2dma_sys::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

pub struct Socket {
    work_pool: Arc<WorkPool>,
    queue_pair: ibv::QueuePair,
    comp_queue: ibv::CompQueue,
    channel: Arc<Channel>,
    unack_events_count: AtomicU32,
    pub(super) ready_to_error: AtomicBool,
}

impl Socket {
    pub fn create(event_loop: &Arc<EventLoop>, config: &Config) -> Result<Arc<Self>> {
        let card = &event_loop.card;
        let (comp_queue, cq_context) = unsafe {
            let comp_queue = ibv_create_cq(
                card.context.as_mut_ptr(),
                config.max_cqe as _,
                std::ptr::null_mut(),
                event_loop.channel.as_mut_ptr(),
                0,
            );
            if comp_queue.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCQFail));
            }
            (comp_queue, &mut (*comp_queue).cq_context)
        };
        let comp_queue = ibv::CompQueue::new(comp_queue);

        let mut attr = ibv_qp_init_attr {
            qp_context: std::ptr::null_mut(),
            send_cq: comp_queue.as_mut_ptr(),
            recv_cq: comp_queue.as_mut_ptr(),
            srq: std::ptr::null_mut(),
            cap: ibv_qp_cap {
                max_send_wr: config.max_wr as _,
                max_recv_wr: config.max_wr as _,
                max_send_sge: config.max_sge as _,
                max_recv_sge: config.max_sge as _,
                max_inline_data: 0,
            },
            qp_type: ibv_qp_type::IBV_QPT_RC,
            sq_sig_all: 0,
        };
        let mut queue_pair = ibv::QueuePair::new(unsafe {
            let queue_pair = ibv_create_qp(card.protection_domain.as_mut_ptr(), &mut attr);
            if queue_pair.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCQFail));
            }
            queue_pair
        });

        queue_pair.init(1, 0)?;

        let arc = Arc::new(Self {
            work_pool: event_loop.work_pool.clone(),
            queue_pair,
            comp_queue,
            channel: event_loop.channel.clone(),
            unack_events_count: Default::default(),
            ready_to_error: Default::default(),
        });

        *cq_context = arc.as_ref() as *const _ as _;
        arc.notify()?;
        event_loop.add_socket(arc.clone());

        Ok(arc)
    }

    pub fn notify(&self) -> Result<()> {
        unsafe {
            let ret = ibv_req_notify_cq(self.comp_queue.as_mut_ptr(), 0);
            if ret != 0 {
                return Err(Error::with_errno(ErrorKind::IBReqNotifyCQFail));
            }
        };
        Ok(())
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

    pub fn poll<'a>(&self, wc: &'a mut [ibv::WorkCompletion]) -> Result<&'a [ibv::WorkCompletion]> {
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

    pub fn send(&self, buf: BufferSlice) -> Result<tokio::sync::oneshot::Receiver<Result<u32>>> {
        let slice = buf.as_ref();
        let mut sge = ibv_sge {
            addr: slice.as_ptr() as u64,
            length: slice.len() as u32,
            lkey: buf.lkey(),
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut work = self.work_pool.get()?;
        work.bufs.push(buf);
        work.sender = Some(tx);

        let wr_id = work.release() as _;
        let mut wr = ibv_send_wr {
            wr_id,
            next: std::ptr::null_mut(),
            sg_list: &mut sge,
            num_sge: 1,
            opcode: ibv_wr_opcode::IBV_WR_SEND,
            send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
            ..Default::default()
        };
        let mut bad_wr = std::ptr::null_mut();
        let ret = unsafe { ibv_post_send(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) };
        if ret == 0 {
            Ok(rx)
        } else {
            let _ = WorkRef::new(&self.work_pool, wr_id as _);
            Err(Error::with_errno(ErrorKind::IBPostSendFail))
        }
    }

    pub fn set_to_error(&self) -> Result<()> {
        self.queue_pair.set_to_error()
    }

    pub fn prepare_close(&self) -> Result<tokio::sync::oneshot::Receiver<Result<u32>>> {
        self.wake_up(WorkType::PrepareClose)
    }

    pub(super) fn wake_up(
        &self,
        ty: WorkType,
    ) -> Result<tokio::sync::oneshot::Receiver<Result<u32>>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut work = self.work_pool.get()?;
        work.ty = ty;
        work.sender = Some(tx);
        let wr_id = work.release() as _;
        let mut wr = ibv_send_wr {
            wr_id,
            next: std::ptr::null_mut(),
            sg_list: std::ptr::null_mut(),
            num_sge: 0,
            opcode: ibv_wr_opcode::IBV_WR_SEND,
            send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
            ..Default::default()
        };
        let mut bad_wr = std::ptr::null_mut();
        let ret = unsafe { ibv_post_send(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) };
        if ret == 0 {
            Ok(rx)
        } else {
            let _ = WorkRef::new(&self.work_pool, wr_id as _);
            Err(Error::with_errno(ErrorKind::IBPostSendFail))
        }
    }

    pub fn recv(&self, buf: BufferSlice) -> Result<tokio::sync::oneshot::Receiver<Result<u32>>> {
        let slice = buf.as_ref();
        let mut sge = ibv_sge {
            addr: slice.as_ptr() as u64,
            length: slice.len() as u32,
            lkey: buf.lkey(),
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut work = self.work_pool.get()?;
        work.bufs.push(buf);
        work.sender = Some(tx);

        let wr_id = work.release() as _;
        let mut wr = ibv_recv_wr {
            wr_id,
            next: std::ptr::null_mut(),
            sg_list: &mut sge,
            num_sge: 1,
        };
        let mut bad_wr = std::ptr::null_mut();
        let ret = unsafe { ibv_post_recv(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) };
        if ret == 0 {
            Ok(rx)
        } else {
            let _ = WorkRef::new(&self.work_pool, wr_id as _);
            Err(Error::with_errno(ErrorKind::IBPostRecvFail))
        }
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

        let mut send_memory = manager.allocate_buffer().unwrap();
        println!("send memory: {:#?}", send_memory);
        send_memory.as_mut().fill(0x23);
        let mut recv_memory = manager.allocate_buffer().unwrap();
        println!("recv memory: {:#?}", recv_memory);
        recv_memory.as_mut().fill(0);
        assert_ne!(send_memory.as_ref(), recv_memory.as_ref());

        let recv_rx = recv_socket.recv(recv_memory).unwrap();
        let send_rx = send_socket.send(send_memory).unwrap();

        let (send_result, recv_result) = tokio::join!(send_rx, recv_rx);
        println!("send result: {}", send_result.unwrap().unwrap());
        println!("recv result: {}", recv_result.unwrap().unwrap());

        let recv_memory = manager.allocate_buffer().unwrap();
        let recv_rx = recv_socket.recv(recv_memory).unwrap();
        let clone = recv_socket.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            clone.set_to_error()
        });
        let recv_result = recv_rx.await.unwrap();
        assert!(recv_result.is_err());

        let invalid_socket = manager.create_socket().unwrap();
        let memory = manager.allocate_buffer().unwrap();
        invalid_socket.send(memory).unwrap_err();
    }
}
