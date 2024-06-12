use crate::*;
use std::{
    ops::Deref,
    sync::{mpsc, Arc},
};

#[derive(Debug)]
pub struct Manager {
    threads: Vec<std::thread::JoinHandle<()>>,
    pub channels: Vec<Arc<Channel>>,
    buffer_pool: Arc<BufferPool>,
    pub cards: Arc<Cards>,
    work_pool: Arc<WorkPool>,
    config: Config,
}

impl Manager {
    pub fn init(config: &Config) -> Result<Self> {
        let cards = Cards::open()?;
        let buffer_pool = BufferPool::new(&cards, config.buffer_size, config.buffer_count)?;
        let work_pool = Arc::new(WorkPool::new(config.work_pool_size));

        let mut channels = vec![];
        let mut threads = vec![];
        for card in cards.as_ref().deref() {
            let (sender, receiver) = mpsc::channel();

            let channel = Arc::new(Channel::new(card, sender)?);
            channels.push(channel.clone());

            let mut event_loop = EventLoop::new(&buffer_pool, &work_pool);

            threads.push(std::thread::spawn(move || {
                event_loop.run(channel, receiver);
            }))
        }

        Ok(Self {
            threads,
            channels,
            buffer_pool,
            cards,
            work_pool,
            config: *config,
        })
    }

    pub fn allocate_buffer(&self) -> Result<BufferSlice> {
        self.buffer_pool.get()
    }

    pub fn allocate_work(&self) -> Result<Box<Work>> {
        self.work_pool.get()
    }

    pub fn create_socket(&self) -> Result<Arc<Socket>> {
        let channel = self
            .channels
            .first()
            .ok_or(Error::new(ErrorKind::IBDeviceNotFound))?;
        let socket = channel.create_socket(&self.config)?;

        channel
            .sender
            .send(Task::AddSocket(socket.clone()))
            .map_err(|e| Error::with_msg(ErrorKind::ChannelSendFail, e.to_string()))?;

        let block = || -> Result<()> {
            channel.wake_up()?;
            for _ in 0..18 {
                let mut work = self.work_pool.get()?;
                work.ty = WorkType::RECV;
                work.buf = Some(self.buffer_pool.get()?);
                socket.submit_recv(work)?;
            }
            Ok(())
        };
        if block().is_err() {
            socket.set_to_error();
        }

        Ok(socket)
    }

    pub fn stop_and_join(&mut self) -> Result<()> {
        // TODO(SF): release receiving buffers.
        for channel in &self.channels {
            channel.stop()?;
        }

        for thread in self.threads.drain(..) {
            thread
                .join()
                .map_err(|e| Error::with_msg(ErrorKind::IOError, format!("{:?}", e)))?;
        }

        Ok(())
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        match self.stop_and_join() {
            Ok(_) => (),
            Err(err) => tracing::error!("stop manager failed: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_memory_pool() {
        let config = Config::default();
        let manager = Manager::init(&config).unwrap();
        println!("{:#?}", manager);

        let pool = &manager.buffer_pool;
        for _ in 0..3 {
            let mut mems = vec![];
            for i in 0..config.buffer_count {
                let mut mem = pool.get().unwrap();
                mem.as_mut().fill(i as u8);
                mem.rkey();
                mems.push(mem);
            }
            pool.get().unwrap_err();
        }

        for _ in 0..1024 {
            let _ = manager.allocate_work().unwrap();
        }
    }
}
