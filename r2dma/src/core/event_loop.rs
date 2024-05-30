use crate::*;
use ibv::CompChannel;
use nix::sys::{epoll::*, eventfd::*};
use r2dma_sys::*;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Default)]
pub struct Task;

#[derive(Debug)]
pub struct EventLoop {
    epoll: Epoll,
    eventfd: EventFd,
    pub comp_channel: CompChannel,
    stopping: AtomicBool,
    pub card: Arc<Card>,
}

impl EventLoop {
    pub fn new(card: &Arc<Card>) -> Result<Arc<Self>> {
        let comp_channel = CompChannel::new(unsafe {
            let channel = ibv_create_comp_channel(card.context.as_mut_ptr());
            if channel.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCompChannelFail));
            }
            channel
        });
        comp_channel.set_nonblock()?;

        let epoll = Epoll::new(EpollCreateFlags::empty())?;
        let eventfd = EventFd::from_flags(EfdFlags::EFD_NONBLOCK)?;

        epoll.add(
            &eventfd,
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLET, 0),
        )?;

        epoll.add(
            comp_channel.fd(),
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLET, 1),
        )?;

        Ok(Arc::new(Self {
            epoll,
            eventfd,
            comp_channel,
            stopping: Default::default(),
            card: card.clone(),
        }))
    }

    pub fn wake_up(&self) -> Result<()> {
        self.eventfd.write(1)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        tracing::info!("event_loop is stopping...");
        self.stopping.store(true, Ordering::Release);
        self.wake_up()?;
        Ok(())
    }

    pub fn run(&self, receiver: mpsc::Receiver<Task>) {
        let mut events = vec![EpollEvent::empty(); 1024];
        while !self.stopping.load(Ordering::Acquire) {
            match self.epoll.wait(&mut events, EpollTimeout::NONE) {
                Ok(n) => {
                    for event in &events[..n] {
                        match event.data() {
                            0 => {
                                // just wake up!
                                let _ = self.eventfd.read();
                            }
                            _ => {
                                self.handle_work_completion();
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("epoll wait failed: {:?}", err);
                    continue;
                }
            }

            while let Ok(_task) = receiver.try_recv() {
                // TODO(SF): handle user events.
            }
        }
        tracing::info!("event_loop is stopped.");
    }

    fn handle_work_completion(&self) {
        loop {
            match self.comp_channel.poll() {
                Ok(None) => break,
                Ok(Some(socket)) => {
                    socket.notify().unwrap();
                    socket.poll_cq();
                    let _ = Arc::into_raw(socket);
                }
                Err(err) => {
                    tracing::error!("comp channel poll: {:?}", err);
                    break;
                }
            }
        }
    }
}
