use std::sync::atomic::{AtomicU64, Ordering};

const ERROR: u64 = 1 << 63;

pub struct State {
    max_send: u32,
    max_read: u32,

    last_send: AtomicU64,
    last_read: AtomicU64,

    send_index: AtomicU64,
    send_local_finished: AtomicU64,
    send_remote_finished: AtomicU64,

    receiving: AtomicU64,

    read_index: AtomicU64,
    read_finished: AtomicU64,
}

impl State {
    pub fn new(max_send: u32, max_read: u32) -> Self {
        Self {
            max_send,
            max_read,
            last_send: Default::default(),
            last_read: Default::default(),
            send_index: Default::default(),
            send_local_finished: Default::default(),
            send_remote_finished: Default::default(),
            receiving: Default::default(),
            read_index: Default::default(),
            read_finished: Default::default(),
        }
    }

    pub fn apply_send(&self) -> Option<u64> {
        let bits = self.send_index.fetch_add(1, Ordering::SeqCst);
        if bits & ERROR == 0 {
            Some(bits)
        } else {
            None
        }
    }

    pub fn apply_recv(&self) {
        self.receiving.fetch_add(1, Ordering::SeqCst);
    }

    pub fn recv_complete(&self) {
        self.receiving.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn apply_read(&self) -> Option<u64> {
        let bits = self.read_index.fetch_add(1, Ordering::SeqCst);
        if bits & ERROR == 0 {
            Some(bits)
        } else {
            None
        }
    }

    pub fn check_send_index(&self, index: u64) -> bool {
        index < self.send_remote_finished.load(Ordering::Acquire) + self.max_send as u64
    }

    pub fn check_notify_index(&self, index: u64) -> bool {
        index < self.send_remote_finished.load(Ordering::Acquire) + self.max_send as u64 + 1
    }

    pub fn check_read_index(&self, index: u64) -> bool {
        index < self.read_finished.load(Ordering::Acquire) + self.max_read as u64
    }

    pub fn send_fail(&self) {
        self.send_local_finished.fetch_add(1, Ordering::SeqCst);
        self.send_remote_finished.fetch_add(1, Ordering::SeqCst);
    }

    pub fn send_local_complete(&self, cnt: u64) -> u64 {
        self.send_local_finished.fetch_add(cnt, Ordering::SeqCst) + cnt + self.max_send as u64
    }

    pub fn send_remote_complete(&self, cnt: u64) -> u64 {
        self.send_remote_finished.fetch_add(cnt, Ordering::SeqCst) + cnt + self.max_send as u64
    }

    pub fn read_fail(&self) {
        self.read_finished.fetch_add(1, Ordering::SeqCst);
    }

    pub fn read_complete(&self, cnt: u64) -> u64 {
        self.read_finished.fetch_add(cnt, Ordering::SeqCst) + cnt + self.max_read as u64
    }

    pub fn set_error(&self) -> bool {
        let bits = self.send_index.fetch_or(ERROR, Ordering::SeqCst);
        let first_set = if bits & ERROR == 0 {
            // first to set error.
            self.last_send.store(bits | ERROR, Ordering::SeqCst);
            true
        } else {
            false
        };

        let bits = self.read_index.fetch_or(ERROR, Ordering::SeqCst);
        if bits & ERROR == 0 {
            self.last_read.store(bits, Ordering::SeqCst);
        }

        first_set
    }

    pub fn is_ok(&self) -> bool {
        self.last_send.load(Ordering::Acquire) & ERROR == 0
    }

    pub fn ready_to_remove(&self) -> bool {
        let bits = self.last_send.load(Ordering::Acquire);
        bits & ERROR != 0
            && (bits & !ERROR) == self.send_local_finished.load(Ordering::Acquire)
            && self.last_read.load(Ordering::Acquire) == self.read_finished.load(Ordering::Acquire)
            && self.receiving.load(Ordering::Acquire) == 0
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bits = self.send_index.load(Ordering::Acquire);
        let is_ok = bits & ERROR == 0;
        let send_index = bits & !ERROR;
        let send_local_finished = self.send_local_finished.load(Ordering::Acquire);
        let send_remote_finished = self.send_remote_finished.load(Ordering::Acquire);
        let receiving = self.receiving.load(Ordering::Acquire);
        let read_index = self.read_index.load(Ordering::Acquire) & !ERROR;
        let read_finished = self.read_finished.load(Ordering::Acquire);
        let (last_send, last_read) = if is_ok {
            (None, None)
        } else {
            (
                Some(self.last_send.load(Ordering::Acquire) & !ERROR),
                Some(self.last_read.load(Ordering::Acquire)),
            )
        };
        f.debug_struct("State")
            .field("is_ok", &is_ok)
            .field("send_index", &send_index)
            .field("send_local_finished", &send_local_finished)
            .field("send_remote_finished", &send_remote_finished)
            .field("receiving", &receiving)
            .field("read_index", &read_index)
            .field("read_finished", &read_finished)
            .field("last_send", &last_send)
            .field("last_read", &last_read)
            .field("max_send", &self.max_send)
            .field("max_read", &self.max_read)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_normal() {
        let state = State::new(16, 16);
        println!("{:#?}", state);

        for _ in 0..16 {
            let index = state.apply_send().unwrap();
            assert!(state.check_send_index(index));
        }
        let index = state.apply_send().unwrap();
        assert!(!state.check_send_index(index));

        state.send_remote_complete(1);
        assert!(state.check_send_index(index));
    }
}
