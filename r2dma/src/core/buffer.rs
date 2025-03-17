use super::*;
use crate::{ibv::MemoryRegion, Error, Result};
use std::alloc::Layout;

pub const ALIGN_SIZE: usize = 4096;

struct AlignedBuffer(&'static mut [u8]);

impl AlignedBuffer {
    pub fn new(size: usize) -> Result<Self> {
        let buf = unsafe {
            let size = std::cmp::max(size, 1).next_multiple_of(ALIGN_SIZE);
            let layout = Layout::from_size_align_unchecked(size, ALIGN_SIZE);
            let ptr = std::alloc::alloc(layout);
            if ptr.is_null() {
                return Err(Error::AllocMemoryFailed);
            }
            std::slice::from_raw_parts_mut(ptr, size)
        };
        Ok(Self(buf))
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.0.len(), ALIGN_SIZE);
            std::alloc::dealloc(self.0.as_mut_ptr(), layout);
        }
    }
}

impl std::ops::Deref for AlignedBuffer {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl std::ops::DerefMut for AlignedBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

pub struct Buffer {
    aligned_buffer: AlignedBuffer,
    memory_regions: Vec<MemoryRegion>,
}

impl Buffer {
    pub fn new(size: usize, devices: &Devices) -> Result<Self> {
        let aligned_buffer = AlignedBuffer::new(size)?;
        let mut memory_regions = vec![];
        for device in devices {
            let memory_region = MemoryRegion::create(device.pd(), &aligned_buffer)?;
            memory_regions.push(memory_region);
        }
        Ok(Self {
            aligned_buffer,
            memory_regions,
        })
    }

    pub fn lkey(&self, index: usize) -> u32 {
        self.memory_regions[index].lkey
    }

    pub fn rkey(&self, index: usize) -> u32 {
        self.memory_regions[index].rkey
    }
}

impl std::ops::Deref for Buffer {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.aligned_buffer.deref()
    }
}

impl std::ops::DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.aligned_buffer.deref_mut()
    }
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lkeys = self
            .memory_regions
            .iter()
            .map(|mr| mr.lkey)
            .collect::<Vec<_>>();
        let rkeys = self
            .memory_regions
            .iter()
            .map(|mr| mr.rkey)
            .collect::<Vec<_>>();
        f.debug_struct("Buffer")
            .field("addr", &self.as_ptr())
            .field("len", &self.len())
            .field("lkeys", &lkeys)
            .field("rkeys", &rkeys)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer() {
        let devices = Device::avaiables(&DeviceConfig::default()).unwrap();
        let buffer = Buffer::new(1024, &devices).unwrap();
        assert_eq!(buffer.len(), ALIGN_SIZE);
        println!("{:#?}", buffer);
    }
}
