use std::sync::atomic::{AtomicU64, Ordering};

/// queue pair state
/// 24bit wr count + 7bit is_removed + 1bit is_error

const ERROR: u64 = 1 << 0;
const ACK_COUNT_SHIFT: u8 = 8;
const ACK_COUNT_MASK: u64 = (1 << WR_COUNT_SHIFT) - (1 << ACK_COUNT_SHIFT);
const WR_COUNT_SHIFT: u8 = 32;
const WR_COUNT_MASK: u64 = u64::MAX - (1 << WR_COUNT_SHIFT) + 1;

trait Bits {
    fn v(&self) -> u64;

    #[inline(always)]
    fn is_ok(&self) -> bool {
        self.v() & ERROR == 0
    }

    #[inline(always)]
    fn wr_cnt(&self) -> u64 {
        (self.v() & WR_COUNT_MASK) >> WR_COUNT_SHIFT
    }

    #[inline(always)]
    fn ack_cnt(&self) -> u64 {
        (self.v() & ACK_COUNT_MASK) >> ACK_COUNT_SHIFT
    }

    #[inline(always)]
    fn from_wr_cnt(wr_cnt: u64) -> u64 {
        wr_cnt << WR_COUNT_SHIFT
    }

    #[inline(always)]
    fn from_ack_cnt(ack_cnt: u32) -> u64 {
        (ack_cnt as u64) << ACK_COUNT_SHIFT
    }

    #[inline(always)]
    fn from_wr_and_ack_cnt(wr_cnt: u64, ack_cnt: u64) -> u64 {
        (wr_cnt << WR_COUNT_SHIFT) | (ack_cnt << ACK_COUNT_SHIFT)
    }
}

impl Bits for u64 {
    #[inline(always)]
    fn v(&self) -> u64 {
        *self
    }
}

pub struct State {
    state: AtomicU64,
    remain: AtomicU64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            state: AtomicU64::new(u64::from_wr_cnt(1)), // 1 for the last wake up wr.
            remain: AtomicU64::new(u64::MAX),
        }
    }
}

pub enum ApplyResult {
    Succ,
    Async,
    Error,
}

impl State {
    pub fn apply_send(&self, unack_limit: u32) -> ApplyResult {
        let diff = u64::from_wr_and_ack_cnt(1 + 1, 1); // one for current request, one for async submit.
        let current = self.state.fetch_add(diff, Ordering::SeqCst) + diff;
        if !current.is_ok() {
            ApplyResult::Error
        } else if current.ack_cnt() > unack_limit as _ {
            ApplyResult::Async
        } else {
            ApplyResult::Succ
        }
    }

    pub fn apply_recv(&self) -> ApplyResult {
        let diff = u64::from_wr_cnt(1);
        match self.state.fetch_add(diff, Ordering::SeqCst).is_ok() {
            true => ApplyResult::Succ,
            false => ApplyResult::Error,
        }
    }

    pub fn recv_ack(&self, ack: u32) {
        let diff = u64::from_ack_cnt(ack);
        self.state.fetch_sub(diff, Ordering::SeqCst);
    }

    // prepare to submit work request. return true if ok.
    #[inline(always)]
    pub fn prepare_submit(&self) -> bool {
        let diff = u64::from_wr_and_ack_cnt(1, 1);
        self.state.fetch_add(diff, Ordering::SeqCst).is_ok()
    }

    // set to error. return true if need send a wake up wr.
    #[inline(always)]
    pub fn set_error(&self) -> bool {
        let bits = self.state.fetch_or(ERROR, Ordering::SeqCst);
        if bits.is_ok() {
            // first to set error.
            let diff = u64::MAX - bits.wr_cnt();
            self.remain.fetch_sub(diff, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    // remove socket if return value is true.
    #[inline(always)]
    pub fn work_complete(&self, cnt: u64) -> bool {
        let diff = u64::from_wr_cnt(cnt);
        let bits = self.state.fetch_sub(diff, Ordering::SeqCst);
        assert!(bits >= diff, "bad state: {:X}, cnt: {}", bits, cnt);

        if !bits.is_ok() {
            let old = self.remain.fetch_sub(cnt, Ordering::SeqCst);
            if old == cnt {
                return true;
            }
        }
        false
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bits = self.state.load(Ordering::Acquire);
        let is_ok = bits.is_ok();
        let wr_cnt = bits.wr_cnt();
        let remain_wr = if is_ok {
            None
        } else {
            Some(self.remain.load(Ordering::Acquire))
        };
        f.debug_struct("State")
            .field("is_ok", &is_ok)
            .field("wr_cnt", &wr_cnt)
            .field("remain_wr", &remain_wr)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_normal() {
        let state = State::default();
        assert!(state.set_error());
        assert!(!state.prepare_submit());

        let state = State::default();
        assert!(state.prepare_submit()); // 1
        assert!(state.prepare_submit()); // 2
        assert!(!state.work_complete(2)); // 0

        assert!(state.prepare_submit()); // 1
        assert!(state.set_error());
        // need one more submit.
        assert!(!state.work_complete(1)); // remain 1
        assert!(state.work_complete(1)); // remain 0

        let bits = u64::from_wr_and_ack_cnt(233, 2333);
        assert_eq!(bits.wr_cnt(), 233);
        assert_eq!(bits.ack_cnt(), 2333);
    }

    #[test]
    #[should_panic]
    fn test_state_panic() {
        let state = State::default();
        state.work_complete(1);
        state.work_complete(1);
    }
}
