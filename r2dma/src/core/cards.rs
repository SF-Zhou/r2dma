use super::{Card, CardIndex};
use crate::{ibv, Result};
use std::{ops::Deref, sync::Arc};

/// Represents a collection of InfiniBand cards and their associated device list.
#[derive(Debug)]
pub struct Cards {
    /// A vector of `Card` instances.
    cards: Vec<Card>,
    /// The list of available InfiniBand devices.
    _device_list: ibv::DeviceList,
}

impl Cards {
    pub fn open() -> Result<Arc<Self>> {
        let mut cards = vec![];

        let device_list = ibv::DeviceList::available()?;
        for (idx, device) in device_list.iter().enumerate() {
            let card = Card::open(CardIndex(idx), device)?;
            cards.push(card);
        }

        Ok(Arc::new(Self {
            cards,
            _device_list: device_list,
        }))
    }

    pub fn iter(self: &Arc<Self>) -> CardsIterator {
        CardsIterator(CardRef {
            index: CardIndex(0),
            cards: self.clone(),
        })
    }
}

#[derive(Clone)]
pub struct CardRef {
    index: CardIndex,
    cards: Arc<Cards>,
}

impl std::ops::Deref for CardRef {
    type Target = Card;

    fn deref(&self) -> &Self::Target {
        &self.cards.cards[self.index.0]
    }
}

impl std::fmt::Debug for CardRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.deref(), f)
    }
}

pub struct CardsIterator(CardRef);
impl Iterator for CardsIterator {
    type Item = CardRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.index.0 < self.0.cards.cards.len() {
            let ret = self.0.clone();
            self.0.index.0 += 1;
            Some(ret)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cards() {
        let cards = Cards::open().unwrap();
        for card in cards.iter() {
            println!("{:#?}", *card);
        }
    }
}
