mod data_repository;
pub mod deck;
mod image_repository;
mod scryfall_client;
mod pdf_calc;

use crate::data_repository::DataRepository;
use crate::deck::DehydratedCard;
use crate::image_repository::ImageRepository;
use crate::pdf_calc::{calculate_dpi_image, grid_translator, PAGE_HEIGHT_A4, PAGE_WIDTH_A4};
use printpdf::{Image, ImageTransform, PdfDocument, PdfLayerIndex, PdfPageIndex};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::StatusCode;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{self, BufRead, BufWriter};
use std::path::Path;
use std::sync::atomic::{AtomicU16, Ordering};

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
    let layer = "Layer 1";
    let (doc, front_page, front_layer) = PdfDocument::new("Proxy Deck", PAGE_WIDTH_A4, PAGE_HEIGHT_A4, layer);
    let mut counter: Option<AtomicU16> = None;
    let mut index = 0;
    let mut page = 0;
    let (back_page, back_layer) = doc.add_page(PAGE_WIDTH_A4, PAGE_HEIGHT_A4, layer);
    let mut pdf_indexes: ((PdfPageIndex, PdfLayerIndex), (PdfPageIndex, PdfLayerIndex)) = ((front_page, front_layer), (back_page, back_layer));
    let total = deck.cards.iter().map(|(_board, cards)| cards.len()).reduce(|a, b| a + b).unwrap();
    for (_board, cards) in &deck.cards {
        for card in cards {
            println!("[{}/{}] Rendering card {} [{}:{}] to pdf", index, total, card.name, card.set_code, card.collector_number);

            let (new_page, x, y, x_flip) = grid_translator(index);
            if new_page > page {
                let (front_page, front_layer) = doc.add_page(PAGE_WIDTH_A4, PAGE_HEIGHT_A4, layer);
                let (back_page, back_layer) = doc.add_page(PAGE_WIDTH_A4, PAGE_HEIGHT_A4, layer);
                pdf_indexes = ((front_page, front_layer), (back_page, back_layer));
                page = new_page;
            }

            let front_layer_reference = doc.get_page(pdf_indexes.0.0).get_layer(pdf_indexes.0.1);
            let (mut front, mut back) = image_repo.get_image(card).unwrap();
            let front_decoder =
                printpdf::image_crate::codecs::jpeg::JpegDecoder::new(&mut front).unwrap();
            let front_img = Image::try_from(front_decoder).unwrap();

            let back_layer_reference = doc.get_page(pdf_indexes.1.0).get_layer(pdf_indexes.1.1);
            let back_decoder =
                printpdf::image_crate::codecs::jpeg::JpegDecoder::new(&mut back).unwrap();
            let back_img = Image::try_from(back_decoder).unwrap();

            let front_dpi = calculate_dpi_image(&front_img);
            front_img.add_to_layer(
                front_layer_reference,
                ImageTransform {
                    translate_x: Some(x),
                    translate_y: Some(y),
                    rotate: None,
                    scale_x: None,
                    scale_y: None,
                    dpi: Some(front_dpi),
                },
            );

            let back_dpi = calculate_dpi_image(&back_img);
            back_img.add_to_layer(
                back_layer_reference,
                ImageTransform {
                    translate_x: Some(x_flip),
                    translate_y: Some(y),
                    rotate: None,
                    scale_x: None,
                    scale_y: None,
                    dpi: Some(back_dpi),
                },
            );
            if let Some(counter) = counter.as_mut() {
                let val = counter.fetch_sub(1, Ordering::SeqCst);
                if val <= 1 {
                    break;
                }
            }
            index += 1;
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
