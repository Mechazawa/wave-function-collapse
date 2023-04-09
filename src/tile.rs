use crate::grid::Direction;
use crate::grid::Grid;
use crate::grid::Neighbors;
use crate::grid::Size;
use crate::superstate::Collapsable;
use crate::tile;
use enum_map::enum_map;
use log::debug;

#[cfg(feature = "image")]
mod image_imports {
    pub use crate::sprite::Sprite;
    pub use image::io::Reader as ImageReader;
    pub use image::DynamicImage;
    pub use image::GenericImageView;
    pub use image::ImageBuffer;
    pub use serde::Deserialize;
    pub use std::collections::hash_map::DefaultHasher;
    pub use std::collections::HashMap;
    pub use std::hash::Hash;
    pub use std::hash::Hasher;
    pub use std::path::PathBuf;
}

#[cfg(feature = "image")]
use image_imports::*;

#[derive(Debug, Clone)]
pub struct Tile<T> {
    pub value: Box<T>,
    /// todo: neighbours per side
    pub neighbors: Neighbors<Vec<u64>>,

    id: u64,
    pub weight: usize,
}

#[cfg(feature = "image")]
#[derive(Debug, Deserialize)]
pub struct TileConfig {
    image: PathBuf,
    slots: Vec<String>,
}

#[cfg(feature = "image")]
impl Tile<Sprite> {
    pub fn from_config(configs: &Vec<TileConfig>) -> Vec<Self> {
        let mut output = Vec::new();
        let mut slots: Vec<(u64, Neighbors<String>)> = Vec::new();

        output.reserve_exact(configs.len());
        slots.reserve_exact(configs.len());

        for config in configs {
            let neighbors = enum_map! {
                Direction::Up => config.slots[0].clone(),
                Direction::Right => config.slots[1].clone(),
                Direction::Down => config.slots[2].clone(),
                Direction::Left => config.slots[3].clone(),
            };

            let image = ImageReader::open(config.image.as_path())
                .unwrap()
                .decode()
                .unwrap();
            let tile = Self::new_image_tile(image);

            slots.push((tile.get_id(), neighbors));
            output.push(tile);
        }

        for index in 0..slots.len() {
            for (id, neighbors) in &slots {
                for (direction, key) in neighbors {
                    let rev_key: String =
                        slots[index].1[direction.invert()].chars().rev().collect();

                    if *key == rev_key {
                        output[index].neighbors[direction].push(*id);
                    }
                }
            }
        }

        output
    }

    pub fn from_image(image: &DynamicImage, tile_size: &Size) -> Vec<Self> {
        let (image_width, image_height) = image.dimensions();
        let grid_width = image_width as usize / tile_size.width;
        let grid_height = image_height as usize / tile_size.height;

        let mut unique: HashMap<u64, Self> = Default::default();

        debug!("Input grid: {grid_width}x{grid_height}");

        debug!("Generating tiles");
        let grid = Grid::new(grid_width, grid_height, &mut |x, y| {
            let view = image.view(
                x as u32 * tile_size.width as u32,
                y as u32 * tile_size.height as u32,
                tile_size.width as u32,
                tile_size.height as u32,
            );

            let buffer =
                ImageBuffer::from_fn(tile_size.width as u32, tile_size.height as u32, |ix, iy| {
                    view.get_pixel(ix, iy)
                });

            let new_tile = Tile::new_image_tile(DynamicImage::from(buffer));
            let tile_id = new_tile.get_id();

            unique.insert(tile_id, new_tile);

            unique.get_mut(&tile_id).unwrap().weight += 1;

            assert_ne!(unique.get(&tile_id).unwrap().get_weight(), 1);
            unique.get(&tile_id).unwrap().get_id()
        });

        debug!("Populating neighbors");

        for (x, y, tile_id) in &grid {
            let tile = unique.get_mut(tile_id).unwrap();

            for (direction, maybe) in grid.get_neighbors(x, y) {
                if let Some(value) = maybe {
                    if !tile.neighbors[direction].contains(value) {
                        tile.neighbors[direction].push(*value);
                        tile.neighbors[direction].sort();
                        assert!(!tile.neighbors[direction].is_empty());
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

    pub fn new_image_tile(image: DynamicImage) -> Self {
        let mut hasher = DefaultHasher::new();
        let sprite = Sprite { image };

        sprite.hash(&mut hasher);

        Self::new(hasher.finish(), sprite)
    }
}

impl<T> Tile<T> {
    pub fn new(id: u64, value: T) -> Self {
        Self {
            id,
            value: Box::new(value),
            neighbors: Default::default(),
            weight: 1,
        }
    }
}

impl<T: Clone> Collapsable for Tile<T> {
    type Identifier = u64;

    fn test(&self, neighbors: &Neighbors<Vec<Self::Identifier>>) -> bool {
        for (direction, tiles) in neighbors {
            if tiles.is_empty() {
                continue;
            }

            let possible = &self.neighbors[direction];

            let mut found = false;

            for tile in tiles {
                if possible.contains(tile) {
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

    fn get_weight(&self) -> usize {
        self.weight
    }
}
