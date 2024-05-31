use crate::*;
use std::{
    ops::Deref,
    sync::{mpsc, Arc},
};

#[derive(Debug)]
pub struct Manager {
    threads: Vec<std::thread::JoinHandle<()>>,
    pub event_loops: Vec<Arc<EventLoop>>,
    buffer_pool: Arc<BufferPool>,
    pub cards: Arc<Cards>,
}

impl Manager {
    pub fn init(config: &Config) -> Result<Self> {
        let cards = Cards::open()?;
        let buffer_pool = BufferPool::new(&cards, config.buffer_size, config.buffer_count)?;

        let mut threads = vec![];
        let mut event_loops = vec![];
        for card in cards.as_ref().deref() {
            let event_loop = EventLoop::new(card, &buffer_pool)?;
            event_loops.push(event_loop.clone());

            threads.push(std::thread::spawn(move || {
                let (_, receiver) = mpsc::sync_channel(1024);
                event_loop.run(receiver);
            }))
        }

        Ok(Self {
            threads,
            event_loops,
            buffer_pool,
            cards,
        })
    }

    pub fn allocate_buffer(&self) -> Result<BufferSlice> {
        self.buffer_pool.get()
    }

    pub fn create_socket(&self) -> Result<Arc<Socket>> {
        let event_loop = self
            .event_loops
            .first()
            .ok_or(Error::new(ErrorKind::IBDeviceNotFound))?;
        Socket::create(event_loop)
    }

    pub fn stop_and_join(&mut self) -> Result<()> {
        for event_loop in &self.event_loops {
            event_loop.stop()?;
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
    }
}
