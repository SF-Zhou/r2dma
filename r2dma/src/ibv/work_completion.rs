use super::*;

impl ibv_wc {
    pub fn result(&self) -> Result<u32, ibv_wc_status> {
        if self.status == ibv_wc_status::IBV_WC_SUCCESS {
            Ok(self.byte_len)
        } else {
            Err(self.status)
        }
    }

    pub fn imm(&self) -> Option<u32> {
        if self.wc_flags & ibv_wc_flags::IBV_WC_WITH_IMM.0 != 0 {
            Some(u32::from_be(unsafe { self.__bindgen_anon_1.imm_data }))
        } else {
            None
        }
    }

    pub fn extract<T>(&mut self) -> Box<T> {
        let ptr = std::mem::take(&mut self.wr_id);
        unsafe { Box::from_raw(ptr as *mut _) }
    }
}

impl std::fmt::Debug for ibv_wc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let imm = self.imm();
        f.debug_struct("WorkCompletion")
            .field("wr_id", &self.wr_id)
            .field("status", &self.status)
            .field("opcode", &self.opcode)
            .field("byte_len", &self.byte_len)
            .field("wc_flags", &self.wc_flags)
            .field("imm", &imm)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_completion() {
        let mut wc = ibv_wc::default();
        println!("{:#?}", wc);
        assert_eq!(wc.result(), Ok(0));
        assert_eq!(wc.imm(), None);

        wc.status = ibv_wc_status::IBV_WC_WR_FLUSH_ERR;
        assert_eq!(wc.result(), Err(ibv_wc_status::IBV_WC_WR_FLUSH_ERR));

        wc.wc_flags = ibv_wc_flags::IBV_WC_WITH_IMM.0;
        assert_eq!(wc.imm(), Some(0));

        let value = Box::new(32);
        wc.wr_id = Box::into_raw(value) as *const _ as _;
        let value: Box<i32> = wc.extract();
        assert_eq!(*value, 32);
    }
}
