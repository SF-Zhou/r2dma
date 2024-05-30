use crate::ibv::*;
use crate::*;
use r2dma_sys::*;

use std::sync::Arc;

const MEMORY_ACCESS_FLAGS: i32 = (ibv_access_flags::IBV_ACCESS_LOCAL_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_WRITE.0
    | ibv_access_flags::IBV_ACCESS_REMOTE_READ.0
    | ibv_access_flags::IBV_ACCESS_RELAXED_ORDERING.0) as i32;

pub struct Buffer {
    memory_region: MemoryRegion,
    _dev: Arc<Card>,
}

impl Buffer {
    pub fn new(card: &Arc<Card>, size: usize) -> Result<Self> {
        let aligned_buffer = utils::AlignedBuffer::new(size);
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
        std::mem::forget(aligned_buffer);

        Ok(Self {
            memory_region,
            _dev: card.clone(),
        })
    }
}

impl std::ops::Deref for Buffer {
    type Target = ibv_mr;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.memory_region
    }
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.memory_region, f)
    }
}

impl std::convert::AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.memory_region.addr as _, self.memory_region.length)
        }
    }
}

impl std::convert::AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.memory_region.addr as _, self.memory_region.length)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ib_memory() {
        let cards = Cards::open().unwrap();
        let card = cards.get(None).unwrap();
        let mem = Buffer::new(&card, 1024).unwrap();
        println!("{:#?}", mem);
    }
}
