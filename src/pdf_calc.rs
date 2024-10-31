use printpdf::{Image, Mm};

pub const CARD_WIDTH: Mm = Mm(63.0);
pub const CARD_HEIGHT: Mm = Mm(88.0);

pub const PAGE_WIDTH_A4: Mm = Mm(210.0);
pub const PAGE_HEIGHT_A4: Mm = Mm(297.0);

///
/// 3 cards wide at
/// A4 is 210mm x 297mm
const WIDTH_OFFSET_MM: Mm = Mm((PAGE_WIDTH_A4.0 - (3f32 * CARD_WIDTH.0)) / 2.0);
const HEIGHT_OFFSET_MM: Mm = Mm((PAGE_HEIGHT_A4.0 - (3f32 * CARD_HEIGHT.0)) / 2.0);

/// Given a card index, return it's position in a pdf
/// (page_number from 0, x position, y position, x position on other side)
pub fn grid_translator(index: usize) -> (usize, Mm, Mm, Mm) {
    let page = index / 9;
    let pos_index = index % 9;
    let grid_x = pos_index % 3;
    let grid_x_flip = 2 - grid_x;
    let grid_x = grid_x as f32;
    let grid_x_flip = grid_x_flip as f32;
    let grid_y = pos_index / 3;
    // Offsets are calculated from left bottom so we need to invert top (0,1 2) becomes (2,1,0)
    let grid_y = 2 - grid_y;
    let grid_y = grid_y as f32;
    let x = Mm(CARD_WIDTH.0 * grid_x);
    let x = Mm(WIDTH_OFFSET_MM.0 + x.0);
    let y = Mm(CARD_HEIGHT.0 * grid_y);
    let y = Mm(HEIGHT_OFFSET_MM.0 + y.0);
    let x_flip = Mm(CARD_WIDTH.0 * grid_x_flip);
    let x_flip = Mm(WIDTH_OFFSET_MM.0 + x_flip.0);
    (page, x, y, x_flip)
}

pub fn calculate_dpi_image(img: &Image) -> f32 {
    calculate_dpi(img.image.width.0, img.image.height.0)
}

/// Given an image width and height, and assuming it is 63mm x 88mm, calculate the DPI
pub fn calculate_dpi(width: usize, height: usize) -> f32 {
    // Convert mm to inches
    let width_in_inches = 63.0 / 25.4;
    let height_in_inches = 88.0 / 25.4;

    // Calculate DPI for width and height
    let dpi_width = width as f32 / width_in_inches;
    let dpi_height = height as f32 / height_in_inches;

    // Return the min DPI
    if dpi_width < dpi_height {
        dpi_width
    } else {
        dpi_height
    }
}

#[cfg(test)]
mod test {
    use crate::data_repository::DataRepository;
    use crate::deck::{DehydratedDeck, MAINBOARD};
    use crate::image_repository::ImageRepository;
    use crate::pdf_calc::{calculate_dpi, grid_translator};
    use printpdf::Image;
    use std::path::Path;

    #[test]
    fn calc_dpi_of_files() {
        let data_repo = DataRepository::new(Path::new("data_repository")).unwrap();
        let image_repo = ImageRepository::new(Path::new("image_repository"), None).unwrap();
        let deck = DehydratedDeck::new_for_test("Colossal Dreadmaw");
        let deck = deck.as_hydrated(&data_repo);
        let mut deck = deck.as_picked();
        let card = deck.cards.get_mut(MAINBOARD).map(|v| v.into_iter().next().unwrap()).unwrap();

        let (mut front, mut back) = image_repo.get_image(card).unwrap();
        let front_decoder =
            printpdf::image_crate::codecs::jpeg::JpegDecoder::new(&mut front).unwrap();
        let front_img = Image::try_from(front_decoder).unwrap();
        let back_decorder =
            printpdf::image_crate::codecs::jpeg::JpegDecoder::new(&mut back).unwrap();
        let back_img = Image::try_from(back_decorder).unwrap();

        let (fw, fh) = (&front_img.image.width.0, &front_img.image.height.0);
        let (bw, bh) = (&back_img.image.width.0, &back_img.image.height.0);
        assert_eq!((fw, fh), (&672usize, &936usize));
        assert_eq!((bw, bh), (&488usize, &680usize));
        let front_dpi = calculate_dpi(*fw, *fh);
        let back_dpi = calculate_dpi(*bw, *bh);
        assert_eq!(front_dpi, 270.54846f32);
        assert_eq!(back_dpi, 196.51096f32);
    }

    #[test]
    pub fn test_grid_translator() {
        let mut results = Vec::new();
        for index in 0..10 {
            let (page, x, y, x_flip) = grid_translator(index);
            results.push(GridTranslateTestCase {
                index,
                page,
                x_offset: x.0,
                y_offset: y.0,
                x_flip_offset: x_flip.0,
            });
        }
        assert_eq!(results, &[
            GridTranslateTestCase {
                index: 0,
                page: 0,
                x_offset: 10.5,
                y_offset: 88.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 1,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 2,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 3,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 4,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 5,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 6,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 7,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 8,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 9,
                page: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                x_flip_offset: 0.0,
            },
            GridTranslateTestCase {
                index: 10,
                page: 1,
                x_offset: 199.5,
                y_offset: 176.0,
                x_flip_offset: 0.0,
            },
        ]);
    }

    #[derive(Debug)]
    struct GridTranslateTestCase {
        index: usize,
        page: usize,
        x_offset: f32,
        y_offset: f32,
        x_flip_offset: f32,
    }

    impl PartialEq for GridTranslateTestCase {
        fn eq(&self, other: &Self) -> bool {
            self.index == other.index &&
                self.page == other.page &&
                (self.x_offset - other.x_offset).abs() < 0.0001 &&
                (self.y_offset - other.y_offset).abs() < 0.0001
        }
    }
}