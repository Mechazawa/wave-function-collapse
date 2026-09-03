use std::sync::Arc;
use wave_function_collapse::{Grid, Neighbors, Set, SuperState, Tile, Wave};
use wfc_cli::compose;

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

const TILE_SIZE: u32 = 3;
const TEST_SEED: u64 = 42;

/// Tiles of one flat colour each, every tile allowed beside every other, so a wave
/// over them always solves and the colour identifies which tile landed in a cell.
fn colour_tiles(count: u8) -> Vec<Tile<DynamicImage>> {
    let every_tile: Set<u64> = (0..u64::from(count)).collect();

    (0..count)
        .map(|index| {
            let colour = Rgba([index * 40 + 15, 0, 0, 255]);
            let image = RgbaImage::from_pixel(TILE_SIZE, TILE_SIZE, colour);

            Tile::with_neighbors(
                u64::from(index),
                DynamicImage::from(image),
                1,
                Neighbors::from_fn(|_| every_tile.clone()),
            )
        })
        .collect()
}

fn wave(width: usize, height: usize, tiles: Vec<Tile<DynamicImage>>) -> Wave<Tile<DynamicImage>> {
    let base = SuperState::new(tiles.into_iter().map(Arc::new).collect());

    Wave::new(
        Grid::new(width, height, &mut |_, _| base.clone()),
        TEST_SEED,
    )
}

#[test]
fn canvas_covers_the_grid_at_tile_resolution() {
    let composed = compose::to_rgba(&wave(4, 7, colour_tiles(5))).unwrap();

    assert_eq!(composed.dimensions(), (4 * TILE_SIZE, 7 * TILE_SIZE));
}

#[test]
fn open_cells_stay_transparent() {
    let composed = compose::to_rgba(&wave(3, 3, colour_tiles(5))).unwrap();

    assert!(
        composed.pixels().all(|pixel| pixel.0[3] == 0),
        "nothing is collapsed yet, so every pixel should be untouched"
    );
}

#[test]
fn each_cell_lands_at_its_own_offset() {
    let mut wave = wave(6, 6, colour_tiles(5));
    wave.run(0);

    assert!(
        wave.done(),
        "the tile set is fully permissive, so it must solve"
    );

    let composed = compose::to_rgba(&wave).unwrap();

    for (x, y, cell) in wave.grid() {
        let expected = cell.collapsed().unwrap().value.get_pixel(0, 0);
        let placed = composed.get_pixel(x as u32 * TILE_SIZE, y as u32 * TILE_SIZE);

        assert_eq!(
            *placed, expected,
            "cell ({x}, {y}) was not drawn at its own offset"
        );
    }
}

#[test]
fn a_wave_without_tiles_has_nothing_to_compose() {
    assert!(compose::to_rgba(&wave(2, 2, Vec::new())).is_none());
}
