use crate::*;
use r2dma_sys::*;
use std::{ops::Deref, sync::Mutex};

#[derive(Default, Debug)]
pub struct Work {
    pub recv: bool,
    pub buf: Option<BufferSlice>,
}

pub trait Submittable {
    fn sge(&self) -> ibv_sge;
    fn wr_id(&self) -> WorkRequestId;
    fn release(self);
}

pub struct WorkPool {
    _vec: Vec<Work>,
    base: usize,
    pool: Mutex<Vec<usize>>,
}

pub struct WorkRef<'a> {
    pool: &'a WorkPool,
    ptr: usize,
}

impl<'a> WorkRef<'a> {
    pub fn new(pool: &'a WorkPool, off: u32) -> Self {
        Self {
            pool,
            ptr: pool.base + off as usize,
        }
    }
}

impl Submittable for WorkRef<'_> {
    fn sge(&self) -> ibv_sge {
        self.buf.as_ref().map_or(ibv_sge::default(), |buf| ibv_sge {
            addr: buf.as_ref().as_ptr() as u64,
            length: buf.as_ref().len() as u32,
            lkey: buf.lkey(),
        })
    }

    fn wr_id(&self) -> WorkRequestId {
        let off = (self.ptr - self.pool.base) as u32;
        match self.recv {
            true => WorkRequestId::RecvData(off),
            false => WorkRequestId::SendData(off),
        }
    }

    fn release(self) {
        std::mem::forget(self)
    }
}

impl Drop for WorkRef<'_> {
    fn drop(&mut self) {
        self.buf = None;
        self.recv = false;
        self.pool.put(self.ptr)
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
        self.as_ref()
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

pub struct AsyncWork<T: Submittable + Sized>(pub T);

impl<T: Submittable + Sized> Submittable for AsyncWork<T> {
    fn sge(&self) -> ibv_sge {
        Default::default()
    }

    fn wr_id(&self) -> WorkRequestId {
        WorkRequestId::send_msg(self.0.wr_id())
    }

    fn release(self) {
        self.0.release()
    }
}

impl Submittable for WorkRequestId {
    fn sge(&self) -> ibv_sge {
        ibv_sge::default()
    }

    fn wr_id(&self) -> WorkRequestId {
        *self
    }

    fn release(self) {}
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
            base: pool[0],
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
        match id {
            WorkRequestId::SendData(id) => WorkRef::new(&work_pool, id),
            _ => panic!(),
        };
    }
}
