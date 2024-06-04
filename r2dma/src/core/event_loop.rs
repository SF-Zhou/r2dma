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
    sockets: Mutex<Vec<Arc<Socket>>>,
    epoll: Epoll,
    eventfd: EventFd,
    stopping: AtomicBool,

    pub channel: Arc<Channel>,
    pub card: Arc<Card>,
    pub buffer_pool: Arc<BufferPool>,
    pub work_pool: Arc<WorkPool>,
}

impl EventLoop {
    pub fn new(
        card: &Arc<Card>,
        buffer_pool: &Arc<BufferPool>,
        work_pool: &Arc<WorkPool>,
    ) -> Result<Arc<Self>> {
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
            stopping: Default::default(),

            channel,
            card: card.clone(),
            buffer_pool: buffer_pool.clone(),
            work_pool: work_pool.clone(),
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
                                self.handle_channel_event();
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

    fn handle_channel_event(&self) {
        loop {
            match self.channel.poll() {
                Ok(None) => break,
                Ok(Some(socket)) => self.handle_work_completion(socket),
                Err(err) => {
                    tracing::error!("comp channel poll: {:?}", err);
                    break;
                }
            }
        }
    }

    fn handle_work_completion(&self, socket: &Socket) {
        socket.notify().unwrap();

        let mut wcs = [0u8; 16].map(|_| ibv::WorkCompletion::default());
        match socket.poll(&mut wcs) {
            Ok(wcs) => {
                for wc in wcs {
                    let mut work = WorkRef::new(&self.work_pool, wc.wr_id as _);

                    match work.ty {
                        WorkType::Send => socket.on_send(wc),
                        WorkType::Recv => socket.on_recv(wc, &mut work),
                    }
                    work.bufs.clear();

                    if let Some(sender) = work.sender.take() {
                        let _ = sender.send(
                            wc.result()
                                .map_err(|_| Error::new(ErrorKind::WorkCompletionFail)),
                        );
                    }
                }

                let need_remove = socket.state.work_complete(wcs.len() as u64);
                if need_remove {
                    // TODO(SF): remove this socket.
                }
            }
            Err(err) => tracing::error!("poll comp_queue failed: {:?}", err),
        }
    }
}
