use std::{collections::HashMap, sync::Arc};

use crate::*;

#[derive(Debug)]
pub struct Cards {
    cards: HashMap<String, Arc<Card>>,
}

impl Cards {
    pub fn open() -> Result<Arc<Self>> {
        let mut cards = HashMap::new();

        let device_list = ibv::DeviceList::cached();
        for device in device_list.as_ref() {
            let card = Card::open(device)?;
            cards.insert(card.name().to_string(), card);
        }

        Ok(Arc::new(Self { cards }))
    }

    pub fn get(&self, name: Option<&str>) -> Result<Arc<Card>> {
        match name {
            Some(name) => self
                .cards
                .get(name)
                .cloned()
                .ok_or(Error::new(ErrorKind::IBDeviceNotFound)),
            None => self
                .cards
                .iter()
                .next()
                .map(|p| p.1.clone())
                .ok_or(Error::new(ErrorKind::IBDeviceNotFound)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cards() {
        let cards = Cards::open().unwrap();
        println!("{:#?}", cards);

        let card = cards.get(None).unwrap();
        let _ = cards.get(Some(&card.name())).unwrap();
        let _ = cards.get(Some("")).unwrap_err();
    }
}
