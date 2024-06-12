use std::sync::Arc;

use crate::*;

#[derive(Debug)]
pub struct Cards {
    cards: Vec<Arc<Card>>,
    device_list: ibv::DeviceList,
}

impl Cards {
    pub fn open() -> Result<Arc<Self>> {
        let mut cards = vec![];

        let device_list = ibv::DeviceList::available()?;
        for device in device_list.as_ref() {
            let card = Card::open(device)?;
            cards.push(card);
        }

        Ok(Arc::new(Self { cards, device_list }))
    }

    pub fn device_list(&self) -> &ibv::DeviceList {
        &self.device_list
    }
}

impl std::ops::Deref for Cards {
    type Target = [Arc<Card>];

    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cards() {
        let cards = Cards::open().unwrap();
        println!("{:#?}", cards);
        println!("{:#?}", cards.device_list());
    }
}
