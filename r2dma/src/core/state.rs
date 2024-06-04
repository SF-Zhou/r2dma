use std::sync::atomic::{AtomicU64, Ordering};

/// queue pair state
/// 24bit wr count + 7bit is_removed + 1bit is_error

const ERROR: u64 = 1 << 0;
const COUNT: u64 = 1 << 8;
const COUNT_SHIFT: u8 = 8;

trait Bits {
    fn v(&self) -> u64;

    #[inline(always)]
    fn is_ok(&self) -> bool {
        self.v() & ERROR == 0
    }

    #[inline(always)]
    fn is_error(&self) -> bool {
        self.v() & ERROR != 0
    }

    #[inline(always)]
    fn wr_cnt(&self) -> u64 {
        self.v() >> COUNT_SHIFT
    }

    #[inline(always)]
    fn from_wr_cnt(cnt: u64) -> u64 {
        cnt << COUNT_SHIFT
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
            state: AtomicU64::new(COUNT), // 1 for the last wake up wr.
            remain: AtomicU64::new(u64::MAX),
        }
    }
}

impl State {
    // prepare to submit work request. return true if ok.
    #[inline(always)]
    pub fn prepare_submit(&self) -> bool {
        self.state.fetch_add(COUNT, Ordering::SeqCst).is_ok()
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

        if bits.is_error() {
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
        let is_error = bits.is_error();
        let wr_cnt = bits.wr_cnt();
        let remain_wr = if is_error {
            Some(self.remain.load(Ordering::Acquire))
        } else {
            None
        };
        f.debug_struct("State")
            .field("is_error", &is_error)
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
    }

    #[test]
    #[should_panic]
    fn test_state_panic() {
        let state = State::default();
        state.work_complete(1);
    }
}
