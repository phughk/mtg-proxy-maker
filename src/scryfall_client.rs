use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;

const PROXY_MAKER_AGENT: &str = "MyMTGApp/1.0";
const BASE_URL: &str = "https://api.scryfall.com";

#[derive(Clone, Debug)]
pub struct ScryfallClient {
    client: Client,
}

impl ScryfallClient {
    pub fn new() -> Self {
        ScryfallClient {
            client: Client::new(),
        }
    }

    pub fn get_image(&self, set: &str, collector_number: &str, back: bool) -> Response {
        let back_str = match back {
            true => "&face=back",
            false => "",
        };
        let url = format!(
            "{BASE_URL}/cards/{}/{}?format=image{}",
            set, collector_number, back_str
        );
        println!("url: {}", url);
        // Scryfall requests that we are polite
        sleep(Duration::from_millis(100));
        let resp = self
            .client
            .get(&url)
            .header(USER_AGENT, PROXY_MAKER_AGENT) // Customize as needed
            .header(ACCEPT, "*/*")
            .send()
            .expect("Expected get image scryfall response to work");
        resp
    }

    pub fn get_card_variants(
        &self,
        name: &str,
    ) -> Result<Vec<ScryfallSearchResultEntry>, SearchCardError> {
        let mut results = vec![];
        let mut has_more = true;
        let mut page = 1;
        while has_more {
            let url = format!(
                "{BASE_URL}/cards/search?q=\"{}\"&page={}&unique=prints",
                name, page
            );
            let mut resp = self
                .client
                .get(&url)
                .header(USER_AGENT, PROXY_MAKER_AGENT)
                .header(ACCEPT, "application/json")
                .send()
                .expect("Expected search result to work");
            let mut data = String::new();
            resp.read_to_string(&mut data).unwrap();
            let search_result: ScryfallSearchResult = serde_json::from_str(&data).map_err(|e| {
                let value: Value = serde_json::from_str(&data).unwrap();
                let data = serde_json::to_string_pretty(&value).unwrap();
                SearchCardError {
                    url,
                    error: format!("{:?}", e),
                    cause: "Expected json to deserialise".to_string(),
                    response: data,
                }
            })?;
            has_more = search_result.has_more;
            results.extend(search_result.data.into_iter().filter(|entry| {
                match entry.name.contains("//") {
                    true => {
                        // Split card, compare both names
                        let mut split = entry.name.split("//");
                        let first = split.next().unwrap();
                        let second = split.next().unwrap();
                        assert!(split.next().is_none());
                        println!("first='{}' second='{}' card='{}'", first, second, name);
                        first.trim().to_lowercase() == name.trim().to_lowercase() || second.trim().to_lowercase() == name.trim().to_lowercase()
                    }
                    false => {
                        entry.name.trim().to_lowercase() == name.trim().to_lowercase()
                    }
                }
            }
            ));
            page += 1;
        }
        Ok(results)
    }
}

#[derive(Debug)]
pub struct SearchCardError {
    pub url: String,
    pub error: String,
    pub cause: String,
    pub response: String,
}

#[derive(Deserialize)]
pub struct ScryfallSearchResult {
    pub object: String,
    pub total_cards: u32,
    pub has_more: bool,
    pub data: Vec<ScryfallSearchResultEntry>,
}

#[derive(Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct ScryfallSearchResultEntry {
    pub name: String,
    pub lang: String,
    pub set: String,
    pub collector_number: String,
    pub image_uris: Option<ScryfallSearchResultEntryImageUris>,
    pub card_faces: Option<Vec<ScryfallSearchResultEntryCardFace>>,
}

impl ScryfallSearchResultEntry {
    /// Returns true for cards that have an image
    /// Invalid for digital cards that do not have an image for whatever bizarre reason
    pub fn valid(&self) -> bool {
        let has_image = self.image_uris.is_some();
        let has_faces = match &self.card_faces {
            None => false,
            Some(faces) => {
                let has_faces: Vec<_> = faces.iter().map(|f| f.image_uris.is_some()).collect();
                has_faces.into_iter().reduce(|a, b| a && b).unwrap_or(false)
            }
        };
        has_image || has_faces
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct ScryfallSearchResultEntryImageUris {
    pub small: String,
    pub normal: String,
    pub large: String,
    pub png: String,
}

#[derive(Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct ScryfallSearchResultEntryCardFace {
    pub name: String,
    pub image_uris: Option<ScryfallSearchResultEntryImageUris>,
}

#[cfg(test)]
mod test {
    use crate::scryfall_client::ScryfallClient;

    #[test]
    pub fn test_copies() {
        let client = ScryfallClient::new();
        let copies = client.get_card_variants("llanowar elves").unwrap();
        assert_eq!(copies.len(), 44);
    }

    #[test]
    pub fn test_subset_name() {
        let client = ScryfallClient::new();
        // let copies = client.get_card_variants("Plateau").unwrap();
        // let copies = client.get_card_variants("Michiko's Reign of Truth").unwrap();
        let copies = client.get_card_variants("Expansion").unwrap();
        let strang: Vec<_> = copies.iter().map(|e| format!("{:?}", e)).collect();
        let strang = strang.join("\n");
        panic!("Cards:\n{}", strang);
    }
}
