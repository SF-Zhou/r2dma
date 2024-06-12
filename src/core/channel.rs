use crate::ibv::verbs::*;
use crate::*;
use ibv::CompChannel;
use nix::sys::{epoll::*, eventfd::*};
use std::{
    os::raw::c_void,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};

#[derive(Debug)]
pub struct Channel {
    epoll: Epoll,
    eventfd: EventFd,
    stopping: AtomicBool,
    comp_channel: ibv::CompChannel,
    pub(super) card: Arc<Card>,
    pub(super) sender: mpsc::Sender<Task>,
}

impl Channel {
    pub fn new(card: &Arc<Card>, sender: mpsc::Sender<Task>) -> Result<Self> {
        let epoll = Epoll::new(EpollCreateFlags::empty())?;
        let eventfd = EventFd::from_flags(EfdFlags::EFD_NONBLOCK)?;

        epoll.add(
            &eventfd,
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLET, 0),
        )?;

        let comp_channel = ibv::CompChannel::new(unsafe {
            let channel = ibv_create_comp_channel(card.context.as_mut_ptr());
            if channel.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCompChannelFail));
            }
            channel
        });
        comp_channel.set_nonblock()?;

        epoll.add(
            comp_channel.fd(),
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLET, 1),
        )?;

        Ok(Self {
            epoll,
            eventfd,
            stopping: Default::default(),
            comp_channel,
            card: card.clone(),
            sender,
        })
    }

    pub fn create_socket(self: &Arc<Self>, config: &Config) -> Result<Arc<Socket>> {
        let card = &self.card;
        let comp_queue = ibv::CompQueue::new(unsafe {
            let comp_queue = ibv_create_cq(
                card.context.as_mut_ptr(),
                config.max_cqe as _,
                std::ptr::null_mut(),
                self.as_mut_ptr(),
                0,
            );
            if comp_queue.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCQFail));
            }
            comp_queue
        });

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

        Ok(Socket::new(queue_pair, comp_queue, self.clone()))
    }

    pub fn wake_up(&self) -> Result<()> {
        self.eventfd.write(1)?;
        Ok(())
    }

    #[inline(always)]
    pub(super) fn on_wake_up(&self) {
        let _ = self.eventfd.read();
    }

    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Acquire)
    }

    pub fn stop(&self) -> Result<()> {
        tracing::info!("event_loop is stopping...");
        self.stopping.store(true, Ordering::Release);
        self.wake_up()?;
        Ok(())
    }

    pub fn poll_events<'a>(&self, events: &'a mut [EpollEvent]) -> Result<&'a [EpollEvent]> {
        match self.epoll.wait(events, EpollTimeout::NONE) {
            Ok(n) => Ok(&events[..n]),
            Err(e) => Err(e.into()),
        }
    }

    pub fn poll_socket(&self) -> Result<*mut c_void> {
        self.comp_channel.get_cq_event()
    }
}

impl std::ops::Deref for Channel {
    type Target = CompChannel;

    fn deref(&self) -> &Self::Target {
        &self.comp_channel
    }
}
