use crate::deck;
use crate::deck::{DehydratedCard, HydratedCard};
use crate::scryfall_client::{ScryfallClient, ScryfallSearchResultEntryImageUris};
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone)]
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

    pub fn delete(&self, card: &str) {
        self.db.remove(card.clone()).expect("Failed to remove card");
    }

    pub fn scan_range(&self, prefix: &str) -> BTreeMap<String, CardInfo> {
        let mut results = BTreeMap::new();
        let scan = self.db.scan_prefix(prefix.to_string());
        for res in scan {
            match res {
                Ok((key_ivec, val_ivec)) => {
                    let key = String::from_utf8_lossy(key_ivec.as_ref()).to_string();
                    let val: CardInfo = CardInfo::from(val_ivec);
                    results.insert(key, val);
                }
                Err(e) => {
                    panic!("Scan error: {}", e);
                }
            }
        }
        results
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
                println!("Card variants: {:?}", vars);
                let first = vars
                    .first()
                    .clone()
                    .expect("The search results did not have a first card")
                    .clone();
                let double_sided = first
                    .card_faces
                    .iter()
                    .flat_map(|card_faces_vec| card_faces_vec.iter())
                    .map(|card_face| card_face.image_uris.is_some())
                    .reduce(|a, b| a | b)
                    .unwrap_or(false);
                let variants = vars
                    .into_iter()
                    .filter(|var| var.valid())
                    .map(|var| CardVariant {
                        set: var.set,
                        collector_number: var.collector_number,
                        lang: var.lang,
                        img_url: var.image_uris,
                    })
                    .map(|mut var| {
                        match var.set == "plist" {
                            true => {
                                let mut split_res = var.collector_number.split("-");
                                let set = split_res.next().unwrap();
                                let collector_number = split_res.next().unwrap();
                                assert!(split_res.next().is_none());
                                var.set = set.to_string();
                                var.collector_number = collector_number.to_string();
                                var
                            }
                            false => {
                                var
                            }
                        }
                    })
                    .collect();
                let entry = CardInfo {
                    name: first.name,
                    double_sided,
                    variants,
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
        let first_item = card_info
            .variants
            .first()
            .expect("There were no variants")
            .clone();
        Ok(HydratedCard {
            quantity: card.quantity,
            set_code: card
                .set_code
                .unwrap_or(first_item.set.clone())
                .to_lowercase(),
            collector_number: card
                .collector_number
                .unwrap_or(first_item.collector_number.clone())
                .to_lowercase(),
            name: card_info.name,
            double_sided: card_info.double_sided,
            variants: card_info
                .variants
                .into_iter()
                .map(|cv| deck::CardVariant {
                    set: cv.set.to_lowercase(),
                    collector_number: cv.collector_number.to_lowercase(),
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
    img_url: Option<ScryfallSearchResultEntryImageUris>,
}

impl From<IVec> for CardInfo {
    fn from(value: IVec) -> Self {
        let card_info: CardInfo =
            bincode::deserialize(value.as_ref()).expect("IVec of CardInfo can't be deserialized");
        card_info
    }
}

impl Into<IVec> for CardInfo {
    fn into(self) -> IVec {
        let data = bincode::serialize(&self).expect("IVec of CardInfo can't be serialized");
        IVec::from(data)
    }
}

#[cfg(test)]
mod test {
    use crate::data_repository::DataRepository;
    use crate::deck::DehydratedCard;
    use std::path::Path;

    #[test]
    pub fn test_plist() {
        let card = "Stitcher's Supplier";
        let repo = DataRepository::new(Path::new("data_repository")).unwrap();
        let res = repo.scan_range(&card);
        for (k, v) in res {
            println!("Scan result: {}", k);
            println!("Value: {:?}", v);
        }
    }

    #[test]
    pub fn delete_some_shit() {
        let repo = DataRepository::new(Path::new("data_repository")).unwrap();
        repo.delete("Michiko's Reign of Truth");
        repo.delete("Expansion");
        repo.delete("Expansion // Explosion");
        repo.delete("Kabira Takedown");
        let hydrated = repo.get(DehydratedCard {
            quantity: 1,
            set_code: None,
            collector_number: None,
            name: "Kabira Takedown".to_string(),
            flip_name: None,
            double_sided: None,
        }).unwrap();
        repo.delete("Plateau");
        let hydrated = repo.get(DehydratedCard {
            quantity: 1,
            set_code: None,
            collector_number: None,
            name: "Expansion".to_string(),
            flip_name: None,
            double_sided: None,
        }).unwrap();
        panic!("{:?}", hydrated)
    }
}