use r2dma_sys::*;

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct WorkCompletion(ibv_wc);

impl WorkCompletion {
    pub fn result(&self) -> Result<u32, ibv_wc_status> {
        if self.0.status == ibv_wc_status::IBV_WC_SUCCESS {
            Ok(self.0.byte_len)
        } else {
            Err(self.0.status)
        }
    }

    pub fn imm(&self) -> Option<u32> {
        if self.0.wc_flags & ibv_wc_flags::IBV_WC_WITH_IMM.0 != 0 {
            Some(unsafe { self.0.__bindgen_anon_1.imm_data })
        } else {
            None
        }
    }
}

impl std::ops::Deref for WorkCompletion {
    type Target = ibv_wc;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for WorkCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkCompletion")
            .field("wr_id", &self.0.wr_id)
            .field("status", &self.0.status)
            .field("opcode", &self.0.opcode)
            .field("byte_len", &self.0.byte_len)
            .field("wc_flags", &self.0.wc_flags)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_completion() {
        let mut wc = WorkCompletion::default();
        println!("{:#?}", wc);
        assert_eq!(wc.result(), Ok(0));
        assert_eq!(wc.imm(), None);

        wc.0.status = ibv_wc_status::IBV_WC_WR_FLUSH_ERR;
        assert_eq!(wc.result(), Err(ibv_wc_status::IBV_WC_WR_FLUSH_ERR));

        wc.0.wc_flags = ibv_wc_flags::IBV_WC_WITH_IMM.0;
        assert_eq!(wc.imm(), Some(0));
    }
}
