use crate::data_repository::DataRepository;
use regex::Regex;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read};

pub const MAINBOARD: &str = "Mainboard";
pub const SIDEBOARD: &str = "Sideboard";

/// Dehydrated card is a card processed from file input, but without scryfall card information
#[derive(Debug)]
pub struct DehydratedCard {
    pub quantity: u32,
    pub set_code: Option<String>,
    pub collector_number: Option<String>,
    pub name: String,
    pub double_sided: Option<bool>,
}

/// HydratedCard is a card that has been filled with Scryfall information
#[derive(Debug)]
pub struct HydratedCard {
    pub quantity: u32,
    pub name: String,
    pub set_code: String,
    pub collector_number: String,
    pub double_sided: bool,
    pub variants: Vec<CardVariant>,
}

#[derive(Debug)]
pub struct CardVariant {
    pub set: String,
    pub collector_number: String,
}

/// PickedCard is a card that has a selected style
#[derive(Debug)]
pub struct PickedCard {
    pub quantity: u32,
    pub set_code: String,
    pub collector_number: String,
    pub name: String,
    pub double_sided: bool,
}

#[derive(Debug)]
pub struct DehydratedDeck {
    pub cards: BTreeMap<String, Vec<DehydratedCard>>,
}

impl DehydratedDeck {
    pub fn as_hydrated(self, data_repository: &DataRepository) -> HydratedDeck {
        let mut hydrated_cards = BTreeMap::new();
        for (section, dehydrated_cards) in self.cards {
            let mut cards = vec![];
            for dehydrated_card in dehydrated_cards {
                let hydrated_card = data_repository
                    .get(dehydrated_card)
                    .expect("Expected the data repository to have data");
                cards.push(hydrated_card);
            }
            hydrated_cards.insert(section, cards);
        }
        HydratedDeck {
            cards: hydrated_cards,
        }
    }
}

pub struct HydratedDeck {
    pub cards: BTreeMap<String, Vec<HydratedCard>>,
}

impl HydratedDeck {
    pub fn as_picked(self) -> PickedDeck {
        let mut picked_cards = BTreeMap::new();
        for (section, hydrated_cards) in self.cards {
            let mut cards = vec![];
            for card in hydrated_cards {
                // We need to validate that the picked card is actually one that is available
                // because input data can be mental (both auto-generated from xmage/cubecobra and
                // user-modified)
                let mut valid = false;
                for candidate in &card.variants {
                    if card.set_code == candidate.set
                        && card.collector_number == candidate.collector_number
                    {
                        valid = true;
                        break;
                    }
                }
                let first_variant = card.variants.first().unwrap();
                let (set, num) = match valid {
                    true => (card.set_code, card.collector_number),
                    false => (
                        first_variant.set.clone(),
                        first_variant.collector_number.clone(),
                    ),
                };
                cards.push(PickedCard {
                    quantity: card.quantity,
                    set_code: set,
                    collector_number: num,
                    name: card.name,
                    double_sided: card.double_sided,
                })
            }
            picked_cards.insert(section, cards);
        }
        PickedDeck {
            cards: picked_cards,
        }
    }
}

pub struct PickedDeck {
    pub cards: BTreeMap<String, Vec<PickedCard>>,
}

pub fn process_input<READ: Read>(read: READ) -> Result<DehydratedDeck, &'static str> {
    let mut buf_read = BufReader::new(read);
    let lines: Vec<String> = buf_read.lines().flatten().collect();
    if let Ok(deck) = try_xmage(&lines) {
        return Ok(deck);
    }
    if let Ok(deck) = try_mtgo(&lines) {
        return Ok(deck);
    }
    if let Ok(deck) = try_mtga(&lines) {
        return Ok(deck);
    }
    Err("Could not detect deck format")
}

fn try_xmage(lines: &[String]) -> Result<DehydratedDeck, ()> {
    // Space separated
    // <quantity> [<SET>] <Name>
    // Set is format
    // SET:SEQUENCE
    // Sequence can be either number, or special, if set is PLIST
    // Compile the regex pattern
    let re = Regex::new(
        r"(?x)
        ^(SB:\s*)?          # Optional 'SB:' prefix with optional space
        (\d+)               # Quantity at the start of the line
        \ \[([A-Z0-9]+):([\w-]+)\] # Set code and card number
        \ (.+)$             # Card name (can contain special characters)
    ",
    )
    .unwrap();

    let mut deck = DehydratedDeck {
        cards: BTreeMap::new(),
    };
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let card = try_xmage_line(line, &re)
            .map_err(|e| format!("Failed for line `{}`", line))
            .unwrap();
        match card {
            XMageCard::Mainboard(c) => {
                deck.cards
                    .entry(MAINBOARD.to_string())
                    .or_insert(vec![])
                    .push(c);
            }
            XMageCard::Sideboard(c) => {
                deck.cards
                    .entry(SIDEBOARD.to_string())
                    .or_insert(vec![])
                    .push(c);
            }
        }
    }
    Ok(deck)
}

fn try_xmage_line(line: &str, re: &Regex) -> Result<XMageCard, ()> {
    match re.captures(line) {
        None => Err(()),
        Some(c) => {
            let sideboard = c.get(1).is_some();
            let quantity = &c[2];
            let set_code = &c[3];
            let collector_number = &c[4];
            let name = c[5].to_string();
            let card = DehydratedCard {
                quantity: quantity.parse().unwrap(),
                set_code: Some(set_code.to_string()),
                collector_number: Some(collector_number.to_string()),
                name,
                double_sided: None,
            };
            match sideboard {
                true => Ok(XMageCard::Sideboard(card)),
                false => Ok(XMageCard::Mainboard(card)),
            }
        }
    }
}

enum XMageCard {
    Mainboard(DehydratedCard),
    Sideboard(DehydratedCard),
}

/// Try processing the line of input as Magic the Gathering: Online format
fn try_mtgo(lines: &[String]) -> Result<DehydratedDeck, ()> {
    Err(())
}

/// Try processing the line of input as Magic the Gathering: Arena format
fn try_mtga(lines: &[String]) -> Result<DehydratedDeck, ()> {
    Err(())
}

#[cfg(test)]
mod test {
    use crate::deck::{process_input, MAINBOARD};
    use std::io::Cursor;

    #[test]
    pub fn test_xmage() {
        let input = r#"
1 [MID:163] Tavern Ruffian
1 [ELD:287] Murderous Rider
1 [GRN:153] Aurelia, Exemplar of Justice
1 [3ED:9] Circle of Protection: Black
1 [AKH:194] Ahn-Crop Champion
1 [PLST:M19-121] Stitcher's Supplier
SB: 1 [AKH:194] Ahn-Crop Champion
        "#;
        let processed = process_input(Cursor::new(input)).unwrap();
        let cards = processed.cards.get(MAINBOARD).unwrap();
        assert_eq!(cards.len(), 6)
    }

    #[test]
    pub fn test_mtgo() {
        let input = r#"
1 Tavern Ruffian
1 Murderous Rider
1 Aurelia, Exemplar of Justice
1 Circle of Protection: Black
1 Ahn-Crop Champion

1 Ahn-Crop Champion
        "#;
    }

    #[test]
    pub fn test_mtga() {
        let input = r#"
1 Tavern Ruffian (MID) 163
1 Murderous Rider (ELD) 287
1 Aurelia, Exemplar of Justice (GRN) 153
1 Circle of Protection: Black (3ED) 9
1 Ahn-Crop Champion (AKH) 194

1 Ahn-Crop Champion (AKH) 194
        "#;
    }
}
