use crate::*;

use nix::sys::{epoll::*, eventfd::*};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::sync::{mpsc, Mutex};

#[derive(Debug, Default)]
pub struct Task;

#[derive(Debug)]
pub struct EventLoop {
    sockets: Arc<Mutex<Vec<Arc<Socket>>>>,
    epoll: Epoll,
    eventfd: EventFd,
    pub channel: Arc<Channel>,
    stopping: AtomicBool,
    pub card: Arc<Card>,
    _buffer_pool: Arc<BufferPool>,
}

impl EventLoop {
    pub fn new(card: &Arc<Card>, buffer_pool: &Arc<BufferPool>) -> Result<Arc<Self>> {
        let epoll = Epoll::new(EpollCreateFlags::empty())?;
        let eventfd = EventFd::from_flags(EfdFlags::EFD_NONBLOCK)?;
        let channel = Arc::new(Channel::new(card)?);

        epoll.add(
            &eventfd,
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLET, 0),
        )?;

        epoll.add(
            channel.fd(),
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLET, 1),
        )?;

        Ok(Arc::new(Self {
            sockets: Default::default(),
            epoll,
            eventfd,
            channel,
            stopping: Default::default(),
            card: card.clone(),
            _buffer_pool: buffer_pool.clone(),
        }))
    }

    pub fn add_socket(&self, socket: Arc<Socket>) {
        let mut sockets = self.sockets.lock().unwrap();
        sockets.push(socket);
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
            match self.channel.poll() {
                Ok(None) => break,
                Ok(Some(socket)) => {
                    socket.notify().unwrap();
                    socket.poll_cq();
                }
                Err(err) => {
                    tracing::error!("comp channel poll: {:?}", err);
                    break;
                }
            }
        }
    }
}
