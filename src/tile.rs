use crate::grid::Direction;
use crate::grid::Grid;
use crate::grid::Neighbors;
use crate::grid::Size;
use crate::superstate::Collapsable;
use crate::sprite::Sprite;
use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use log::debug;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct Tile {
    pub sprite: Rc<Sprite>,
    /// todo: neighbours per side
    pub neighbors: Neighbors<Vec<u64>>,

    id: u64,
}

#[derive(Debug)]
pub struct TileConfig {
    image: DynamicImage,
    slots: Neighbors<String>,
}

impl Tile {
    pub fn from_config(configs: Vec<TileConfig>) -> Vec<Self> {
        let mut output = Vec::new();
        let mut slots: Vec<(u64, Neighbors<String>)> = Vec::new();

        output.reserve_exact(configs.len());
        slots.reserve_exact(configs.len());

        for config in configs {
            let tile = Self::new(config.image);
            slots.push((tile.get_id(), config.slots));
            output.push(tile);
        }

        for index in 0..slots.len()  {
            for (id, neighbors) in slots {
                for (direction, key) in neighbors {
                    let inv_dir = match direction {
                        Direction::Up => Direction::Down,
                        Direction::Down => Direction::Up,
                        Direction::Left => Direction::Right,
                        Direction::Right => Direction::Left,
                    };
                    let rev_key: String = slots[index].1[inv_dir].chars().rev().collect();

                    if key == rev_key {
                        output[index].neighbors[direction].push(id);
                    }
                }
            }
        }

        output
    }

    pub fn from_image(image: &DynamicImage, grid_size: &Size) -> Vec<Self> {
        let (image_width, image_height) = image.dimensions();
        let tile_width = image_width / grid_size.width as u32;
        let tile_height = image_height / grid_size.height as u32;

        let mut unique: HashMap<u64, Self> = Default::default();

        debug!("Generating tiles");
        let grid = Grid::new(grid_size.width as usize, grid_size.height as usize, &mut |x, y| {
            let view = image.view(x as u32 * tile_width, y as u32 * tile_height, tile_width, tile_height);

            let buffer =
                ImageBuffer::from_fn(tile_width, tile_height, |ix, iy| view.get_pixel(ix, iy));

            let new_tile = Tile::new(DynamicImage::from(buffer));
            let tile_id = new_tile.get_id();

            unique.insert(tile_id, new_tile);

            unique.get(&tile_id).unwrap().get_id()
        });

        debug!("Populating neighbors");

        for (x, y, tile_id) in &grid {
            let tile = unique.get_mut(&tile_id).unwrap();

            for (direction, maybe) in grid.get_neighbors(x, y) {
                if let Some(value) = maybe {
                    if !tile.neighbors[direction].contains(value) {
                        tile.neighbors[direction].push(*value);
                        tile.neighbors[direction].sort_unstable();
                        assert!(tile.neighbors[direction].len() > 0);
                    }
                }
            }

            assert!(tile.neighbors.len() > 0);
        }

        let output: Vec<Self> = unique.values().cloned().collect::<Vec<Self>>();

        for tile in output.iter() {
            assert!(tile.neighbors.len() > 0);
        }

        // todo: Keep track of rotation

        output
    }
}

impl Tile {
    pub fn new(image: DynamicImage) -> Self {
        let mut hasher = DefaultHasher::new();
        let sprite = Sprite { image };

        sprite.hash(&mut hasher);

        Self {
            id: hasher.finish(),
            sprite: Rc::new(sprite),
            neighbors: Default::default(),
        }
    }
}

impl Collapsable for Tile {
    type Identifier = u64;

    fn test(&self, neighbors: &Neighbors<Vec<Self::Identifier>>) -> bool {
        for (direction, tiles) in neighbors {
            if tiles.len() == 0 {
                continue
            }

            let possible = &self.neighbors[direction];

            let mut found = false;

            for index in 0..tiles.len() {
                if possible.contains(&tiles[index]) {
                    found = true;
                }
            }

            if !found {
                return false;
            }
        }

        true
    }

    fn get_id(&self) -> Self::Identifier {
        self.id
    }
}