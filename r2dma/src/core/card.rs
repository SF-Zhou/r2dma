use crate::ibv::*;
use crate::*;
use r2dma_sys::*;
use std::{
    borrow::Cow,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[derive(Debug)]
pub struct Card {
    pub comp_channel: CompChannel,
    pub protection_domain: ProtectionDomain,
    pub context: Context,
    pub port_attr: ibv_port_attr,
    pub gid: Gid,
    pub stopping: AtomicBool,
    pub thread: Mutex<Option<std::thread::JoinHandle<()>>>,
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

        Ok(Arc::new(Self {
            comp_channel,
            protection_domain,
            context,
            port_attr,
            gid,
            stopping: Default::default(),
            thread: Default::default(),
        }))
    }

    pub fn name(&self) -> Cow<str> {
        self.context.device().name()
    }

    pub fn start_comp_channel_consumer(self: &Arc<Self>) {
        let clone = self.clone();
        std::thread::spawn(move || {
            while !clone.stopping.load(Ordering::Acquire) {
                match clone.comp_channel.wait() {
                    Ok(0) => continue,
                    Ok(_) => loop {
                        match clone.comp_channel.poll() {
                            Ok(None) => break,
                            Ok(Some(socket)) => {
                                socket.notify().unwrap();
                                socket.poll_cq();
                                let _ = Arc::into_raw(socket);
                            }
                            Err(err) => {
                                tracing::error!("comp channel poll: {:?}", err);
                                break;
                            }
                        }
                    },
                    Err(err) => {
                        tracing::error!("comp channel poll error: {:?}", err)
                    }
                }
            }
        });
    }

    pub fn stop_and_join(&self) {
        self.stopping.store(true, Ordering::Release);

        let mut thread = self.thread.lock().unwrap();
        if let Some(t) = thread.take() {
            t.join().unwrap();
        }
    }
}

impl Drop for Card {
    fn drop(&mut self) {
        self.stop_and_join();
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
