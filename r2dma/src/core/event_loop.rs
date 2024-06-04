use nix::sys::epoll::EpollEvent;

use crate::*;

use std::sync::Arc;
use std::sync::{mpsc, Mutex};

#[derive(Debug, Default)]
pub struct Task;

#[derive(Debug)]
pub struct EventLoop {
    sockets: Mutex<Vec<Arc<Socket>>>,
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
        let channel = Arc::new(Channel::new(card)?);

        Ok(Arc::new(Self {
            sockets: Default::default(),
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

    pub fn stop(&self) -> Result<()> {
        tracing::info!("event_loop is stopping...");
        self.channel.set_stop();
        self.channel.wake_up()?;
        Ok(())
    }

    pub fn run(&self, receiver: mpsc::Receiver<Task>) {
        let mut events = vec![EpollEvent::empty(); 1024];
        while !self.channel.is_stopping() {
            match self.channel.epoll_wait(&mut events) {
                Ok(events) => {
                    for event in events {
                        match event.data() {
                            0 => {
                                // just wake up!
                                self.channel.on_wake_up();
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
