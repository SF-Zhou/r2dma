use crate::*;
use std::{
    ops::Deref,
    sync::{mpsc, Arc},
};

#[derive(Debug)]
pub struct Manager {
    threads: Vec<std::thread::JoinHandle<()>>,
    senders: Vec<mpsc::Sender<Task>>,
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
        for card in cards.as_ref().deref() {
            channels.push(Arc::new(Channel::new(card)?));
        }

        let mut senders = vec![];
        let mut threads = vec![];
        for channel in &channels {
            let channel = channel.clone();
            let mut event_loop = EventLoop::new(&buffer_pool, &work_pool);

            let (sender, receiver) = mpsc::channel();
            senders.push(sender);
            threads.push(std::thread::spawn(move || {
                event_loop.run(channel, receiver);
            }))
        }

        Ok(Self {
            threads,
            senders,
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

    pub fn allocate_work(&self) -> Result<WorkRef> {
        self.work_pool.get()
    }

    pub fn create_socket(&self) -> Result<Arc<Socket>> {
        let channel = self
            .channels
            .first()
            .ok_or(Error::new(ErrorKind::IBDeviceNotFound))?;
        let socket = channel.create_socket(&self.config)?;

        self.senders[0]
            .send(Task::AddSocket(socket.clone()))
            .map_err(|e| Error::with_msg(ErrorKind::ChannelSendFail, e.to_string()))?;
        self.channels[0].wake_up()?;

        for _ in 0..4 {
            let mut work = self.work_pool.get()?;
            work.buf = Some(self.buffer_pool.get()?);
            socket.submit_recv_work(work)?;
        }

        Ok(socket)
    }

    pub fn stop_and_join(&mut self) -> Result<()> {
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
