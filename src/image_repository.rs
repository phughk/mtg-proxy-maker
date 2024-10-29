use crate::deck::PickedCard;
use crate::scryfall_client::ScryfallClient;
use reqwest::blocking::Response;
use std::fmt::{Display, Formatter};
use std::fs::{create_dir_all, File};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_BACK_FILENAME: &str = "000_BACK.jpg";
const DEFAULT_BACK_CONTENTS: &[u8] = include_bytes!("card_back_normal.jpg");

pub struct ImageRepository<'a> {
    path: &'a Path,
    client: ScryfallClient,
    default_back: PathBuf,
}

impl<'a> ImageRepository<'a> {
    pub fn new(
        path: &'a Path,
        default_back: Option<&'a Path>,
    ) -> Result<ImageRepository<'a>, String> {
        if !path.exists() {
            create_dir_all(path).map_err(|e| format!("Could not create directory: {}", e))?;
        }

        match path.is_dir() {
            true => {
                let client = ScryfallClient::new();
                let default_back = match default_back {
                    None => path.join(DEFAULT_BACK_FILENAME),
                    Some(s) => s.to_path_buf(),
                };
                if !default_back.exists() {
                    // let resp = client.get_image("m21", "176", true);
                    let mut default_back_file =
                        File::create(&default_back).expect("Unable to create default back file");
                    default_back_file
                        .write_all(DEFAULT_BACK_CONTENTS)
                        .expect("Unable to write to default back file");
                };
                Ok(ImageRepository {
                    path,
                    client,
                    default_back,
                })
            }
            false => Err("Provided path was not a directory".to_string()),
        }
    }

    /// Return 2 files for requested image (foreground, background), or error if no such card
    pub fn get_image(&self, card: &PickedCard) -> Result<(File, File), String> {
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
                let back = File::open(self.default_back.clone()).map_err(|e| {
                    format!(
                        "Unable to open back image for '{}' {} {}",
                        card.name, card.set_code, card.collector_number
                    )
                })?;
                Ok((front, back))
            }
        } else {
            let mut front =
                File::create(&front_path).expect("Expected to be able to create front file");
            if card.double_sided {
                let mut back =
                    File::create(&back_path).expect("Expected to be able to create back file");
                match self.retrieve_images_from_scryfall(card, &mut front, Some(&mut back)) {
                    Ok(_) => {}
                    Err(ImageRequestError::NotDoubleSided) => {
                        panic!(
                            "You fucked up card {} set {} coll {}",
                            card.name, card.set_code, card.collector_number
                        )
                    }
                    Err(e) => return Err(format!("{:?}", e)),
                }
            } else {
                match self.retrieve_images_from_scryfall(card, &mut front, None) {
                    Ok(_) => {}
                    Err(ImageRequestError::NotDoubleSided) => {
                        panic!("You fucked up but in yellow; card {} set {} coll {}", card.name, card.set_code, card.collector_number)
                    }
                    Err(e) => return Err(format!("{:?}", e.to_string())),
                }
            }
            // Now we re-open because writing moves the pointer
            let front = File::open(front_path).expect("Expected front image file to open");
            let back = match card.double_sided {
                true => File::open(back_path).expect("Expected back image file to open"),
                false => {
                    File::open(self.default_back.clone()).expect("Expected back image file to open")
                }
            };
            Ok((front, back))
        }
    }

    fn retrieve_images_from_scryfall(
        &self,
        card: &PickedCard,
        front: &mut File,
        back: Option<&mut File>,
    ) -> Result<(), ImageRequestError> {
        // TODO check if card downloaded
        // TODO check if card has face=back not 422 - https://scryfall.com/docs/api/cards/collector
        // TODO clap args
        // TODO rename front cards to have FRONT; BACK should be generic back (copy) or actual
        let resp = self
            .client
            .get_image(&card.set_code, &card.collector_number, false);

        save_image_response_to_file(resp, front)?;

        if let Some(back) = back {
            let resp = self
                .client
                .get_image(&card.set_code, &card.collector_number, true);

            save_image_response_to_file(resp, back)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ImageRequestError {
    NotDoubleSided,
    OtherStatus(u16, String),
    Other(String),
}

impl Display for ImageRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

fn save_image_response_to_file(resp: Response, file: &mut File) -> Result<(), ImageRequestError> {
    match resp.status().as_u16() {
        200 => {
            io::copy(&mut resp.bytes().unwrap().as_ref(), file).unwrap();
            Ok(())
        }
        422 => Err(ImageRequestError::NotDoubleSided),
        e => Err(ImageRequestError::OtherStatus(e, resp.text().unwrap())),
    }
}

#[cfg(test)]
mod test {
    use crate::data_repository::DataRepository;
    use crate::deck::{process_input, MAINBOARD};
    use crate::image_repository::ImageRepository;
    use std::fs::File;
    use std::io::Cursor;
    use std::path::Path;

    #[test]
    pub fn test() {
        let input = "1 [MH3:246] Pinnacle Monk";
        let deck = process_input(Cursor::new(input)).unwrap();
        let data_repo = DataRepository::new(Path::new("data_repository")).unwrap();
        data_repo.delete("Pinnacle Monk");
        let deck = deck.as_hydrated(&data_repo);
        let card = deck.cards.get(MAINBOARD).unwrap().first().unwrap();
        assert!(card.double_sided);
        assert_eq!(card.set_code, "mh3");
        assert_eq!(card.collector_number, "246");
        assert_eq!(card.name, "Pinnacle Monk // Mystic Peak");
        assert_eq!(card.quantity, 1);
        let deck = deck.as_picked();
        let card = deck.cards.get(MAINBOARD).unwrap().first().unwrap();
        assert!(card.double_sided);
        assert_eq!(card.set_code, "mh3");
        assert_eq!(card.collector_number, "246");
        assert_eq!(card.name, "Pinnacle Monk // Mystic Peak");
        assert_eq!(card.quantity, 1);
        let img_repo = ImageRepository::new(Path::new("data_repository"), None).unwrap();
        let mut front = File::create(Path::new("Test_Temp_Front.jpg")).unwrap();
        let mut back = File::create(Path::new("Test_Temp_Back.jpg")).unwrap();
        img_repo
            .retrieve_images_from_scryfall(&card, &mut front, Some(&mut back))
            .unwrap()
    }

    #[test]
    pub fn adventure_cards_are_not_double_sided() {
        let input = "1 [ELD:39] Brazen Borrower";
        let deck = process_input(Cursor::new(input)).unwrap();
        let data_repo = DataRepository::new(Path::new("data_repository")).unwrap();
        data_repo.delete("Brazen Borrower");
        let deck = deck.as_hydrated(&data_repo);
        let card = deck.cards.get(MAINBOARD).unwrap().first().unwrap();
        assert!(!card.double_sided);
        assert_eq!(card.set_code, "eld");
        assert_eq!(card.collector_number, "39");
        assert_eq!(card.name, "Brazen Borrower // Petty Theft");
        assert_eq!(card.quantity, 1);
        let deck = deck.as_picked();
        let card = deck.cards.get(MAINBOARD).unwrap().first().unwrap();
        assert!(!card.double_sided);
        assert_eq!(card.set_code, "eld");
        assert_eq!(card.collector_number, "39");
        assert_eq!(card.name, "Brazen Borrower // Petty Theft");
        assert_eq!(card.quantity, 1);
        let img_repo = ImageRepository::new(Path::new("data_repository"), None).unwrap();
        let mut front = File::create(Path::new("Test_Temp_Front.jpg")).unwrap();
        img_repo
            .retrieve_images_from_scryfall(&card, &mut front, None)
            .unwrap()
    }
}
