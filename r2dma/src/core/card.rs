use crate::ibv::*;
use crate::*;
use r2dma_sys::*;
use std::{
    borrow::Cow,
    sync::{mpsc, Arc},
};

#[derive(Debug)]
pub struct Card {
    pub event_loop: Arc<EventLoop>,
    pub protection_domain: ProtectionDomain,
    pub context: Context,
    pub port_attr: ibv_port_attr,
    pub gid: Gid,
}

impl Card {
    pub fn open(device: &Device) -> Result<Arc<Self>> {
        let context = Context::new(unsafe {
            let context = ibv_open_device(device.as_mut_ptr());
            if context.is_null() {
                return Err(Error::with_errno(ErrorKind::IBOpenDeviceFail));
            }
            context
        });

        let protection_domain = ProtectionDomain::new(unsafe {
            let protection_domain = ibv_alloc_pd(context.as_mut_ptr());
            if protection_domain.is_null() {
                return Err(Error::with_errno(ErrorKind::IBAllocPDFail));
            }
            protection_domain
        });

        let port_attr = context.query_port(1)?;
        let gid = context.query_gid(1, 1)?;

        let comp_channel = CompChannel::new(unsafe {
            let channel = ibv_create_comp_channel(context.as_mut_ptr());
            if channel.is_null() {
                return Err(Error::with_errno(ErrorKind::IBCreateCompChannelFail));
            }
            channel
        });
        comp_channel.set_nonblock()?;

        let event_loop = EventLoop::new(comp_channel).unwrap();

        Ok(Arc::new(Self {
            event_loop,
            protection_domain,
            context,
            port_attr,
            gid,
        }))
    }

    pub fn name(&self) -> Cow<str> {
        self.context.device().name()
    }

    pub fn start_comp_channel_consumer(self: &Arc<Self>) {
        let (_, receiver) = mpsc::sync_channel(1024);
        let event_loop = self.event_loop.clone();
        std::thread::spawn(move || {
            event_loop.run(receiver);
        });
    }

    pub fn stop_and_join(&self) -> Result<()> {
        self.event_loop.stop()?;
        Ok(())
    }
}

impl Drop for Card {
    fn drop(&mut self) {
        let _ = self.stop_and_join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ib_device() {
        let first_device = DeviceList::cached().first().unwrap();
        let card = Card::open(first_device).unwrap();
        println!("{:#?}", card);

        let context = &card.context;
        let gid = context.query_gid(1, 0).unwrap();
        println!("{:?} {} {}", gid, gid.subnet_prefix(), gid.interface_id());
        assert!(context.query_gid(1, u16::MAX).is_err());

        let port_attr = context.query_port(1).unwrap();
        println!("{:#?}", port_attr);
        assert!(context.query_port(10).is_err());
    }
}
