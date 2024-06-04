use crate::*;
use ibv::CompChannel;
use nix::sys::{epoll::*, eventfd::*};
use r2dma_sys::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug)]
pub struct Channel {
    epoll: Epoll,
    eventfd: EventFd,
    stopping: AtomicBool,
    comp_channel: ibv::CompChannel,
    pub card: Arc<Card>,
}

impl Channel {
    pub fn new(card: &Arc<Card>) -> Result<Self> {
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
        })
    }

    pub fn wake_up(&self) -> Result<()> {
        self.eventfd.write(1)?;
        Ok(())
    }

    pub fn on_wake_up(&self) {
        let _ = self.eventfd.read();
    }

    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Acquire)
    }

    pub fn set_stop(&self) {
        self.stopping.store(true, Ordering::Release);
    }

    pub fn epoll_wait<'a>(&self, events: &'a mut [EpollEvent]) -> Result<&'a [EpollEvent]> {
        match self.epoll.wait(events, EpollTimeout::NONE) {
            Ok(n) => Ok(&events[..n]),
            Err(e) => Err(e.into()),
        }
    }
}

impl std::ops::Deref for Channel {
    type Target = CompChannel;

    fn deref(&self) -> &Self::Target {
        &self.comp_channel
    }
}
