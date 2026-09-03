use crate::tile::Tile;
use crate::wave::Wave;

use image::{DynamicImage, GenericImageView, RgbaImage, imageops};

impl Wave<Tile<DynamicImage>> {
    /// Pixel size of a single tile. `None` when the wave holds no tiles.
    #[must_use]
    pub fn tile_size(&self) -> Option<(u32, u32)> {
        Some(self.base_state()?.possible.first()?.value.dimensions())
    }

    /// Draws every collapsed cell onto one canvas. Cells that are still open stay
    /// fully transparent. `None` when the wave holds no tiles to take a size from.
    #[must_use]
    pub fn to_rgba(&self) -> Option<RgbaImage> {
        let (tile_width, tile_height) = self.tile_size()?;

        let mut canvas = RgbaImage::new(
            self.width() as u32 * tile_width,
            self.height() as u32 * tile_height,
        );

        self.grid()
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
}
