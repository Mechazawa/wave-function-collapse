use crate::grid::Size;
use crate::renderer::{Renderer, RendererConfig};
use crate::sprite::Sprite;
use crate::superstate::{Collapsable, SuperState};
use crate::tile::Tile;
use image::GenericImageView;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture};
use sdl2::video::{FullscreenType, Window};
use sdl2::EventPump;
use std::collections::HashMap;

pub struct SdlRenderer {
    canvas: Canvas<Window>,
    events: EventPump,
    textures: HashMap<u64, Texture>,
    tile_size: (u32, u32),
    should_quit: bool,
}

impl Renderer for SdlRenderer {
    type Error = String;

    fn new(size: Size, tiles: &[Tile<Sprite>], config: RendererConfig) -> Result<Self, Self::Error> {
        let context = sdl2::init()?;
        let video = context.video()?;

        let mut window = video
            .window(
                "Wave Function Collapse",
                size.width as u32,
                size.height as u32,
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
        let texture_creator = canvas.texture_creator();
        let mut textures = HashMap::new();

        for tile in tiles {
            if textures.contains_key(&tile.get_id()) {
                continue;
            }

            let rgba = tile.value.image.to_rgba8();
            let (width, height) = tile.value.image.dimensions();

            let mut texture = texture_creator
                .create_texture_streaming(PixelFormatEnum::RGBA32, width, height)
                .map_err(|e| e.to_string())?;

            texture
                .with_lock(None, |buffer: &mut [u8], _: usize| {
                    buffer.copy_from_slice(&rgba);
                })
                .map_err(|e| e.to_string())?;

            textures.insert(tile.get_id(), texture);
        }

        Ok(Self {
            canvas,
            events,
            textures,
            tile_size: config.tile_size,
            should_quit: false,
        })
    }

    fn clear(&mut self) {
        self.canvas.clear();
        self.canvas.set_blend_mode(BlendMode::Blend);
    }

    fn draw_tile(&mut self, x: u32, y: u32, tile: &Tile<Sprite>) -> Result<(), Self::Error> {
        let rect = Rect::new(
            x as i32 * self.tile_size.0 as i32,
            y as i32 * self.tile_size.1 as i32,
            self.tile_size.0,
            self.tile_size.1,
        );

        let texture = self.textures.get(&tile.get_id()).ok_or("Texture not found")?;
        self.canvas.set_draw_color(Color::GRAY);
        self.canvas.fill_rect(rect).map_err(|e| e.to_string())?;
        self.canvas.copy(texture, None, Some(rect)).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    fn draw_cell(&mut self, x: usize, y: usize, cell: &SuperState<Tile<Sprite>>, base_entropy: usize, show_debug: bool) -> Result<(), Self::Error>
    {
        let rect = Rect::new(
            x as i32 * self.tile_size.0 as i32,
            y as i32 * self.tile_size.1 as i32,
            self.tile_size.0,
            self.tile_size.1,
        );

        if let Some(tile) = cell.collapsed() {
            self.draw_tile(x as u32, y as u32, &tile)?;
        } else {
            let mut color = if cell.entropy() > 0 {
                let ratio = cell.entropy() as f32 / base_entropy as f32;
                let value = (255.0 * (1.0 - ratio)) as u8;
                Color::RGB(0, value / 3, value / 2)
            } else {
                Color::BLACK
            };

            if show_debug {
                color.r = 80;
            }

            self.canvas.set_draw_color(color);
            self.canvas.fill_rect(rect).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    fn present(&mut self) {
        self.canvas.present();
    }

    fn handle_events(&mut self) -> bool {
        for event in self.events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.should_quit = true;
                    return false;
                }
                _ => {}
            }
        }
        true
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }
}