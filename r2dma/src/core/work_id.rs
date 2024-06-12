use crate::BufferSlice;
use r2dma_sys::ibv_sge;

const IMM_SHIFT: u8 = 1;
const BOX_SHIFT: u8 = 2;

#[derive(Debug)]
pub enum WorkID {
    Empty,
    Imm(u32),
    Box(Box<Work>),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WorkType {
    #[default]
    SEND,
    RECV,
    READ,
}

#[derive(Debug, Default)]
pub struct Work {
    pub ty: WorkType,
    pub buf: Option<BufferSlice>,
}

impl From<Box<Work>> for WorkID {
    fn from(val: Box<Work>) -> Self {
        WorkID::Box(val)
    }
}

impl WorkID {
    pub fn is_send(&self) -> bool {
        match self {
            WorkID::Box(b) => b.ty == WorkType::SEND,
            WorkID::Imm(_) => true,
            _ => false,
        }
    }

    pub fn is_recv(&self) -> bool {
        match self {
            WorkID::Box(b) => b.ty == WorkType::RECV,
            _ => false,
        }
    }

    pub fn is_read(&self) -> bool {
        match self {
            WorkID::Box(b) => b.ty == WorkType::READ,
            _ => false,
        }
    }

    pub fn sge(&self) -> ibv_sge {
        match self {
            WorkID::Box(b) if b.buf.is_some() => {
                let buf = b.buf.as_ref().unwrap();
                let slice = buf.as_ref();
                ibv_sge {
                    addr: slice.as_ptr() as _,
                    length: slice.len() as _,
                    lkey: buf.lkey(),
                }
            }
            _ => Default::default(),
        }
    }
}

impl From<&WorkID> for u64 {
    fn from(value: &WorkID) -> Self {
        match value {
            WorkID::Empty => 0,
            WorkID::Box(b) => b.as_ref() as *const _ as u64,
            WorkID::Imm(v) => (*v as u64) << BOX_SHIFT | (1 << IMM_SHIFT),
        }
    }
}

impl From<u64> for WorkID {
    fn from(value: u64) -> Self {
        if value == 0 {
            WorkID::Empty
        } else if (value & (1 << IMM_SHIFT)) != 0 {
            WorkID::Imm((value >> BOX_SHIFT) as u32)
        } else {
            WorkID::Box(unsafe { Box::from_raw(value as *mut _) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_request_id() {
        for wr_id in [
            WorkID::Empty,
            WorkID::Imm(233),
            WorkID::Box(Default::default()),
        ] {
            let value = u64::from(&wr_id);
            std::mem::forget(wr_id);
            let _ = WorkID::from(value);
        }
    }
}
