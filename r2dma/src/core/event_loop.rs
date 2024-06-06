use crate::*;
use ibv::WorkCompletion;
use nix::sys::epoll::EpollEvent;
use r2dma_sys::*;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::mpsc;
use std::sync::Arc;

#[derive(Debug)]
pub enum Task {
    AddSocket(Arc<Socket>),
}

const N: usize = 64;

#[derive(Debug)]
struct SocketWrapper {
    socket: Arc<Socket>,
    qp_num: u32,
    waiting: Vec<WorkRequestId>,
    unack_recv_count: u32,
    unack_events_count: u32,
}

impl SocketWrapper {
    pub fn notify(&self) {
        match unsafe { ibv_req_notify_cq(self.socket.comp_queue.as_mut_ptr(), 0) } {
            0 => (),
            _ => panic!(
                "ibv_req_notify_cq failed: {:?}",
                std::io::Error::last_os_error()
            ),
        }
    }

    fn poll_completions<'a>(
        &self,
        wc: &'a mut [ibv::WorkCompletion],
    ) -> Result<&'a [ibv::WorkCompletion]> {
        let num_entries = wc.len() as i32;
        let num = unsafe {
            ibv_poll_cq(
                self.socket.comp_queue.as_mut_ptr(),
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

    pub fn on_send_data(&self, wc: &WorkCompletion, work: WorkRef) -> Result<()> {
        tracing::info!("on send: {wc:#?}, work: {work:?}");

        wc.result()?;
        Ok(())
    }

    pub fn on_send_imm(&self, wc: &WorkCompletion, imm: u32) -> Result<()> {
        tracing::info!("on send: {wc:#?}, work: {imm:?}");

        wc.result()?;
        Ok(())
    }

    pub fn on_recv_data(&mut self, wc: &WorkCompletion, work: WorkRef) -> Result<()> {
        tracing::info!("on send: {wc:#?}, work: {work:?}");

        if let Some(ack) = wc.imm() {
            self.socket.state.recv_ack(ack);
        }

        self.socket.submit_recv_work(work)?; // re-submit to receive again.

        // ack recv.
        self.unack_recv_count += 1;
        if self.unack_recv_count >= 4 {
            let w = WorkRequestId::SendImm(self.unack_recv_count);
            self.socket.submit_send_work(w)?;
            self.unack_recv_count = 0;
        }

        Ok(())
    }
}

impl Drop for SocketWrapper {
    fn drop(&mut self) {
        unsafe {
            ibv_ack_cq_events(self.socket.comp_queue.as_mut_ptr(), self.unack_events_count);
        }
        println!("drop socket: {self:#?}");
    }
}

#[derive(Debug)]
pub struct EventLoop {
    pub buffer_pool: Arc<BufferPool>,
    pub work_pool: Arc<WorkPool>,
    sockets: HashMap<u32, Pin<Box<SocketWrapper>>>,
    completions: [WorkCompletion; N],
}

impl EventLoop {
    pub fn new(buffer_pool: &Arc<BufferPool>, work_pool: &Arc<WorkPool>) -> Self {
        Self {
            buffer_pool: buffer_pool.clone(),
            work_pool: work_pool.clone(),
            sockets: Default::default(),
            completions: [(); N].map(|_| WorkCompletion::default()),
        }
    }

    pub fn run(&mut self, channel: Arc<Channel>, receiver: mpsc::Receiver<Task>) {
        let mut events = [(); N].map(|_| EpollEvent::empty());
        while !channel.is_stopping() {
            match channel.poll_events(&mut events) {
                Ok(events) => {
                    for event in events {
                        match event.data() {
                            0 => channel.on_wake_up(),
                            _ => self.handle_cq_events(&channel),
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("epoll wait failed: {:?}", err);
                    continue;
                }
            }

            while let Ok(task) = receiver.try_recv() {
                match task {
                    Task::AddSocket(socket) => self.add_socket(socket),
                }
            }
        }
        tracing::info!("event_loop is stopped.");
    }

    fn handle_cq_events(&mut self, channel: &Channel) {
        loop {
            match channel.poll_socket() {
                Ok(ptr) if ptr.is_null() => break,
                Ok(ptr) => {
                    let wrapper = unsafe { &mut *(ptr as *mut SocketWrapper) };
                    wrapper.unack_events_count += 1;
                    if self.handle_work_completion(wrapper) {
                        self.sockets.remove(&wrapper.qp_num);
                    }
                }
                Err(err) => {
                    tracing::error!("comp channel poll: {:?}", err);
                    break;
                }
            }
        }
    }

    fn add_socket(&mut self, socket: Arc<Socket>) {
        let qp_num = socket.queue_pair.qp_num;
        let mut wrapper = Box::new(SocketWrapper {
            socket,
            qp_num,
            waiting: Default::default(),
            unack_recv_count: Default::default(),
            unack_events_count: Default::default(),
        });

        let comp_queue = unsafe { &mut *wrapper.socket.comp_queue.as_mut_ptr() };
        comp_queue.cq_context = wrapper.as_mut() as *mut _ as _;

        if !self.handle_work_completion(&mut wrapper) {
            self.sockets.insert(qp_num, Box::into_pin(wrapper));
        }
    }

    fn handle_work_completion(&mut self, wrapper: &mut SocketWrapper) -> bool {
        wrapper.notify();

        loop {
            match wrapper.poll_completions(&mut self.completions) {
                Ok(wcs) => {
                    let mut is_error = false;
                    let mut finished = wcs.len();

                    for wc in wcs {
                        let wr_id = WorkRequestId::from(wc.wr_id);

                        let result = match wr_id {
                            WorkRequestId::Empty => continue,
                            WorkRequestId::SendData(off) | WorkRequestId::AsyncSendData(off) => {
                                let work = WorkRef::new(&self.work_pool, off);
                                wrapper.on_send_data(wc, work)
                            }
                            WorkRequestId::SendImm(imm) | WorkRequestId::AsyncSendImm(imm) => {
                                wrapper.on_send_imm(wc, imm)
                            }
                            WorkRequestId::SendMsg(_, _) => {
                                let msg_wr_id = wr_id.msg();
                                if msg_wr_id != WorkRequestId::Empty {
                                    wrapper.waiting.push(msg_wr_id);
                                }
                                Ok(())
                            }
                            WorkRequestId::RecvData(off) => {
                                let work = WorkRef::new(&self.work_pool, off);
                                wrapper.on_recv_data(wc, work)
                            }
                        };
                        is_error |= result.is_err();

                        match wr_id {
                            WorkRequestId::SendData(_) | WorkRequestId::SendImm(_) => {
                                finished += 1;
                            }
                            _ => (),
                        }
                    }

                    if is_error {
                        let _ = wrapper.socket.set_to_error();
                    }

                    let need_remove = wrapper.socket.state.work_complete(finished as u64);
                    if need_remove {
                        return true;
                    }

                    if wcs.len() < self.completions.len() {
                        break;
                    }
                }
                Err(err) => tracing::error!("poll comp_queue failed: {:?}", err),
            }
        }

        false
    }
}
