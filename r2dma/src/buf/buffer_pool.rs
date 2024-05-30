use std::sync::{Arc, Mutex};

use crate::*;

pub struct BufferPool {
    bufs: Vec<Buffer>,
    pool: Mutex<Vec<(u32, u32)>>,
    size: usize,
    count: usize,
}

impl BufferPool {
    pub fn new(cards: &Arc<Vec<Arc<Card>>>, size: usize, count: usize) -> Result<Arc<Self>> {
        let size = size.next_power_of_two();
        let count = count.next_power_of_two();

        let mem_count = (size * count).next_multiple_of(1 << 32) / (1 << 32);
        let mem_size = std::cmp::min(size * count, 1 << 32);
        let sub_count = mem_size / size;

        let mut mems = vec![];
        let mut pool = vec![];
        for i in 0..mem_count {
            let mem = Buffer::new(cards, mem_size)?;
            for j in 0..sub_count {
                pool.push((i as u32, j as u32));
            }
            mems.push(mem);
        }

        Ok(Arc::new(Self {
            bufs: mems,
            pool: Mutex::new(pool),
            size,
            count,
        }))
    }

    pub fn get(self: &Arc<Self>) -> Result<BufferSlice> {
        let mut pool = self.pool.lock().unwrap();
        match pool.pop() {
            Some((i, j)) => Ok(BufferSlice::new(self, i, j)),
            None => Err(Error::new(ErrorKind::IBAllocPDFail)),
        }
    }

    fn put(&self, i: u32, j: u32) {
        let mut pool = self.pool.lock().unwrap();
        pool.push((i, j))
    }
}

impl std::fmt::Debug for BufferPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.pool.lock().unwrap().len();
        f.debug_struct("BufferPool")
            .field("size", &self.size)
            .field("count", &self.count)
            .field("remain", &count)
            .finish()
    }
}

#[derive(Debug)]
pub struct BufferSlice {
    pool: Arc<BufferPool>,
    i: u32,
    j: u32,
}

impl BufferSlice {
    fn new(mem: &Arc<BufferPool>, i: u32, j: u32) -> Self {
        Self {
            pool: mem.clone(),
            i,
            j,
        }
    }

    pub fn lkey(&self) -> u32 {
        self.pool.bufs[self.i as usize].lkey
    }

    pub fn rkey(&self) -> u32 {
        self.pool.bufs[self.i as usize].rkey
    }
}

impl std::convert::AsRef<[u8]> for BufferSlice {
    fn as_ref(&self) -> &[u8] {
        let memory_region = &*self.pool.bufs[self.i as usize];
        let size = self.pool.size;
        let base = self.j as usize * size;
        unsafe { std::slice::from_raw_parts(memory_region.addr.byte_add(base) as _, size) }
    }
}

impl std::convert::AsMut<[u8]> for BufferSlice {
    fn as_mut(&mut self) -> &mut [u8] {
        let memory_region = &*self.pool.bufs[self.i as usize];
        let size = self.pool.size;
        let base = self.j as usize * size;
        unsafe { std::slice::from_raw_parts_mut(memory_region.addr.byte_add(base) as _, size) }
    }
}

impl Drop for BufferSlice {
    fn drop(&mut self) {
        self.pool.put(self.i, self.j);
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_memory_pool() {
        let n = 128;

        let cards = Cards::open().unwrap();
        let pool = BufferPool::new(&cards.cards, 1 << 20, n).unwrap();
        println!("{:#?}", pool);

        for _ in 0..3 {
            let mut mems = vec![];
            for i in 0..n {
                let mut mem = pool.get().unwrap();
                mem.as_mut().fill(i as u8);
                mems.push(mem);
            }
            pool.get().unwrap_err();
        }
    }
}
