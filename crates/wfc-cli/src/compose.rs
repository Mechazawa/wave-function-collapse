use crate::tiles::ImageTile;

use image::{GenericImageView, RgbaImage, imageops};
use wave_function_collapse::Wave;

/// Pixel size of a single tile. `None` when the wave holds no tiles.
#[must_use]
pub fn tile_size(wave: &Wave<ImageTile>) -> Option<(u32, u32)> {
    Some(wave.base_state()?.possible.first()?.value.dimensions())
}

/// Draws every collapsed cell onto one canvas. Cells that are still open stay
/// fully transparent. `None` when the wave holds no tiles to take a size from.
#[must_use]
pub fn to_rgba(wave: &Wave<ImageTile>) -> Option<RgbaImage> {
    let (tile_width, tile_height) = tile_size(wave)?;

    let mut canvas = RgbaImage::new(
        wave.width() as u32 * tile_width,
        wave.height() as u32 * tile_height,
    );

    wave.grid()
        .iter()
        .filter_map(|(x, y, cell)| cell.collapsed().map(|tile| (x, y, tile)))
        .for_each(|(x, y, tile)| {
            imageops::overlay(
                &mut canvas,
                &tile.value,
                x as i64 * i64::from(tile_width),
                y as i64 * i64::from(tile_height),
            );
        });

    Some(canvas)
}
