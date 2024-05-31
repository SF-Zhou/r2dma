use std::sync::Mutex;

use crate::*;

#[derive(Default)]
pub struct Work {
    pub bufs: Vec<BufferSlice>,
    pub sender: Option<tokio::sync::oneshot::Sender<Result<u32>>>,
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
    pub fn new(pool: &'a WorkPool, id: usize) -> Self {
        Self { pool, ptr: id as _ }
    }

    pub fn release(self) -> usize {
        let id = self.ptr;
        std::mem::forget(self);
        id
    }
}

impl Drop for WorkRef<'_> {
    fn drop(&mut self) {
        self.bufs.clear();
        self.sender = None;
        self.pool.put(self.ptr)
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
        assert!(work.bufs.is_empty());
        let id = work.release();
        let _ = WorkRef::new(&work_pool, id);
    }
}
