use crate::ibv::*;
use crate::*;
use r2dma_sys::*;

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

const MEMORY_ACCESS_FLAGS: i32 = (ibv_access_flags::IBV_ACCESS_LOCAL_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_READ.0
    | ibv_access_flags::IBV_ACCESS_RELAXED_ORDERING.0) as i32;

pub struct Buffer {
    pub regions: Vec<MemoryRegion>,
    aligned_buffer: utils::AlignedBuffer,
    _cards: Arc<Vec<Arc<Card>>>,
}

impl Buffer {
    pub fn new(cards: &Arc<Vec<Arc<Card>>>, size: usize) -> Result<Self> {
        let aligned_buffer = utils::AlignedBuffer::new(size);

        let mut regions = vec![];
        for card in cards.deref() {
            let memory_region = MemoryRegion::new(unsafe {
                let memory_region = ibv_reg_mr(
                    card.protection_domain.as_mut_ptr(),
                    aligned_buffer.as_ptr() as _,
                    aligned_buffer.len(),
                    MEMORY_ACCESS_FLAGS,
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
        let cards = Cards::open().unwrap();
        let mem = Buffer::new(&cards.cards, 1024).unwrap();
        println!("{:#?}", mem);
    }
}
