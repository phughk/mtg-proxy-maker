mod data_repository;
pub mod deck;
mod image_repository;
mod scryfall_client;

use crate::data_repository::DataRepository;
use crate::deck::DehydratedCard;
use crate::image_repository::ImageRepository;
use printpdf::{Image, ImageTransform, Mm, PdfDocument};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::StatusCode;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{self, BufRead, BufWriter};
use std::path::Path;

fn fetch_card_image(
    client: &Client,
    card: &DehydratedCard,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO check if card downloaded
    // TODO check if card has face=back not 422 - https://scryfall.com/docs/api/cards/collector
    // TODO clap args
    // TODO rename front cards to have FRONT; BACK should be generic back (copy) or actual
    let url = format!(
        "https://api.scryfall.com/cards/{}/{}?format=image",
        card.set_code
            .clone()
            .expect("Expected card to have a set code when fetching")
            .to_lowercase(),
        card.collector_number
            .clone()
            .expect("Expected card to have a collector number when fetching")
            .to_string()
    );
    let resp = client
        .get(&url)
        .header(USER_AGENT, "MyMTGApp/1.0") // Customize as needed
        .header(ACCEPT, "application/json")
        .send()?;

    match resp.status() {
        StatusCode::OK => {
            let mut file = File::create(format!(
                "{}_{}.jpg",
                card.set_code
                    .clone()
                    .expect("Expected card to have a set code when fetching")
                    .to_lowercase(),
                card.collector_number
                    .clone()
                    .expect("Expected card to have a collector number when fetching")
                    .to_string()
            ))?;
            io::copy(&mut resp.bytes()?.as_ref(), &mut file)?;
            println!("Downloaded image for {}.", card.name);
        }
        StatusCode::NOT_FOUND => {
            println!("Card not found in Scryfall: {}", card.name);
            println!("Request was: {}", url);
            return Err(Box::new(BlaError::Generic("not found".to_string())));
        }
        _ => {
            println!("Error fetching {}: {}", card.name, resp.status());
            println!("{}", resp.text()?);
            return Err(Box::new(BlaError::Generic("req error".to_string())));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum BlaError {
    Generic(String),
}

impl Display for BlaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let BlaError::Generic(s) = self {
            f.write_str(format!("Failed BlaError - {}", s).as_str())
        } else {
            panic!()
        }
    }
}

impl std::error::Error for BlaError {}

fn process_dck_file(file_path: &str, pdf_file_path: &str) -> Result<(), String> {
    let file = File::open(file_path)
        .map_err(|e| format!("Could not open data file: {}", e.to_string()))?;
    let deck = crate::deck::process_input(file).unwrap();
    let data_repo = DataRepository::new(Path::new("data_repository")).unwrap();
    let deck = deck.as_hydrated(&data_repo);
    let deck = deck.as_picked();
    let image_repo = ImageRepository::new(Path::new("image_repository"), None)
        .expect("Expected image repository constructor to work");
    let (doc, page1, layer1) = PdfDocument::new("Proxy Deck", Mm(210.0), Mm(297.0), "Layer 1");
    for (_board, cards) in &deck.cards {
        for card in cards {
            let current_layer = doc.get_page(page1).get_layer(layer1);
            let (mut front, mut back) = image_repo.get_image(card).unwrap();
            let front_decoder =
                printpdf::image_crate::codecs::jpeg::JpegDecoder::new(&mut front).unwrap();
            let front_img = Image::try_from(front_decoder).unwrap();
            let back_decorder =
                printpdf::image_crate::codecs::jpeg::JpegDecoder::new(&mut back).unwrap();
            let back_img = Image::try_from(back_decorder).unwrap();
            front_img.add_to_layer(
                current_layer.clone(),
                ImageTransform {
                    translate_x: None,
                    translate_y: None,
                    rotate: None,
                    scale_x: None,
                    scale_y: None,
                    dpi: None,
                },
            );
            back_img.add_to_layer(
                current_layer,
                ImageTransform {
                    translate_x: None,
                    translate_y: None,
                    rotate: None,
                    scale_x: None,
                    scale_y: None,
                    dpi: None,
                },
            );
        }
    }
    doc.save(&mut BufWriter::new(File::create(pdf_file_path).unwrap()))
        .unwrap();

    Ok(())
}

fn main() {
    let file_path =
        "/Users/hugh/Downloads/mtg-cube-project-halloween/CalebGannonsPoweredSynergyCube.dck";
    if let Err(e) = process_dck_file(file_path, "executable-output.pdf") {
        eprintln!("Error processing file: {}", e);
    }
}
