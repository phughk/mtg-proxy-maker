pub mod deck;
mod image_repository;

use crate::deck::Card;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::StatusCode;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{self, BufRead};

fn parse_dck_line(line: &str) -> Option<Card> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let quantity: u32 = parts[0].parse().ok()?;
    let set_and_seq = parts[1].trim_matches(|c| c == '[' || c == ']');
    let set_parts: Vec<&str> = set_and_seq.split(':').collect();

    if set_parts.len() != 2 {
        return None;
    }

    let set_code = set_parts[0].to_string();
    let collector_number = set_parts[1].to_string();
    let name = parts[2..].join(" ");
    Some(Card {
        quantity,
        set_code,
        collector_number,
        name,
    })
}

fn fetch_card_image(client: &Client, card: &Card) -> Result<(), Box<dyn std::error::Error>> {
    // TODO check if card downloaded
    // TODO check if card has face=back not 422 - https://scryfall.com/docs/api/cards/collector
    // TODO clap args
    // TODO rename front cards to have FRONT; BACK should be generic back (copy) or actual
    let url = format!(
        "https://api.scryfall.com/cards/{}/{}?format=image",
        card.set_code.to_lowercase(), card.collector_number
    );
    let resp = client.get(&url)
        .header(USER_AGENT, "MyMTGApp/1.0") // Customize as needed
        .header(ACCEPT, "application/json")
        .send()?;

    match resp.status() {
        StatusCode::OK => {
            let mut file = File::create(format!("{}_{}.jpg", card.set_code, card.collector_number))?;
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

fn process_dck_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if let Some(card) = parse_dck_line(&line) {
            fetch_card_image(&client, &card)?;
        }
    }

    Ok(())
}

fn main() {
    let file_path = "/Users/hugh/Downloads/mtg-cube-project-halloween/HalloweenThemedCube.dck";
    if let Err(e) = process_dck_file(file_path) {
        eprintln!("Error processing file: {}", e);
    }
}
