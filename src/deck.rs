use std::fs::File;

#[derive(Debug)]
pub struct Card {
    pub quantity: u32,
    pub set_code: String,
    pub collector_number: String,
    pub name: String,
    pub double_sided: bool,
}

#[derive(Debug)]
pub struct Deck {
    file: File,
}