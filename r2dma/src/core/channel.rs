use crate::*;
use ibv::CompChannel;
use r2dma_sys::*;
use std::sync::Arc;

#[derive(Debug)]
pub struct Channel {
    comp_channel: ibv::CompChannel,
    pub card: Arc<Card>,
}

impl Channel {
    pub fn new(card: &Arc<Card>) -> Result<Self> {
        let comp_channel = ibv::CompChannel::new(unsafe {
            let channel = ibv_create_comp_channel(card.context.as_mut_ptr());
            if channel.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCompChannelFail));
            }
            channel
        });
        comp_channel.set_nonblock()?;

        Ok(Self {
            comp_channel,
            card: card.clone(),
        })
    }
}

impl std::ops::Deref for Channel {
    type Target = CompChannel;

    fn deref(&self) -> &Self::Target {
        &self.comp_channel
    }
}
