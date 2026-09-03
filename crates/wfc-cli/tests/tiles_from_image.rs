use wave_function_collapse::{Collapsable, Size};
use wfc_cli::{Error, tiles};

#[test]
fn tiles_from_a_checkerboard_dedupe_to_two_weighted_by_their_count() {
    use image::{DynamicImage, Rgba, RgbaImage};

    const TILE: u32 = 8;
    const GRID: u32 = 4;

    let red = Rgba([255, 0, 0, 255]);
    let green = Rgba([0, 255, 0, 255]);

    let sample = DynamicImage::ImageRgba8(RgbaImage::from_fn(TILE * GRID, TILE * GRID, |x, y| {
        if (x / TILE + y / TILE).is_multiple_of(2) {
            red
        } else {
            green
        }
    }));

    let tiles = tiles::from_image(&sample, &Size::uniform(TILE as usize)).unwrap();

    assert_eq!(
        tiles.len(),
        2,
        "a two-colour checkerboard has two distinct tiles"
    );

    let mut weights: Vec<usize> = tiles.iter().map(Collapsable::get_weight).collect();
    weights.sort_unstable();

    let half = (GRID * GRID / 2) as usize;
    assert_eq!(weights, vec![half, half], "each colour fills half the grid");
}

#[test]
fn a_sample_smaller_than_one_tile_is_rejected() {
    use image::{DynamicImage, RgbaImage};

    let sample = DynamicImage::ImageRgba8(RgbaImage::new(4, 40));
    let result = tiles::from_image(&sample, &Size::uniform(8));

    assert!(matches!(result, Err(Error::SampleTooSmall { .. })));
}
