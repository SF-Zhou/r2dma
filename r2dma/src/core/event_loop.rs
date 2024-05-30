use crate::*;
use ibv::CompChannel;
use nix::sys::{epoll::*, eventfd::*};
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
}

impl EventLoop {
    pub fn new(comp_channel: CompChannel) -> Result<Arc<Self>> {
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
        }))
    }

    pub fn wake_up(&self) -> Result<()> {
        self.eventfd.write(1)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        tracing::warn!("event_loop is stopping...");
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
        tracing::warn!("event_loop is stopped.");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_loop_normal() {
        let cards = Cards::open().unwrap();
        let card = cards.get(None).unwrap();
        let event_loop = &card.event_loop;

        println!("{:#?}", event_loop);
        let (sender, receiver) = mpsc::sync_channel(1024);

        let clone = event_loop.clone();
        let thread = std::thread::spawn(move || {
            clone.run(receiver);
        });

        std::thread::sleep(std::time::Duration::from_millis(10));
        sender.send(Task).unwrap();
        event_loop.wake_up().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        event_loop.stop().unwrap();
        thread.join().unwrap();
    }
}
