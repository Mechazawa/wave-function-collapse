use super::Renderer;
use crate::tile::Tile;

use image::{DynamicImage, GenericImageView, RgbaImage};
use std::path::PathBuf;

/// Image file renderer that saves the final result to disk
pub struct ImageRenderer {
    output_path: PathBuf,
    tile_size: (u32, u32),
    grid_size: (usize, usize),
    final_image: Option<RgbaImage>,
}

impl ImageRenderer {
    pub fn new(output_path: PathBuf) -> Self {
        Self {
            output_path,
            tile_size: (0, 0),
            grid_size: (0, 0),
            final_image: None,
        }
    }


    fn create_final_image_from_wfc(&mut self, wfc: &crate::wave::Wave<crate::tile::Tile<DynamicImage>>) -> Result<(), String> {
        let mut canvas = RgbaImage::new(
            self.grid_size.0 as u32 * self.tile_size.0,
            self.grid_size.1 as u32 * self.tile_size.1,
        );

        for (x, y, cell) in &wfc.grid {
            if let Some(tile) = cell.collapsed() {
                image::imageops::overlay(
                    &mut canvas,
                    tile.value.as_ref(),
                    x as i64 * self.tile_size.0 as i64,
                    y as i64 * self.tile_size.1 as i64,
                );
            }
        }

        self.final_image = Some(canvas);
        Ok(())
    }
}

impl Renderer<DynamicImage> for ImageRenderer {
    type Error = String;

    fn initialize(&mut self, tiles: &[Tile<DynamicImage>], output_size: (usize, usize)) -> Result<(), Self::Error> {
        if tiles.is_empty() {
            return Err("No tiles provided".to_string());
        }

        let (tile_width, tile_height) = tiles[0].value.as_ref().dimensions();
        self.tile_size = (tile_width, tile_height);
        self.grid_size = output_size;
        
        Ok(())
    }


    fn finalize(&mut self, wfc: &crate::wave::Wave<crate::tile::Tile<DynamicImage>>) -> Result<(), Self::Error> {
        self.create_final_image_from_wfc(wfc)?;
        
        if let Some(image) = &self.final_image {
            image.save(&self.output_path)
                .map_err(|e| format!("Failed to save image: {}", e))?;
        }
        
        Ok(())
    }

}

