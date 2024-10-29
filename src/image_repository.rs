use crate::deck::PickedCard;
use crate::scryfall_client::ScryfallClient;
use reqwest::StatusCode;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

const DEFAULT_BACK_FILENAME: &str = "000_BACK.jpg";

pub struct ImageRepository<'a> {
    path: &'a Path,
    client: ScryfallClient,
    default_back: PathBuf,
}

impl<'a> ImageRepository<'a> {
    pub fn new(
        path: &'a Path,
        default_back: Option<&'a Path>,
    ) -> Result<ImageRepository<'a>, &'static str> {
        match path.is_dir() {
            true => {
                let client = ScryfallClient::new();
                let default_back = match default_back {
                    None => path.join(DEFAULT_BACK_FILENAME),
                    Some(s) => s.to_path_buf(),
                };
                Ok(ImageRepository {
                    path,
                    client,
                    default_back,
                })
            }
            false => Err("Provided path was not a directory"),
        }
    }

    /// Return 2 files for requested image (foreground, background), or error if no such card
    pub fn get_image(&self, card: &PickedCard) -> Result<(File, File), &'static str> {
        let front_path = self.path.join(format!(
            "{}_{}_front.jpg",
            card.set_code, card.collector_number
        ));
        let back_path = self.path.join(format!(
            "{}_{}_back.jpg",
            card.set_code, card.collector_number
        ));
        if front_path.exists() && front_path.is_file() {
            if card.double_sided {
                if back_path.exists() {
                    Ok((
                        File::open(&front_path).expect("Expected front image file to open"),
                        File::open(&back_path).expect("Expected back image file to open"),
                    ))
                } else {
                    todo!("retrieve back")
                }
            } else {
                let front = File::open(front_path).expect("Expected front image file to open");
                let back = File::open(self.default_back.clone()).expect("Expected back image file to open");
                Ok((front, back))
            }
        } else {
            let mut front = File::create(front_path).expect("Expected to be able to create front file");
            if card.double_sided {
                let back = File::create(back_path).expect("Expected to be able to create back file");
                self.retrieve_images_from_scryfall(card, &mut front, Some(&back))?;
                Ok((front, back))
            } else {
                self.retrieve_images_from_scryfall(card, &mut front, None)?;
                let back = File::open(self.default_back.clone()).expect("Expected back image file to open");
                Ok((front, back))
            }
        }
    }

    fn retrieve_images_from_scryfall(
        &self,
        card: &PickedCard,
        front: &mut File,
        back: Option<&File>,
    ) -> Result<(), &'static str> {
        // TODO check if card downloaded
        // TODO check if card has face=back not 422 - https://scryfall.com/docs/api/cards/collector
        // TODO clap args
        // TODO rename front cards to have FRONT; BACK should be generic back (copy) or actual
        let resp = self
            .client
            .get_image(&card.set_code, &card.collector_number);

        match resp.status() {
            StatusCode::OK => {
                io::copy(&mut resp.bytes().unwrap().as_ref(), front).unwrap();
                println!("Downloaded image for {}.", card.name);
                Ok(())
            }
            StatusCode::NOT_FOUND => {
                println!("Card not found in Scryfall: {}", card.name);
                Err("not found")
            }
            _ => {
                println!("Error fetching {}: {}", card.name, resp.status());
                println!("{}", resp.text().unwrap());
                Err("req error")
            }
        }
    }
}
