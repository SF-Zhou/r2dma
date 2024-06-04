use std::sync::atomic::{AtomicU64, Ordering};

/// queue pair state
/// 24bit wr count + 7bit is_removed + 1bit is_error
#[derive(Default)]
pub struct State(AtomicU64);

impl State {
    const ERROR: u64 = 1 << 0;
    const REMOVED: u64 = 1 << 1;
    const REMOVED_MARK: u64 = 0xFE;
    const COUNT: u64 = 1 << 8;
    const COUNT_SHIFT: u8 = 8;

    // prepare to submit work request. return true if ok.
    #[inline(always)]
    pub fn prepare_submit(&self) -> bool {
        let value = self.0.fetch_add(Self::COUNT, Ordering::SeqCst);
        value & Self::ERROR == 0
    }

    #[inline(always)]
    pub fn set_error_and_try_to_remove(&self) -> bool {
        let value = self.0.fetch_or(Self::ERROR, Ordering::SeqCst);
        if value == 0 {
            // count is zero, not removed.
            let value = self.0.fetch_add(Self::REMOVED, Ordering::SeqCst);
            if value & Self::REMOVED_MARK == 0 {
                return true;
            }
        }
        false
    }

    // call this if prepare_sumbmit is failed.
    #[inline(always)]
    pub fn rollback_submit_and_try_to_remove(&self) -> bool {
        self.work_complete(1)
    }

    // remove socket if return value is true.
    #[inline(always)]
    pub fn work_complete(&self, cnt: u64) -> bool {
        let diff = cnt * Self::COUNT;
        let value = self.0.fetch_sub(diff, Ordering::SeqCst);
        if value & Self::ERROR != 0 && value >> Self::COUNT_SHIFT == cnt {
            // count is zero, not removed.
            let value = self.0.fetch_add(Self::REMOVED, Ordering::SeqCst);
            if value & Self::REMOVED_MARK == 0 {
                return true;
            }
        } else if value < diff {
            panic!("bad state: {:X}, cnt: {}", value, cnt);
        }
        false
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.0.load(Ordering::Acquire);
        let is_error = state & Self::ERROR != 0;
        let is_removed = (state & Self::REMOVED_MARK) >> 1;
        let wr_cnt = state >> Self::COUNT_SHIFT;
        f.debug_struct("State")
            .field("wr_cnt", &wr_cnt)
            .field("is_removed", &is_removed)
            .field("is_error", &is_error)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_normal() {
        let state = State::default();
        assert!(state.set_error_and_try_to_remove());
        assert!(!state.prepare_submit());

        let state = State::default();
        assert!(state.prepare_submit()); // 1
        assert!(state.prepare_submit()); // 2
        assert!(!state.work_complete(2)); // 0

        assert!(state.prepare_submit()); // 1
        assert!(!state.set_error_and_try_to_remove());
        assert!(state.work_complete(1)); // 0
    }

    #[test]
    #[should_panic]
    fn test_state_panic() {
        let state = State::default();
        state.work_complete(1);
    }
}
