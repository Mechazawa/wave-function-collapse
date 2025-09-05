use crate::grid::Size;
use crate::renderer::{Renderer, RendererConfig};
use crate::sprite::Sprite;
use crate::superstate::{Collapsable, SuperState};
use crate::tile::Tile;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

pub struct CanvasRenderer {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    image_data_cache: HashMap<u64, ImageData>,
    tile_size: (u32, u32),
    canvas_size: (u32, u32),
}

impl Renderer for CanvasRenderer {
    type Error = JsValue;

    fn new(size: Size, tiles: &[Tile<Sprite>], config: RendererConfig) -> Result<Self, Self::Error> {
        let window = web_sys::window().ok_or("No global window exists")?;
        let document = window.document().ok_or("Window should have a document")?;
        
        let canvas = document
            .get_element_by_id("wfc-canvas")
            .ok_or("Canvas element with id 'wfc-canvas' not found")?
            .dyn_into::<HtmlCanvasElement>()?;
        
        let canvas_width = size.width as u32 * config.tile_size.0;
        let canvas_height = size.height as u32 * config.tile_size.1;
        
        canvas.set_width(canvas_width);
        canvas.set_height(canvas_height);
        
        let context = canvas
            .get_context("2d")?
            .ok_or("Canvas should have 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;
        
        let mut image_data_cache = HashMap::new();
        
        for tile in tiles {
            if image_data_cache.contains_key(&tile.get_id()) {
                continue;
            }
            
            let image_data = tile.value.to_image_data()?;
            image_data_cache.insert(tile.get_id(), image_data);
        }
        
        Ok(Self {
            canvas,
            context,
            image_data_cache,
            tile_size: config.tile_size,
            canvas_size: (canvas_width, canvas_height),
        })
    }

    fn clear(&mut self) {
        self.context.clear_rect(0.0, 0.0, self.canvas_size.0 as f64, self.canvas_size.1 as f64);
    }

    fn draw_tile(&mut self, x: u32, y: u32, tile: &Tile<Sprite>) -> Result<(), Self::Error> {
        let image_data = self.image_data_cache
            .get(&tile.get_id())
            .ok_or("Tile image data not found in cache")?;
        
        let x_pos = x as f64 * self.tile_size.0 as f64;
        let y_pos = y as f64 * self.tile_size.1 as f64;
        
        self.context.put_image_data(image_data, x_pos, y_pos)?;
        Ok(())
    }

    fn draw_cell(&mut self, x: usize, y: usize, cell: &SuperState<Tile<Sprite>>, base_entropy: usize, show_debug: bool) -> Result<(), Self::Error>
    {
        let x_pos = x as f64 * self.tile_size.0 as f64;
        let y_pos = y as f64 * self.tile_size.1 as f64;
        let width = self.tile_size.0 as f64;
        let height = self.tile_size.1 as f64;

        if let Some(tile) = cell.collapsed() {
            self.draw_tile(x as u32, y as u32, &tile)?;
        } else {
            let color = if cell.entropy() > 0 {
                let ratio = cell.entropy() as f32 / base_entropy as f32;
                let value = (255.0 * (1.0 - ratio)) as u8;
                if show_debug {
                    format!("rgb(80, {}, {})", value / 3, value / 2)
                } else {
                    format!("rgb(0, {}, {})", value / 3, value / 2)
                }
            } else {
                "rgb(0, 0, 0)".to_string()
            };

            self.context.set_fill_style(&color.into());
            self.context.fill_rect(x_pos, y_pos, width, height);
        }

        Ok(())
    }

    fn present(&mut self) {
        // Canvas automatically presents, no explicit present needed
    }

    fn handle_events(&mut self) -> bool {
        // In WASM, events are handled by JavaScript
        // This could be extended to communicate with JS for user input
        true
    }

    fn should_quit(&self) -> bool {
        // WASM doesn't quit in the traditional sense
        false
    }
}