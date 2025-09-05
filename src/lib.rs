#![cfg(feature = "wasm")]

use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::{console, ImageData};
use image::GenericImageView;

mod grid;
mod renderer;
mod sprite;
mod superstate;
mod tile;
mod wave;

use grid::{Grid, Size};
use renderer::{CanvasRenderer, Renderer, RendererConfig};
use sprite::Sprite;
use superstate::SuperState;
use tile::Tile;
use wave::Wave;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct WfcGenerator {
    wave: Wave<Tile<Sprite>>,
    renderer: Option<CanvasRenderer>,
    base_entropy: usize,
}

#[wasm_bindgen]
impl WfcGenerator {
    #[wasm_bindgen(constructor)]
    pub fn new(
        input_image_data: &ImageData,
        input_tile_size: usize,
        output_width: usize,
        output_height: usize,
        seed: Option<u64>,
    ) -> Result<WfcGenerator, JsValue> {
        init_panic_hook();
        
        let sprite = Sprite::from_image_data(input_image_data)?;
        let input_size = Size::uniform(input_tile_size);
        let tiles = Tile::from_image(&sprite.image, &input_size);
        
        console::log_1(&format!("{} unique tiles found", tiles.len()).into());
        
        if tiles.is_empty() {
            return Err("No tiles could be generated from input image".into());
        }
        
        let base_state = SuperState::new(tiles.iter().cloned().map(Arc::new).collect());
        let base_entropy = base_state.entropy();
        
        let grid = Grid::new(
            output_width,
            output_height,
            &mut |_, _| base_state.clone(),
        );
        
        let seed = seed.unwrap_or_else(|| {
            (js_sys::Math::random() * (u64::MAX as f64)) as u64
        });
        
        console::log_1(&format!("Using seed: {}", seed).into());
        
        let wave = Wave::new(grid, seed);
        
        Ok(WfcGenerator {
            wave,
            renderer: None,
            base_entropy,
        })
    }
    
    #[wasm_bindgen]
    pub fn init_renderer(&mut self, canvas_size_width: u32, canvas_size_height: u32) -> Result<(), JsValue> {
        if let Some(tile) = self.wave.grid.get(0, 0).and_then(|cell| cell.possible.first()) {
            let (tile_width, tile_height) = tile.value.image.dimensions();
            let size = Size::new(canvas_size_width as usize, canvas_size_height as usize);
            let config = RendererConfig::new(false, false, (tile_width, tile_height));
            let tiles: Vec<_> = self.wave.grid.iter()
                .flat_map(|(_, _, cell)| cell.possible.iter().map(|arc_tile| (&**arc_tile).clone()))
                .collect();
            
            self.renderer = Some(CanvasRenderer::new(size, &tiles, config)?);
        }
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn step(&mut self) -> bool {
        if self.wave.done() {
            return false;
        }
        
        self.wave.tick_once();
        true
    }
    
    #[wasm_bindgen]
    pub fn step_batch(&mut self, steps: usize) -> usize {
        let mut completed_steps = 0;
        
        for _ in 0..steps {
            if self.wave.done() {
                break;
            }
            self.wave.tick_once();
            completed_steps += 1;
        }
        
        completed_steps
    }
    
    #[wasm_bindgen]
    pub fn render(&mut self, show_debug: bool) -> Result<(), JsValue> {
        if let Some(renderer) = &mut self.renderer {
            renderer.clear();
            
            for (x, y, cell) in &self.wave.grid {
                renderer.draw_cell(x, y, cell, self.base_entropy, show_debug)?;
            }
            
            renderer.present();
        }
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn is_done(&self) -> bool {
        self.wave.done()
    }
    
    #[wasm_bindgen]
    pub fn get_progress(&self) -> f64 {
        let total = self.wave.grid.size();
        let remaining = self.wave.remaining();
        (total - remaining) as f64 / total as f64
    }
    
    #[wasm_bindgen]
    pub fn get_output_image_data(&self) -> Result<ImageData, JsValue> {
        if !self.wave.done() {
            return Err("Generation not complete".into());
        }
        
        let mut first_tile_dims = None;
        for (_, _, cell) in &self.wave.grid {
            if let Some(tile) = cell.collapsed() {
                first_tile_dims = Some(tile.value.image.dimensions());
                break;
            }
        }
        
        let (tile_width, tile_height) = first_tile_dims
            .ok_or("No collapsed tiles found")?;
            
        let canvas_width = self.wave.grid.width() * tile_width as usize;
        let canvas_height = self.wave.grid.height() * tile_height as usize;
        
        let mut output_data = vec![0u8; canvas_width * canvas_height * 4];
        
        for (x, y, cell) in &self.wave.grid {
            if let Some(tile) = cell.collapsed() {
                let tile_rgba = tile.value.image.to_rgba8();
                
                for ty in 0..tile_height {
                    for tx in 0..tile_width {
                        let _src_idx = (ty * tile_width + tx) as usize * 4;
                        let dst_x = x * tile_width as usize + tx as usize;
                        let dst_y = y * tile_height as usize + ty as usize;
                        let dst_idx = (dst_y * canvas_width + dst_x) * 4;
                        
                        if dst_idx + 3 < output_data.len() {
                            let pixel = tile_rgba.get_pixel(tx, ty);
                            output_data[dst_idx] = pixel[0];
                            output_data[dst_idx + 1] = pixel[1];
                            output_data[dst_idx + 2] = pixel[2];
                            output_data[dst_idx + 3] = pixel[3];
                        }
                    }
                }
            }
        }
        
        ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&output_data[..]),
            canvas_width as u32,
            canvas_height as u32,
        )
    }
}

#[wasm_bindgen]
pub fn generate_from_tiles(
    tiles_data: &js_sys::Array,
    output_width: usize,
    output_height: usize,
    seed: Option<u64>,
) -> Result<ImageData, JsValue> {
    init_panic_hook();
    
    let mut tiles = Vec::new();
    
    for i in 0..tiles_data.length() {
        let tile_data = tiles_data.get(i);
        if let Ok(image_data) = tile_data.dyn_into::<ImageData>() {
            let sprite = Sprite::from_image_data(&image_data)?;
            tiles.push(Tile::new(i as u64, sprite));
        }
    }
    
    if tiles.is_empty() {
        return Err("No valid tiles provided".into());
    }
    
    let base_state = SuperState::new(tiles.iter().cloned().map(Arc::new).collect());
    let grid = Grid::new(
        output_width,
        output_height,
        &mut |_, _| base_state.clone(),
    );
    
    let seed = seed.unwrap_or_else(|| {
        (js_sys::Math::random() * (u64::MAX as f64)) as u64
    });
    
    let mut wave = Wave::new(grid, seed);
    
    while !wave.done() {
        wave.tick();
    }
    
    let (tile_width, tile_height) = tiles[0].value.image.dimensions();
    let canvas_width = output_width * tile_width as usize;
    let canvas_height = output_height * tile_height as usize;
    
    let mut output_data = vec![0u8; canvas_width * canvas_height * 4];
    
    for (x, y, cell) in &wave.grid {
        if let Some(tile) = cell.collapsed() {
            let tile_rgba = tile.value.image.to_rgba8();
            
            for ty in 0..tile_height {
                for tx in 0..tile_width {
                    let _src_idx = (ty * tile_width + tx) as usize * 4;
                    let dst_x = x * tile_width as usize + tx as usize;
                    let dst_y = y * tile_height as usize + ty as usize;
                    let dst_idx = (dst_y * canvas_width + dst_x) * 4;
                    
                    if dst_idx + 3 < output_data.len() {
                        let pixel = tile_rgba.get_pixel(tx, ty);
                        output_data[dst_idx] = pixel[0];
                        output_data[dst_idx + 1] = pixel[1];
                        output_data[dst_idx + 2] = pixel[2];
                        output_data[dst_idx + 3] = pixel[3];
                    }
                }
            }
        }
    }
    
    ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&output_data[..]),
        canvas_width as u32,
        canvas_height as u32,
    )
}