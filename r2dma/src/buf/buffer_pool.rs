use crate::*;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

/// A pool of buffers that can be allocated and deallocated.
/// This pool is designed to manage a fixed-size buffer that can be divided into smaller blocks.
pub struct BufferPool {
    buffer: RegisteredBuffer,
    block_size: usize,
    free_list: Mutex<Vec<usize>>,
}

pub struct Buffer {
    pool: Arc<BufferPool>,
    idx: usize,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.pool.deallocate(self.idx);
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let start = self.idx * self.pool.block_size;
        &self.pool.buffer[start..start + self.pool.block_size]
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let buf: &[u8] = self.deref();
        unsafe { std::slice::from_raw_parts_mut(buf.as_ptr() as *mut u8, buf.len()) }
    }
}

impl Buffer {
    pub fn lkey(&self, device: &Device) -> u32 {
        self.pool.buffer.lkey(device.index())
    }

    pub fn rkey(&self, device: &Device) -> u32 {
        self.pool.buffer.rkey(device.index())
    }
}

impl BufferPool {
    pub fn create(block_size: usize, block_count: usize, devices: &Devices) -> Result<Arc<Self>> {
        let buffer_size = block_size * block_count;
        let buffer = RegisteredBuffer::create(devices, buffer_size)?;
        let free_list = Mutex::new((0..block_count).collect());
        Ok(Arc::new(Self {
            buffer,
            block_size,
            free_list,
        }))
    }

    pub fn allocate(self: &Arc<Self>) -> Result<Buffer> {
        let mut free_list = self.free_list.lock().unwrap();
        match free_list.pop() {
            Some(idx) => Ok(Buffer {
                pool: self.clone(),
                idx,
            }),
            None => Err(ErrorKind::AllocMemoryFailed.into()),
        }
    }

    fn deallocate(&self, idx: usize) {
        let mut free_list = self.free_list.lock().unwrap();
        free_list.push(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer() {
        const LEN: usize = 1 << 20;
        let devices = Devices::availables().unwrap();
        let buffer_pool = BufferPool::create(LEN, 32, &devices).unwrap();
        let mut buffer = buffer_pool.allocate().unwrap();
        assert_eq!(buffer.len(), LEN);
        buffer.fill(1);

        let mut another = buffer_pool.allocate().unwrap();
        assert_ne!(buffer.as_ptr(), another.as_ptr());
        another.fill(2);
        drop(another);
        drop(buffer);

        let buffer = buffer_pool.allocate().unwrap();
        assert_eq!(buffer.len(), LEN);
        buffer.iter().all(|&x| x == 1);

        let another = buffer_pool.allocate().unwrap();
        assert_eq!(another.len(), LEN);
        another.iter().all(|&x| x == 2);
    }
}
