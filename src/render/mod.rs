pub mod events;

#[cfg(feature = "visual")]
pub mod sdl_renderer;

#[cfg(feature = "image-output")]
pub mod image_renderer;

use crate::tile::Tile;

pub use events::RenderEvent;

use crate::wave::Wave;

/// Core trait for rendering WFC generation progress
pub trait Renderer<T>
where
    T: Clone + Sync + Send,
{
    type Error;

    /// Initialize the renderer with configuration
    fn initialize(&mut self, tiles: &[Tile<T>], output_size: (usize, usize)) -> Result<(), Self::Error>;
    
    /// Handle a render event during generation
    fn handle_event(&mut self, event: &RenderEvent) -> Result<(), Self::Error>;
    
    /// Update renderer with current WFC state (for visual renderers)
    fn update(&mut self, wfc: &Wave<Tile<T>>) -> Result<(), Self::Error> {
        let _ = wfc;
        Ok(())
    }
    
    /// Check if the user wants to quit (for interactive renderers)
    fn should_quit(&mut self) -> bool {
        false
    }
    
    /// Finalize rendering with final state (e.g., save to file, display final result)
    fn finalize(&mut self, wfc: &Wave<Tile<T>>) -> Result<(), Self::Error>;
    
}

