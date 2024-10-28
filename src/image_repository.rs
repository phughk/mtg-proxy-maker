use crate::deck::Card;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::StatusCode;
use std::fs::File;
use std::io;
use std::path::Path;

const DEFAULT_BACK_FILENAME: &str = "000_BACK.jpg";

pub struct ImageRepository<'a> {
    path: &'a Path,
    client: Client,
    default_back: &'a Path,
}

impl ImageRepository {
    pub fn new(path: &Path, default_back: Option<&Path>) -> Result<ImageRepository, &'static str> {
        match path.is_dir() {
            true => {
                let client = Client::new();
                let default_back = default_back.unwrap_or(path.join(default_back).as_path());
                Ok(ImageRepository { path, client, default_back })
            }
            false => Err("Provided path was not a directory")
        }
    }

    /// Return 2 files for requested image (foreground, background), or error if no such card
    pub fn get_image(&self, card: &Card) -> Result<(File, File), &'static str> {
        let front_path = self.path.join(format!("{}_{}_front.jpg", card.set_code, card.collector_number));
        let back_name = self.path.join(format!("{}_{}_back.jpg", card.set_code, card.collector_number));
        if front_path.exists() && front_path.is_file() {
            if card.double_sided {
                if back_name.exists() {
                    Ok((File::open(&front_path).unwrap(), File::open(&back_name).unwrap()))
                } else {
                    todo!("retrieve back")
                }
            } else {
                let front = File::open(front_path).unwrap();
                let back = File::open(self.default_back).unwrap();
                Ok((front,back))
            }
        } else {
            
        }
    }

    fn resolve_

    fn retrieve_images_from_scryfall(&self, card: &Card, front: File, back: Option<File>) -> Result<(), &'static str> {
        // TODO check if card downloaded
        // TODO check if card has face=back not 422 - https://scryfall.com/docs/api/cards/collector
        // TODO clap args
        // TODO rename front cards to have FRONT; BACK should be generic back (copy) or actual
        let url = format!(
            "https://api.scryfall.com/cards/{}/{}?format=image",
            card.set_code.to_lowercase(), card.collector_number
        );
        let resp = self.client.get(&url)
            .header(USER_AGENT, "MyMTGApp/1.0") // Customize as needed
            .header(ACCEPT, "application/json")
            .send()?;


        match resp.status() {
            StatusCode::OK => {
                let mut front = File::create(self.path.join(front_name))?;
                let back = front.try_clone().unwrap();
                io::copy(&mut resp.bytes()?.as_ref(), &mut front)?;
                println!("Downloaded image for {}.", card.name);
                Ok((front, back))
            }
            StatusCode::NOT_FOUND => {
                println!("Card not found in Scryfall: {}", card.name);
                println!("Request was: {}", url);
                return Err("not found");
            }
            _ => {
                println!("Error fetching {}: {}", card.name, resp.status());
                println!("{}", resp.text()?);
                return Err("req error");
            }
        }
    }
}