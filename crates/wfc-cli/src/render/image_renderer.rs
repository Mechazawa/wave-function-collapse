use super::Renderer;
use crate::compose;
use crate::error::Error;

use image::DynamicImage;
use std::path::PathBuf;
use wave_function_collapse::{Tile, Wave};

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
    fn initialize(
        &mut self,
        tiles: &[Tile<DynamicImage>],
        _output_size: (usize, usize),
    ) -> Result<(), Error> {
        if tiles.is_empty() {
            return Err(Error::NoTiles);
        }

        Ok(())
    }

    fn finalize(&mut self, wfc: &Wave<Tile<DynamicImage>>) -> Result<(), Error> {
        compose::to_rgba(wfc)
            .ok_or(Error::NoTiles)?
            .save(&self.output_path)
            .map_err(|source| Error::Output {
                path: self.output_path.clone(),
                source,
            })
    }
}
