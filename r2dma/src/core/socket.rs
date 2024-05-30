use crate::ibv::*;
use crate::*;
use r2dma_sys::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct Socket {
    queue_pair: QueuePair,
    comp_queue: CompQueue,
    card: Arc<Card>,
    events: AtomicU32,
}

impl Socket {
    pub fn create(card: &Arc<Card>) -> Result<Arc<Self>> {
        let cqe = 32;

        let (comp_queue, cq_context) = unsafe {
            let comp_queue = ibv_create_cq(
                card.context.as_mut_ptr(),
                cqe as _,
                std::ptr::null_mut(),
                card.comp_channel.as_mut_ptr(),
                0,
            );
            if comp_queue.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCQFail));
            }
            (comp_queue, &mut (*comp_queue).cq_context)
        };
        let comp_queue = CompQueue::new(comp_queue);

        let mut attr = ibv_qp_init_attr {
            qp_context: std::ptr::null_mut(),
            send_cq: comp_queue.as_mut_ptr(),
            recv_cq: comp_queue.as_mut_ptr(),
            srq: std::ptr::null_mut(),
            cap: ibv_qp_cap {
                max_send_wr: 10,
                max_recv_wr: 10,
                max_send_sge: 5,
                max_recv_sge: 5,
                max_inline_data: 0,
            },
            qp_type: ibv_qp_type::IBV_QPT_RC,
            sq_sig_all: 0,
        };
        let mut queue_pair = QueuePair::new(unsafe {
            let queue_pair = ibv_create_qp(card.protection_domain.as_mut_ptr(), &mut attr);
            if queue_pair.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCQFail));
            }
            queue_pair
        });

        let flags = ibv_access_flags::IBV_ACCESS_LOCAL_WRITE
            | ibv_access_flags::IBV_ACCESS_REMOTE_READ
            | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE
            | ibv_access_flags::IBV_ACCESS_RELAXED_ORDERING;
        queue_pair.init(flags, 1, 0)?;

        let arc = Arc::new(Self {
            queue_pair,
            comp_queue,
            card: card.clone(),
            events: Default::default(),
        });

        *cq_context = arc.as_ref() as *const _ as _;
        arc.notify()?;
        let _ = Arc::into_raw(arc.clone());

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
            lid: self.card.port_attr.lid,
            gid: self.card.gid,
        }
    }

    pub fn ready(&self, remote: &Endpoint) -> Result<()> {
        self.queue_pair.ready_to_recv(remote)?;
        self.queue_pair.ready_to_send()
    }

    pub fn from_cq_context(cq_context: *mut std::ffi::c_void) -> Arc<Self> {
        let socket = unsafe { Arc::from_raw(cq_context as *const Socket) };
        socket.events.fetch_add(1, Ordering::Relaxed);
        socket
    }

    pub fn poll_cq(self: &Arc<Self>) {
        let mut wcs = [0u8; 16].map(|_| WorkCompletion::default());
        match self.comp_queue.poll(&mut wcs) {
            Ok(wcs) => {
                for wc in wcs {
                    let send_recv = unsafe { &mut *(wc.wr_id as *mut SendRecv) };
                    send_recv.result = Some(wc.result());
                    send_recv.waker.take().unwrap().wake();
                }
            }
            Err(err) => tracing::error!("poll comp_queue failed: {:?}", err),
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            ibv_ack_cq_events(
                self.comp_queue.as_mut_ptr(),
                self.events.load(Ordering::Acquire),
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

pub struct SendRecv<'a> {
    pub is_recv: bool,
    pub socket: Arc<Socket>,
    pub mem: &'a Buffer,
    pub waker: Option<std::task::Waker>,
    pub result: Option<std::result::Result<u32, ibv_wc_status>>,
}

impl<'a> std::future::Future for SendRecv<'a> {
    type Output = std::result::Result<u32, ibv_wc_status>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(result) = self.result.take() {
            std::task::Poll::Ready(result)
        } else {
            self.waker = Some(cx.waker().clone());

            let mut sge = ibv_sge {
                addr: self.mem.addr as u64,
                length: self.mem.length as u32,
                lkey: self.mem.lkey,
            };
            if self.is_recv {
                let mut recv_wr = ibv_recv_wr {
                    wr_id: self.as_ref().get_ref() as *const _ as _,
                    next: std::ptr::null_mut(),
                    sg_list: &mut sge,
                    num_sge: 1,
                };
                let mut bad_wr = std::ptr::null_mut();
                let ret = unsafe {
                    ibv_post_recv(
                        self.socket.queue_pair.as_mut_ptr(),
                        &mut recv_wr,
                        &mut bad_wr,
                    )
                };
                assert_eq!(ret, 0);
            } else {
                let mut send_wr = ibv_send_wr {
                    wr_id: self.as_ref().get_ref() as *const _ as _,
                    next: std::ptr::null_mut(),
                    sg_list: &mut sge,
                    num_sge: 1,
                    opcode: ibv_wr_opcode::IBV_WR_SEND,
                    send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
                    ..Default::default()
                };
                let mut bad_wr = std::ptr::null_mut();
                let ret = unsafe {
                    ibv_post_send(
                        self.socket.queue_pair.as_mut_ptr(),
                        &mut send_wr,
                        &mut bad_wr,
                    )
                };
                assert_eq!(ret, 0);
            }
            std::task::Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ib_socket() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        let cards = Cards::open().unwrap();
        let card = cards.get(None).unwrap();
        println!("{:#?}", Socket::create(&card).unwrap());

        card.start_comp_channel_consumer();

        let send_socket = Socket::create(&card).unwrap();
        println!("send socket: {:#?}", send_socket);
        let recv_socket = Socket::create(&card).unwrap();
        println!("recv socket: {:#?}", recv_socket);

        let send_endpoint = send_socket.endpoint();
        println!("send endpoint: {:#?}", send_endpoint);
        let recv_endpoint = recv_socket.endpoint();
        println!("recv endpoint: {:#?}", recv_endpoint);

        send_socket.ready(&recv_endpoint).unwrap();
        recv_socket.ready(&send_endpoint).unwrap();

        let mut send_memory = Buffer::new(&card, 1048576).unwrap();
        println!("send memory: {:#?}", send_memory);
        send_memory.as_mut().fill(0x23);
        let mut recv_memory = Buffer::new(&card, 1048576).unwrap();
        println!("recv memory: {:#?}", recv_memory);
        recv_memory.as_mut().fill(0);
        assert_ne!(send_memory.as_ref(), recv_memory.as_ref());

        let send = SendRecv {
            is_recv: false,
            socket: send_socket.clone(),
            mem: &send_memory,
            waker: None,
            result: None,
        };
        let recv = SendRecv {
            is_recv: true,
            socket: recv_socket.clone(),
            mem: &recv_memory,
            waker: None,
            result: None,
        };
        let (send_result, recv_result) = tokio::join!(recv, send);
        send_result.unwrap();
        assert_eq!(recv_result, Ok(send_memory.length as _));
    }
}
