use std::sync::{mpsc, Arc};

use crate::*;

#[derive(Debug)]
pub struct Cards {
    pub event_loops: Vec<Arc<EventLoop>>,
    pub cards: Arc<Vec<Arc<Card>>>,
    pub threads: Vec<std::thread::JoinHandle<()>>,
}

impl Cards {
    pub fn open() -> Result<Self> {
        let mut cards = vec![];

        let device_list = ibv::DeviceList::cached();
        for device in device_list.as_ref() {
            let card = Card::open(device)?;
            cards.push(card);
        }

        let mut event_loops = vec![];
        for card in &cards {
            event_loops.push(EventLoop::new(card)?);
        }

        let mut threads = vec![];
        for event_loop in &event_loops {
            let event_loop = event_loop.clone();
            threads.push(std::thread::spawn(move || {
                let (_, receiver) = mpsc::sync_channel(1024);
                event_loop.run(receiver);
            }))
        }

        let cards = Arc::new(cards);

        Ok(Self {
            cards,
            event_loops,
            threads,
        })
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

impl Drop for Cards {
    fn drop(&mut self) {
        match self.stop_and_join() {
            Ok(_) => (),
            Err(err) => tracing::error!("cards stop error: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cards() {
        let cards = Cards::open().unwrap();
        println!("{:#?}", cards);
    }
}
