use crate::*;
use ibv::WorkCompletion;
use nix::sys::epoll::EpollEvent;
use r2dma_sys::*;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::VecDeque;

use std::pin::Pin;
use std::sync::mpsc;
use std::sync::Arc;

#[derive(Debug)]
pub struct WaitingWork {
    pub index: u64,
    pub work_id: WorkID,
}

#[derive(Debug)]
pub enum Task {
    AddSocket(Arc<Socket>),
    AsyncSendWork { qp_num: u32, work: WaitingWork },
}

const N: usize = 64;

#[derive(Debug)]
struct SocketWrapper {
    socket: Arc<Socket>,
    qp_num: u32,
    waiting_send_indexs: BTreeSet<u64>,
    waiting_send_works: VecDeque<WorkID>,
    waiting_remote_notification: Option<u32>,

    remote_completion: u32,
    remote_notification: u32,

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

    pub fn on_send_data(&self, wc: &WorkCompletion, work_id: WorkID) -> Result<()> {
        tracing::info!("on send: {wc:#?}, work: {work_id:?}");

        wc.result()?;
        Ok(())
    }

    pub fn on_send_imm(&self, wc: &WorkCompletion, imm: u32) -> Result<()> {
        tracing::info!("on send: {wc:#?}, work: {imm:?}");

        wc.result()?;
        Ok(())
    }

    pub fn on_recv_data(&mut self, wc: &WorkCompletion, work_id: WorkID) -> Result<()> {
        if let Some(ack) = wc.imm() {
            self.remote_completion += ack;
        }

        self.socket.submit_work_id(work_id)?; // re-submit to receive again.
        self.remote_notification += 1;

        Ok(())
    }
}

impl Drop for SocketWrapper {
    fn drop(&mut self) {
        unsafe {
            ibv_ack_cq_events(self.socket.comp_queue.as_mut_ptr(), self.unack_events_count);
        }
    }
}

#[derive(Debug)]
pub struct EventLoop {
    pub buffer_pool: Arc<BufferPool>,
    pub work_pool: Arc<WorkPool>,
    completions: [WorkCompletion; N],
}

type SocketsMap = HashMap<u32, Pin<Box<SocketWrapper>>>;

impl EventLoop {
    pub fn new(buffer_pool: &Arc<BufferPool>, work_pool: &Arc<WorkPool>) -> Self {
        Self {
            buffer_pool: buffer_pool.clone(),
            work_pool: work_pool.clone(),
            completions: [(); N].map(|_| WorkCompletion::default()),
        }
    }

    pub fn run(&mut self, channel: Arc<Channel>, receiver: mpsc::Receiver<Task>) {
        let mut sockets = SocketsMap::default();
        let mut events = [(); N].map(|_| EpollEvent::empty());
        while !channel.is_stopping() {
            match channel.poll_events(&mut events) {
                Ok(events) => {
                    for event in events {
                        match event.data() {
                            0 => channel.on_wake_up(),
                            _ => self.handle_cq_events(&channel, &mut sockets),
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
                    Task::AddSocket(socket) => self.add_socket(socket, &mut sockets),
                    Task::AsyncSendWork {
                        qp_num,
                        work: WaitingWork { index, work_id },
                    } => {
                        if let Some(wrapper) = sockets.get_mut(&qp_num) {
                            if wrapper.socket.state.check_send_index(index) {
                                match wrapper.socket.submit_work_id(work_id) {
                                    Ok(_) => (),
                                    Err(_) => {
                                        let _ = wrapper.socket.set_to_error();
                                    }
                                }
                            } else {
                                wrapper.waiting_send_indexs.insert(index);
                                wrapper.waiting_send_works.push_back(work_id);
                            }
                        }
                    }
                }
            }
        }
        tracing::info!("event_loop is stopped.");
    }

    fn handle_cq_events(&mut self, channel: &Channel, sockets: &mut SocketsMap) {
        loop {
            match channel.poll_socket() {
                Ok(ptr) if ptr.is_null() => break,
                Ok(ptr) => {
                    let wrapper = unsafe { &mut *(ptr as *mut SocketWrapper) };
                    wrapper.unack_events_count += 1;
                    if self.handle_work_completion(wrapper) {
                        sockets.remove(&wrapper.qp_num);
                    }
                }
                Err(err) => {
                    tracing::error!("comp channel poll: {:?}", err);
                    break;
                }
            }
        }
    }

    fn add_socket(&mut self, socket: Arc<Socket>, sockets: &mut SocketsMap) {
        let qp_num = socket.queue_pair.qp_num;
        let mut wrapper = Box::new(SocketWrapper {
            socket,
            qp_num,
            waiting_send_indexs: Default::default(),
            waiting_send_works: Default::default(),
            waiting_remote_notification: None,
            remote_completion: Default::default(),
            remote_notification: Default::default(),
            unack_events_count: Default::default(),
        });

        let comp_queue = unsafe { &mut *wrapper.socket.comp_queue.as_mut_ptr() };
        comp_queue.cq_context = wrapper.as_mut() as *mut _ as _;

        // enable notification and start to handle events.
        if !self.handle_work_completion(&mut wrapper) {
            sockets.insert(qp_num, Box::into_pin(wrapper));
        }
    }

    fn handle_work_completion(&mut self, wrapper: &mut SocketWrapper) -> bool {
        wrapper.notify();

        loop {
            match wrapper.poll_completions(&mut self.completions) {
                Ok(wcs) => {
                    let mut is_error = false;
                    let mut send_local_complete = 0;
                    let mut read_complete = 0;

                    for wc in wcs {
                        let work_id = WorkID::from(wc.wr_id);
                        let result = match &work_id {
                            WorkID::Empty => continue,
                            WorkID::Box(work) => {
                                match work.ty {
                                    WorkType::SEND => {
                                        send_local_complete += 1;
                                        wrapper.on_send_data(wc, work_id)
                                    }
                                    WorkType::RECV => wrapper.on_recv_data(wc, work_id),
                                    WorkType::READ => {
                                        // TODO(SF): handle read result.
                                        read_complete += 1;
                                        Ok(())
                                    }
                                }
                            }
                            WorkID::Imm(imm) => {
                                send_local_complete += 1;
                                wrapper.on_send_imm(wc, *imm)
                            }
                        };
                        is_error |= result.is_err();
                    }

                    if send_local_complete > 0 {
                        wrapper
                            .socket
                            .state
                            .send_local_complete(send_local_complete);
                    }

                    if read_complete > 0 {
                        wrapper.socket.state.read_complete(read_complete);
                    }

                    if is_error {
                        let _ = wrapper.socket.set_to_error();
                    }

                    if wcs.len() < self.completions.len() {
                        break;
                    }
                }
                Err(err) => tracing::error!("poll comp_queue failed: {:?}", err),
            }
        }

        // try to send notification.
        if wrapper.waiting_remote_notification.is_some() && wrapper.remote_notification > 0 {
            *wrapper.waiting_remote_notification.as_mut().unwrap() += wrapper.remote_notification;
            wrapper.remote_notification = 0;
        } else if wrapper.remote_notification >= wrapper.socket.notification_batch {
            let n = wrapper.remote_notification;
            wrapper.remote_notification = 0;
            let state = &wrapper.socket.state;
            if let Some(index) = state.apply_send() {
                if state.check_notify_index(index) {
                    match wrapper.socket.submit_work_id(WorkID::Imm(n)) {
                        Ok(_) => (),
                        Err(_) => {
                            let _ = wrapper.socket.set_to_error();
                        }
                    }
                } else {
                    wrapper.waiting_send_indexs.insert(index);
                    wrapper.waiting_remote_notification = Some(n);
                }
            };
        }

        // try to waiting sends.
        if wrapper.remote_completion > 0 {
            let current_bound = wrapper
                .socket
                .state
                .send_remote_complete(wrapper.remote_completion as u64);
            wrapper.remote_completion = 0;
            let mut split = wrapper.waiting_send_indexs.split_off(&current_bound); // [start, bound) + [bound, end)
            std::mem::swap(&mut split, &mut wrapper.waiting_send_indexs);

            let mut count = split.len();
            if count > 0 && wrapper.waiting_remote_notification.is_some() {
                count -= 1;
                let n = wrapper.waiting_remote_notification.take().unwrap();
                match wrapper.socket.submit_work_id(WorkID::Imm(n)) {
                    Ok(_) => (),
                    Err(_) => {
                        let _ = wrapper.socket.set_to_error();
                    }
                }
            }

            for _ in 0..count {
                let work_id = wrapper.waiting_send_works.pop_front().unwrap();
                match wrapper.socket.submit_work_id(work_id) {
                    Ok(_) => (),
                    Err(_) => {
                        let _ = wrapper.socket.set_to_error();
                    }
                }
            }
        }

        false
    }
}
