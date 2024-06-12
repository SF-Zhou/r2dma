use crate::*;

#[derive(Debug)]
pub struct WorkPool;

impl WorkPool {
    pub fn new(_size: usize) -> Self {
        Self
    }

    pub fn get(&self) -> Result<Box<Work>> {
        Ok(Default::default())
    }
}
