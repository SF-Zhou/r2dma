const EMPTY: u64 = 0;
const SEND_DATA: u64 = 1;
const SEND_IMM: u64 = 2;
const ASYNC_SEND_DATA: u64 = 3;
const ASYNC_SEND_IMM: u64 = 4;
const SEND_MSG: u64 = 8;
const RECV_DATA: u64 = 16;

const TYPE_SHIFT: u8 = 48;
const MSG_SHIFT: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkRequestId {
    Empty, // do notiong.
    SendData(u32),
    SendImm(u32),
    AsyncSendData(u32),
    AsyncSendImm(u32),
    SendMsg(u8, u32),
    RecvData(u32),
}

impl WorkRequestId {
    pub fn send_msg(id: WorkRequestId) -> Self {
        match id {
            Self::SendMsg(_, _) => panic!("invalid send msg wrapper of send msg"),
            Self::RecvData(_) => panic!("invalid send msg wrapper of recv data"),
            Self::AsyncSendData(_) | Self::AsyncSendImm(_) => {
                panic!("invalid send msg wrapper of async send")
            }
            Self::Empty => Self::SendMsg(EMPTY as u8, 0),
            Self::SendData(v) => Self::SendMsg(ASYNC_SEND_DATA as u8, v),
            Self::SendImm(v) => Self::SendMsg(ASYNC_SEND_IMM as u8, v),
        }
    }

    pub fn msg(&self) -> Self {
        match *self {
            Self::SendMsg(m, v) => Self::from((m as u64) << TYPE_SHIFT | v as u64),
            _ => panic!("invalid WorkRequestId::SendMsg {self:?}!"),
        }
    }
}

impl From<WorkRequestId> for u64 {
    fn from(value: WorkRequestId) -> Self {
        match value {
            WorkRequestId::Empty => 0,
            WorkRequestId::SendData(v) => (SEND_DATA << TYPE_SHIFT) | v as u64,
            WorkRequestId::SendImm(v) => (SEND_IMM << TYPE_SHIFT) | v as u64,
            WorkRequestId::AsyncSendData(v) => (ASYNC_SEND_DATA << TYPE_SHIFT) | v as u64,
            WorkRequestId::AsyncSendImm(v) => (ASYNC_SEND_IMM << TYPE_SHIFT) | v as u64,
            WorkRequestId::SendMsg(t, v) => {
                (SEND_MSG << TYPE_SHIFT) | (t as u64) << MSG_SHIFT | v as u64
            }
            WorkRequestId::RecvData(v) => (RECV_DATA << TYPE_SHIFT) | v as u64,
        }
    }
}

impl From<u64> for WorkRequestId {
    fn from(value: u64) -> Self {
        let t = value >> TYPE_SHIFT;
        let m = (value >> MSG_SHIFT) as u8;
        let v = value as u32;
        match t {
            EMPTY => WorkRequestId::Empty,
            SEND_DATA => WorkRequestId::SendData(v),
            SEND_IMM => WorkRequestId::SendImm(v),
            ASYNC_SEND_DATA => WorkRequestId::AsyncSendData(v),
            ASYNC_SEND_IMM => WorkRequestId::AsyncSendImm(v),
            SEND_MSG => WorkRequestId::SendMsg(m, v),
            RECV_DATA => WorkRequestId::RecvData(v),
            _ => panic!("invalid t: {t}, m: {m}, v: {v}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_request_id() {
        for t in [
            EMPTY,
            SEND_DATA,
            SEND_IMM,
            ASYNC_SEND_DATA,
            ASYNC_SEND_IMM,
            SEND_MSG,
            RECV_DATA,
        ] {
            let a = t << TYPE_SHIFT;
            let wr_id = WorkRequestId::from(a);
            let b = u64::from(wr_id);
            assert_eq!(a, b);
        }
    }
}
