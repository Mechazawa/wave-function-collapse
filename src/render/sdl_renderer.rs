use super::Renderer;
use crate::tile::Tile;
use crate::grid::Size;
use crate::superstate::Collapsable;

use image::{DynamicImage, GenericImageView};
use sdl2::video::FullscreenType;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::EventPump;
use std::collections::HashMap;

/// SDL2-based real-time renderer for visualizing WFC generation
pub struct SdlRenderer {
    canvas: Canvas<Window>,
    events: EventPump,
    textures: HashMap<u64, Texture>,
    tile_size: (u32, u32),
    grid_size: (usize, usize),
    show_debug: bool,
    render_every_step: bool,
    should_quit: bool,
    frame_counter: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SdlConfig {
    pub window_size: Size,
    pub vsync: bool,
    pub fullscreen: bool,
    pub show_debug: bool,
    pub render_every_step: bool,
}

impl SdlRenderer {
    pub fn new(config: &SdlConfig) -> Result<Self, String> {
        let context = sdl2::init()?;
        let video = context.video()?;

        let mut window = video
            .window(
                "Wave Function Collapse",
                config.window_size.width as u32,
                config.window_size.height as u32,
            )
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        if config.fullscreen {
            window.set_fullscreen(FullscreenType::True)?;
        }

        if window.fullscreen_state() != FullscreenType::Off {
            context.mouse().show_cursor(false);
        }

        let mut builder = window.into_canvas().target_texture();

        if config.vsync {
            builder = builder.present_vsync();
        }

        let canvas = builder.build().map_err(|e| e.to_string())?;
        let events = context.event_pump()?;

        Ok(Self {
            canvas,
            events,
            textures: HashMap::new(),
            tile_size: (0, 0),
            grid_size: (0, 0),
            show_debug: config.show_debug,
            render_every_step: config.render_every_step,
            should_quit: false,
            frame_counter: 0,
        })
    }

    fn create_textures(&mut self, tiles: &[Tile<DynamicImage>]) -> Result<(), String> {
        let texture_creator = self.canvas.texture_creator();
        
        for tile in tiles {
            if self.textures.contains_key(&tile.get_id()) {
                continue;
            }

            let rgba = tile.value.as_ref().to_rgba8();
            let (width, height) = tile.value.as_ref().dimensions();

            let mut texture = texture_creator
                .create_texture_streaming(PixelFormatEnum::RGBA32, width, height)
                .map_err(|e| e.to_string())?;

            texture
                .with_lock(None, |buffer: &mut [u8], _: usize| {
                    buffer.copy_from_slice(&rgba);
                })
                .map_err(|e| e.to_string())?;

            self.textures.insert(tile.get_id(), texture);
        }
        
        Ok(())
    }

    fn handle_events(&mut self) {
        for event in self.events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.should_quit = true;
                }
                _ => {}
            }
        }
    }


    fn render_grid_from_wfc(&mut self, wfc: &crate::wave::Wave<crate::tile::Tile<DynamicImage>>) -> Result<(), String> {
        use sdl2::render::BlendMode;

        let (tile_width, tile_height) = self.tile_size;

        self.canvas.clear();
        self.canvas.set_blend_mode(BlendMode::Blend);

        for (x, y, cell) in &wfc.grid {
            let rect = Rect::new(
                x as i32 * tile_width as i32,
                y as i32 * tile_height as i32,
                tile_width,
                tile_height,
            );

            if let Some(tile) = cell.collapsed() {
                let texture = self.textures.get(&tile.get_id())
                    .ok_or("Missing texture for tile")?;

                self.canvas.set_draw_color(Color::GRAY);
                self.canvas.fill_rect(rect).map_err(|e| e.to_string())?;
                self.canvas.copy(texture, None, Some(rect)).map_err(|e| e.to_string())?;
            } else {
                let mut color = if cell.entropy() > 0 {
                    let ratio = cell.entropy() as f32 / cell.base_entropy() as f32;
                    let value = (255.0 * (1.0 - ratio)) as u8;

                    Color::RGB(0, value / 3, value / 2)
                } else {
                    Color::BLACK
                };

                if self.show_debug {
                    // TODO: Add debug visualization for cells in propagation stack
                    // This would require access to the Wave's internal data
                    color.r = 80;
                }

                self.canvas.set_draw_color(color);
                self.canvas.fill_rect(rect).map_err(|e| e.to_string())?;
            }
        }

        self.canvas.present();
        Ok(())
    }
}

impl Renderer<DynamicImage> for SdlRenderer {
    type Error = String;

    fn initialize(&mut self, tiles: &[Tile<DynamicImage>], output_size: (usize, usize)) -> Result<(), Self::Error> {
        if tiles.is_empty() {
            return Err("No tiles provided".to_string());
        }

        let (tile_width, tile_height) = tiles[0].value.as_ref().dimensions();
        self.tile_size = (tile_width, tile_height);
        self.grid_size = output_size;
        
        self.create_textures(tiles)?;
        
        Ok(())
    }


    fn should_quit(&mut self) -> bool {
        self.should_quit
    }

    fn update(&mut self, wfc: &crate::wave::Wave<crate::tile::Tile<DynamicImage>>) -> Result<(), Self::Error> {
        self.handle_events();

        if self.should_quit {
            return Ok(());
        }

        self.frame_counter += 1;
        
        // Render every frame if render_every_step is true (slow mode)
        // Otherwise render every 10 frames to show progress without being too slow
        if self.render_every_step || (self.frame_counter % 10 == 0) {
            self.render_grid_from_wfc(wfc)?;
        }
        Ok(())
    }

    fn finalize(&mut self, wfc: &crate::wave::Wave<crate::tile::Tile<DynamicImage>>) -> Result<(), Self::Error> {
        self.render_grid_from_wfc(wfc)?;
        Ok(())
    }

}