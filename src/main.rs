mod data_repository;
pub mod deck;
mod image_repository;
mod scryfall_client;
mod pdf_calc;

use crate::data_repository::DataRepository;
use crate::image_repository::ImageRepository;
use crate::pdf_calc::{calculate_dpi_image, grid_translator, PAGE_HEIGHT_A4, PAGE_WIDTH_A4};
use clap::Parser;
use printpdf::{Image, ImageTransform, PdfDocument, PdfLayerIndex, PdfPageIndex};
use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::{BufRead, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Input file: xmage (dck), forge (dek), mtgo (mtgo), arena (mtga) file
    #[arg(short, long)]
    input_file: String,

    /// Number of cards to process (optional)
    #[arg(short = 'n', long)]
    card_count: Option<u16>,

    /// Path to the image repository
    #[arg(short = 'i', long, default_value = "./data_repository")]
    image_repository: Option<String>,

    /// Path to the data repository
    #[arg(short = 'd', long, default_value = "./image_repository")]
    data_repository: Option<String>,

    /// Output PDF name (defaults to same name as input file)
    #[arg(short = 'o', long)]
    output_pdf_name: Option<String>,
}

fn process_dck_file(file_path: &Path, pdf_file_path: &Path, counter: Option<AtomicU16>) -> Result<(), String> {
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
    let mut index = 0;
    let mut page = 0;
    let (back_page, back_layer) = doc.add_page(PAGE_WIDTH_A4, PAGE_HEIGHT_A4, layer);
    let mut pdf_indexes: ((PdfPageIndex, PdfLayerIndex), (PdfPageIndex, PdfLayerIndex)) = ((front_page, front_layer), (back_page, back_layer));
    let total = deck.cards.iter().map(|(_board, cards)| cards.into_iter().map(|c| c.quantity).reduce(|a, b| a + b).unwrap()).reduce(|a, b| a + b).unwrap();
    for (_board, cards) in &deck.cards {
        for card in cards {
            for _ in 0..card.quantity {
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
                if let Some(counter) = &counter {
                    let val = counter.fetch_sub(1, Ordering::SeqCst);
                    if val <= 1 {
                        break;
                    }
                }
                index += 1;
            }
        }
    }
    doc.save(&mut BufWriter::new(File::create(pdf_file_path).unwrap()))
        .unwrap();

    Ok(())
}

fn main() {
    let args = Args::parse();
    // let file_path =
    //     "/Users/hugh/Downloads/mtg-cube-project-halloween/CalebGannonsPoweredSynergyCube.dck";
    let file_path = Path::new(&args.input_file);
    let output = match &args.output_pdf_name {
        None => file_path.with_extension("pdf"),
        Some(f) => PathBuf::from(Path::new(f))
    };
    let counter = args.card_count.map(|c| AtomicU16::new(c));
    if let Err(e) = process_dck_file(file_path, &output, counter) {
        eprintln!("Error processing file: {}", e);
    }
}
