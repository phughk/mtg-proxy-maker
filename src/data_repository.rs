use crate::deck;
use crate::deck::{DehydratedCard, HydratedCard};
use crate::scryfall_client::ScryfallClient;
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};
use std::path::Path;

pub struct DataRepository {
    db: Db,
    client: ScryfallClient,
}

impl DataRepository {
    pub fn new(path: &Path) -> Result<DataRepository, ()> {
        let db = sled::open(path).map_err(|e| ())?;
        let client = ScryfallClient::new();
        Ok(DataRepository { db, client })
    }

    pub fn get(&self, card: DehydratedCard) -> Result<HydratedCard, ()> {
        let res = self.db.get(card.name.clone()).map_err(|e| ())?;
        let card_info: Option<CardInfo> = res.map(|ivec| ivec.into());
        let card_info = match card_info {
            None => {
                println!("Cache miss for '{}', requesting scryfall data", card.name);
                let vars = match self.client.get_card_variants(&card.name) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("error: {}", e.error);
                        eprintln!("url: {}", e.url);
                        eprintln!("cause: {}", e.cause);
                        eprintln!("response: {}", e.response);
                        panic!()
                    }
                };
                let first = vars.first().clone().expect("The search results did not have a first card").clone();
                let entry = CardInfo {
                    name: first.name,
                    double_sided: first.card_faces.is_some(),
                    variants: vars
                        .into_iter()
                        .filter(|var| var.valid())
                        .map(|var| CardVariant {
                            set: var.set,
                            collector_number: var.collector_number,
                            lang: var.lang,
                        })
                        .collect(),
                };
                self.db
                    .insert(card.name.clone(), entry.clone())
                    .expect("It should have been possible to insert into db");
                entry
            }
            Some(card_info) => {
                println!("Cache hit for '{}'", card.name);
                card_info
            }
        };
        let first_item = card_info.variants.first().expect("There were no variants").clone();
        Ok(HydratedCard {
            quantity: card.quantity,
            set_code: card.set_code.unwrap_or(first_item.set.clone()),
            collector_number: card
                .collector_number
                .unwrap_or(first_item.collector_number.clone()),
            name: card_info.name,
            double_sided: false,
            variants: card_info
                .variants
                .into_iter()
                .map(|cv| deck::CardVariant {
                    set: cv.set,
                    collector_number: cv.collector_number,
                })
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CardInfo {
    name: String,
    double_sided: bool,
    variants: Vec<CardVariant>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CardVariant {
    set: String,
    collector_number: String,
    lang: String,
}

impl From<IVec> for CardInfo {
    fn from(value: IVec) -> Self {
        let card_info: CardInfo = bincode::deserialize(value.as_ref()).expect("IVec of CardInfo can't be deserialized");
        card_info
    }
}

impl Into<IVec> for CardInfo {
    fn into(self) -> IVec {
        let data = bincode::serialize(&self).expect("IVec of CardInfo can't be serialized");
        IVec::from(data)
    }
}
