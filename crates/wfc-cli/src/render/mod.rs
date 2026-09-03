pub mod image_renderer;

#[cfg(feature = "visual")]
pub mod sdl_renderer;

use crate::error::Error;

use wave_function_collapse::{Tile, Wave};

/// Somewhere a wave gets shown or written as it collapses.
pub trait Renderer<T>
where
    T: Clone + Sync + Send,
{
    /// # Errors
    /// When the renderer cannot be set up for this tile set and output size.
    fn initialize(&mut self, tiles: &[Tile<T>], output_size: (usize, usize)) -> Result<(), Error>;

    /// Called between steps, for a renderer that shows progress.
    ///
    /// # Errors
    /// When drawing fails.
    fn update(&mut self, wfc: &Wave<Tile<T>>) -> Result<(), Error> {
        let _ = wfc;
        Ok(())
    }

    /// Whether the user has asked to stop, for an interactive renderer.
    fn should_quit(&mut self) -> bool {
        false
    }

    /// Called once the wave is finished.
    ///
    /// # Errors
    /// When the result cannot be drawn or written.
    fn finalize(&mut self, wfc: &Wave<Tile<T>>) -> Result<(), Error>;
}
