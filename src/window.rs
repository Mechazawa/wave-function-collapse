use std::collections::HashMap;

use ggez::GameResult;
use ggez::event::EventHandler;
use ggez::glam::Vec2;
use ggez::graphics;
use ggez::graphics::Color;
use ggez::graphics::Image;
use ggez::Context;

use crate::grid::Grid;
use crate::sprite::Sprite;
use crate::superstate::{Collapsable, SuperState};
use crate::tile::Tile;

pub struct Window<'a, T: Clone> {
    sprites: HashMap<u64, Image>,
    grid: &'a Grid<SuperState<Tile<T>>>,
}

impl<'a, T: Clone> Window<'a, T> {
    pub fn new(
        ctx: &mut Context,
        tiles: &Vec<Tile<Sprite>>,
        grid: &Grid<SuperState<Tile<T>>>,
    ) -> Self {
        let mut sprites = HashMap::new();

        for tile in tiles {
            let id = tile.get_id();
            let sprite = tile.value.into_image(ctx);

            sprites.insert(id, sprite);
        }

        Self { sprites, grid }
    }
}

impl<'a, T: Clone> EventHandler for Window<'a, T> {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let (width, height) = ctx.gfx.size();
        let tile_width = width / self.grid.width() as f32;
        let tile_height = height / self.grid.height() as f32;

        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);
        
        for (x, y, cell) in self.grid {
            let pos = Vec2::new(
                (x as f32 * tile_width).into(),
                (y as f32 * tile_height).into(),
            );

            if let Some(tile) = cell.collapsed() {
                let sprite = self.sprites.get(&tile.get_id()).unwrap();

                canvas.draw(sprite, pos);
            } else {
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

                canvas.draw(&mesh, pos);
            }
        }

        canvas.finish(ctx)?;

        Ok(())
    }
}
