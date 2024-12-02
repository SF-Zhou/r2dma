use crate::ibv::{self, verbs::*};
use crate::Result;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CardIndex(pub usize);

/// Represents an RDMA card with its associated context, protection domain, port attributes, and GID.
#[derive(Debug)]
pub struct Card {
    // The index of card in list.
    _index: CardIndex,
    /// The protection domain associated with the card.
    _pd: ibv::ProtectionDomain,
    /// The context associated with the card.
    context: ibv::Context,
    /// The port attributes of the card.
    _port_attr: ibv_port_attr,
    /// The GID (Global Identifier) of the card.
    _gid: ibv_gid,
}

impl Card {
    pub fn open(index: CardIndex, device: &ibv::Device) -> Result<Self> {
        let context = ibv::Context::create(device)?;
        let pd = ibv::ProtectionDomain::create(&context)?;

        let port_attr = context.query_port(1)?;
        let gid = context.query_gid(1, 1)?;

        Ok(Self {
            _index: index,
            _pd: pd,
            context,
            _port_attr: port_attr,
            _gid: gid,
        })
    }

    pub fn name(&self) -> Cow<str> {
        self.context.device().name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ib_device() {
        let device_list = ibv::DeviceList::available().unwrap();
        let first_device = device_list.first().unwrap();
        let card = Card::open(CardIndex(0), first_device).unwrap();
        println!("{}: {:#?}", card.name(), card);
    }
}
