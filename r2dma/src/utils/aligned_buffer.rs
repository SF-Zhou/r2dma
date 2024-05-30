use std::alloc::Layout;

pub struct AlignedBuffer(&'static mut [u8]);

impl AlignedBuffer {
    const ALIGN: usize = 512;

    pub fn new(size: usize) -> Self {
        Self(unsafe {
            let size = size.next_multiple_of(Self::ALIGN);
            let layout = Layout::from_size_align_unchecked(size, Self::ALIGN);
            let ptr = std::alloc::alloc(layout);
            std::slice::from_raw_parts_mut(ptr, size)
        })
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.0.len(), Self::ALIGN);
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
