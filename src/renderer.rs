use crate::grid::Size;
use crate::sprite::Sprite;
use crate::superstate::SuperState;
use crate::tile::Tile;

pub trait Renderer {
    type Error;
    
    fn new(size: Size, tiles: &[Tile<Sprite>], config: RendererConfig) -> Result<Self, Self::Error>
    where
        Self: Sized;
    
    fn clear(&mut self);
    
    fn draw_tile(&mut self, x: u32, y: u32, tile: &Tile<Sprite>) -> Result<(), Self::Error>;
    
    fn draw_cell(&mut self, x: usize, y: usize, cell: &SuperState<Tile<Sprite>>, base_entropy: usize, show_debug: bool) -> Result<(), Self::Error>;
    
    fn present(&mut self);
    
    fn handle_events(&mut self) -> bool;
    
    fn should_quit(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct RendererConfig {
    pub vsync: bool,
    pub fullscreen: bool,
    pub tile_size: (u32, u32),
}

impl RendererConfig {
    pub fn new(vsync: bool, fullscreen: bool, tile_size: (u32, u32)) -> Self {
        Self {
            vsync,
            fullscreen,
            tile_size,
        }
    }
}

#[cfg(feature = "sdl2")]
pub mod sdl_renderer;

#[cfg(target_arch = "wasm32")]
pub mod canvas_renderer;

#[cfg(feature = "sdl2")]
pub use sdl_renderer::SdlRenderer;

#[cfg(target_arch = "wasm32")]
pub use canvas_renderer::CanvasRenderer;