use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use ggez::event::EventHandler;
use ggez::glam::Vec2;
use ggez::graphics;
use ggez::graphics::Color;
use ggez::graphics::Image;
use ggez::Context;
use ggez::GameResult;
use structopt::StructOpt;

use crate::wave::Wave;
use crate::sprite::Sprite;
use crate::tile::Tile;
use crate::superstate::Collapsable;

#[derive(Default, Clone, Copy, Debug, StructOpt)]
pub struct WindowConfig {
    #[structopt(long, help = "Render every step during visualisation")]
    pub slow: bool,

    #[structopt(long, help = "Sets max fps")]
    pub max_fps: Option<u32>,

    #[structopt(long, help = "Hold the image for n seconds after finishing")]
    pub hold: Option<f32>,
}

pub struct Window<T: Collapsable> {
    pub sprites: HashMap<T::Identifier, Image>,
    pub wfc: Wave<T>,
    config: WindowConfig,
}

impl Window<Tile<Sprite>> {
    pub fn new(
        ctx: &mut Context,
        tiles: &Vec<Tile<Sprite>>,
        wfc: Wave<Tile<Sprite>>,
        config: WindowConfig
    ) -> Self {
        let mut sprites = HashMap::new();

        for tile in tiles {
            let id = tile.get_id();
            let sprite = tile.value.clone().into_image(ctx);

            sprites.insert(id, sprite);
        }

        Self { sprites, wfc, config }
    }
}

impl<T:Collapsable> Window<T> {
    pub fn draw_context(&mut self, ctx: &mut Context) -> GameResult {
        let (width, height) = ctx.gfx.drawable_size();
        let tile_width = width / self.wfc.grid.width() as f32;
        let tile_height = height / self.wfc.grid.height() as f32;

        let mut canvas = graphics::Canvas::from_frame(
            ctx,
            // graphics::Color::from([0.1, 0.2, 0.3, 1.0]),
            Color::BLACK,
        );

        for (x, y, cell) in &self.wfc.grid {
            let pos = Vec2::new(
                (x as f32 * tile_width).into(),
                (y as f32 * tile_height).into(),
            );

            assert!(pos.x <= width);
            assert!(pos.y <= height);

            if let Some(tile) = cell.collapsed() {
                let sprite = self.sprites.get(&tile.get_id()).unwrap();
                
                canvas.draw(sprite, pos);
            } else if cell.collapsing() {
                let color = if cell.entropy() > 0 {
                    let ratio = cell.entropy() as f32 / cell.base_entropy() as f32;
                    let value = (255.0 * (1.0 - ratio)) as u8;

                    Color::from_rgb(0, value / 3, value / 2)
                } else {
                    Color::RED
                };
                
                let mesh = graphics::Mesh::new_rectangle(
                    ctx,
                    graphics::DrawMode::fill(),
                    graphics::Rect::new(0.0, 0.0, tile_width, tile_height),
                    color,
                )?;

                canvas.draw(&mesh,  pos);
            }
        }

        canvas.finish(ctx)?;

        Ok(())
    }

    pub fn tick(&mut self) {
        self.wfc.tick();
    }

    pub fn tick_once(&mut self) {
        self.wfc.tick_once();
    }
}

impl<T: Collapsable> EventHandler for Window<T> {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if self.config.slow {
            self.tick_once();
        } else {
            self.tick();
        }

        if self.wfc.done() {
            if let Some(secs) = self.config.hold {
                thread::sleep(Duration::from_secs_f32(secs))
            }

            ctx.request_quit();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.draw_context(ctx)
    }
}
