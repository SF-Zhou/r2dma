use crate::*;
use r2dma_sys::*;

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub struct Buffer {
    pub regions: Vec<ibv::MemoryRegion>,
    aligned_buffer: utils::AlignedBuffer,
    _cards: Arc<Cards>,
}

impl Buffer {
    pub fn new(cards: &Arc<Cards>, size: usize) -> Result<Self> {
        let aligned_buffer = utils::AlignedBuffer::new(size);

        let mut regions = vec![];
        for card in cards.as_ref().deref() {
            let memory_region = ibv::MemoryRegion::new(unsafe {
                let memory_region = ibv_reg_mr(
                    card.protection_domain.as_mut_ptr(),
                    aligned_buffer.as_ptr() as _,
                    aligned_buffer.len(),
                    ibv::ACCESS_FLAGS as _,
                );
                if memory_region.is_null() {
                    return Err(Error::with_errno(ErrorKind::IBRegMRFail));
                }
                memory_region
            });
            regions.push(memory_region);
        }

        Ok(Self {
            regions,
            aligned_buffer,
            _cards: cards.clone(),
        })
    }
}

impl std::ops::Deref for Buffer {
    type Target = ibv_mr;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.regions[0]
    }
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.regions, f)
    }
}

impl std::convert::AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self.aligned_buffer.deref()
    }
}

impl std::convert::AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        self.aligned_buffer.deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ib_memory() {
        let config = Config::default();
        let manager = Manager::init(&config).unwrap();
        let mut mem = Buffer::new(&manager.cards, 1024).unwrap();
        println!("{:#?}", mem);
        assert_eq!(mem.as_ref().len(), 1024);
        assert_eq!(mem.as_mut().len(), 1024);
    }
}
