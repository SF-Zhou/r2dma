use std::{num::NonZeroU32, ops::Deref, sync::Mutex};

use crate::*;

#[derive(Debug)]
pub enum WorkType {
    Send,
    Recv,
}

impl Default for WorkType {
    fn default() -> Self {
        Self::Send
    }
}

#[derive(Default, Debug)]
pub struct Work {
    pub ty: WorkType,
    pub imm: Option<NonZeroU32>,
    pub buf: Option<BufferSlice>,
}

pub trait Submittable {
    fn wr_id(&self) -> u64;
    fn release(self);
}

impl Submittable for Work {
    fn wr_id(&self) -> u64 {
        0
    }

    fn release(self) {
        drop(self)
    }
}

pub struct WorkPool {
    _vec: Vec<Work>,
    pool: Mutex<Vec<usize>>,
}

pub struct WorkRef<'a> {
    pool: &'a WorkPool,
    ptr: usize,
}

impl<'a> WorkRef<'a> {
    pub fn new(pool: &'a WorkPool, wr_id: u64) -> Self {
        Self {
            pool,
            ptr: wr_id as _,
        }
    }
}

impl Submittable for WorkRef<'_> {
    fn wr_id(&self) -> u64 {
        self.ptr as _
    }

    fn release(self) {
        std::mem::forget(self)
    }
}

impl Drop for WorkRef<'_> {
    fn drop(&mut self) {
        self.buf = None;
        self.pool.put(self.ptr)
    }
}

impl AsRef<Work> for Work {
    fn as_ref(&self) -> &Work {
        self
    }
}

impl AsRef<Work> for WorkRef<'_> {
    fn as_ref(&self) -> &Work {
        unsafe { &*(self.ptr as *const _) }
    }
}

impl std::ops::Deref for WorkRef<'_> {
    type Target = Work;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.ptr as *mut _) }
    }
}

impl std::ops::DerefMut for WorkRef<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.ptr as *mut _) }
    }
}

impl std::fmt::Debug for WorkRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl WorkPool {
    pub fn new(size: usize) -> Self {
        let mut vec = Vec::with_capacity(size);
        let mut pool = Vec::with_capacity(size);
        for idx in 0..size {
            vec.push(Work::default());
            pool.push(&mut vec[idx] as *mut _ as _);
        }

        Self {
            _vec: vec,
            pool: Mutex::new(pool),
        }
    }

    pub fn get(&self) -> Result<WorkRef> {
        match self.pool.lock().unwrap().pop() {
            Some(ptr) => Ok(WorkRef { pool: self, ptr }),
            None => Err(Error::new(ErrorKind::AllocateWorkFail)),
        }
    }

    pub fn put(&self, ptr: usize) {
        self.pool.lock().unwrap().push(ptr)
    }

    pub fn remain(&self) -> usize {
        self.pool.lock().unwrap().len()
    }
}

impl std::fmt::Debug for WorkPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let remain = self.remain();
        f.debug_struct("WorkPool").field("remain", &remain).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_pool() {
        let work_pool = WorkPool::new(1024);
        println!("{:?}", work_pool);

        let mut vec = vec![];
        for _ in 0..1024 {
            vec.push(work_pool.get().unwrap());
        }
        assert!(work_pool.get().is_err());
        drop(vec);

        let work = work_pool.get().unwrap();
        assert!(work.buf.is_none());
        let id = work.wr_id();
        work.release();
        let _ = WorkRef::new(&work_pool, id);
    }
}
