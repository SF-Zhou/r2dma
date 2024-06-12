use crate::ibv::verbs::*;
use crate::*;
use std::sync::Arc;

pub struct Socket {
    pub(super) queue_pair: ibv::QueuePair,
    pub(super) comp_queue: ibv::CompQueue,
    pub(super) channel: Arc<Channel>,
    pub(super) state: State,
    pub(super) notification_batch: u32,
}

impl Socket {
    pub fn new(
        queue_pair: ibv::QueuePair,
        comp_queue: ibv::CompQueue,
        channel: Arc<Channel>,
    ) -> Arc<Self> {
        let ret = Self {
            queue_pair,
            comp_queue,
            channel,
            state: State::new(16, 16),
            notification_batch: 8,
        };

        Arc::new(ret)
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

    pub(super) fn submit_work_id(&self, work_id: WorkID) -> Result<()> {
        // 1. create sge.
        let mut sge = work_id.sge();
        let (sg_list, num_sge) = match sge.addr {
            0 => (std::ptr::null_mut(), 0),
            _ => (&mut sge as _, 1),
        };

        // 2. create work request and post.
        let ret = match &work_id {
            WorkID::Empty => {
                0 // do nothing.
            }
            WorkID::Box(b) => match b.ty {
                WorkType::SEND => {
                    let mut wr = ibv_send_wr {
                        wr_id: u64::from(&work_id),
                        sg_list,
                        num_sge,
                        opcode: ibv_wr_opcode::IBV_WR_SEND,
                        send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
                        ..Default::default()
                    };
                    let mut bad_wr = std::ptr::null_mut();
                    unsafe { ibv_post_send(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
                }
                WorkType::RECV => {
                    let mut wr = ibv_recv_wr {
                        wr_id: u64::from(&work_id),
                        next: std::ptr::null_mut(),
                        sg_list,
                        num_sge,
                    };
                    let mut bad_wr = std::ptr::null_mut();
                    unsafe { ibv_post_recv(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
                }
                WorkType::READ => {
                    // TODO(SF): handle read job.
                    todo!()
                }
            },
            WorkID::Imm(imm) => {
                let mut wr = ibv_send_wr {
                    wr_id: u64::from(&work_id),
                    opcode: ibv_wr_opcode::IBV_WR_SEND_WITH_IMM,
                    send_flags: ibv_send_flags::IBV_SEND_SIGNALED.0,
                    __bindgen_anon_1: ibv_send_wr__bindgen_ty_1 {
                        imm_data: imm.to_be(),
                    },
                    ..Default::default()
                };
                let mut bad_wr = std::ptr::null_mut();
                unsafe { ibv_post_send(self.queue_pair.as_mut_ptr(), &mut wr, &mut bad_wr) }
            }
        };
        if ret == 0 {
            std::mem::forget(work_id);
            Ok(())
        } else {
            match work_id {
                WorkID::Empty => (),
                WorkID::Imm(_) => {
                    self.state.send_fail();
                }
                WorkID::Box(b) => match b.ty {
                    WorkType::SEND => self.state.send_fail(),
                    WorkType::RECV => self.state.recv_complete(),
                    WorkType::READ => self.state.read_fail(),
                },
            }
            self.set_to_error();
            Err(Error::with_errno(ErrorKind::IBPostSendFail))
        }
    }

    pub fn submit_work<W: Into<WorkID>>(&self, work: W) -> Result<()> {
        let work_id: WorkID = work.into();
        match self.state.apply_send() {
            Some(index) => {
                if self.state.check_send_index(index) {
                    self.submit_work_id(work_id)
                } else {
                    // the socket remains valid until the task is completed.
                    self.send_task(Task::AsyncSendWork {
                        qp_num: self.queue_pair.qp_num,
                        work: WaitingWork { index, work_id },
                    })
                }
            }
            None => Err(Error::new(ErrorKind::IBSocketError)),
        }
    }

    pub(super) fn submit_recv<W: Into<WorkID>>(&self, work: W) -> Result<()> {
        self.state.apply_recv();
        self.submit_work_id(work.into())
    }

    pub fn set_to_error(&self) {
        self.state.set_error();
        self.queue_pair.set_to_error();
        let _ = self.send_task(Task::WakeUpSocket {
            qp_num: self.queue_pair.qp_num,
        });
    }

    fn send_task(&self, task: Task) -> Result<()> {
        self.channel.sender.send(task).unwrap();
        self.channel.wake_up()
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

    #[test]
    fn test_ib_socket() {
        let config = Config {
            buffer_size: 4096,
            buffer_count: 5 * 4096,
            ..Default::default()
        };
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
        send_socket.submit_work(work).unwrap();

        // 2. send a imm.
        let work = manager.allocate_work().unwrap();
        send_socket.submit_work(work).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 3. multi-send.
        let start = std::time::Instant::now();
        let n = 20000;
        for _ in 0..n {
            let buf = manager.allocate_buffer().unwrap();
            let mut work = manager.allocate_work().unwrap();
            work.buf = Some(buf);
            send_socket.submit_work(work).unwrap();
        }
        for _ in 0..20 {
            if send_socket.state.check_send_index(n) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        println!(
            "elpased: {}us",
            std::time::Instant::now().duration_since(start).as_micros()
        );

        // 4. set to error.
        send_socket.set_to_error();
        drop(send_socket);
        std::thread::sleep(std::time::Duration::from_millis(100));
        println!("finished!");
    }
}
