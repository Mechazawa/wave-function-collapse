use super::Renderer;
use wave_function_collapse::tile::Tile;
use wave_function_collapse::wave::Wave;

use image::DynamicImage;
use std::path::PathBuf;

pub struct ImageRenderer {
    output_path: PathBuf,
}

impl ImageRenderer {
    #[must_use]
    pub fn new(output_path: PathBuf) -> Self {
        Self { output_path }
    }
}

impl Renderer<DynamicImage> for ImageRenderer {
    type Error = String;

    fn initialize(
        &mut self,
        tiles: &[Tile<DynamicImage>],
        _output_size: (usize, usize),
    ) -> Result<(), Self::Error> {
        if tiles.is_empty() {
            return Err("No tiles provided".to_string());
        }

        Ok(())
    }

    fn finalize(&mut self, wfc: &Wave<Tile<DynamicImage>>) -> Result<(), Self::Error> {
        wfc.to_rgba()
            .ok_or_else(|| "Wave holds no tiles".to_string())?
            .save(&self.output_path)
            .map_err(|e| format!("Failed to save image: {e}"))
    }
}
