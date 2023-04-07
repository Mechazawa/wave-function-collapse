use crate::grid::{Grid, Neighbors};
use crate::wfc::{Collapsable};
use crate::sprite::Sprite;
use crate::grid::Size;
use image::{DynamicImage, ImageBuffer};
use image::GenericImageView;
use log::debug;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct Tile {
    pub sprite: Rc<Sprite>,
    /// todo: neighbours per side
    pub neighbors: Neighbors<HashSet<u64>>,

    id: u64,
}

impl Tile {
    pub fn get_tile_set(image: &DynamicImage, grid_size: &Size) -> Vec<Self> {
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

            for (direction, value) in grid.get_neighbors(x, y) {
                tile.neighbors
                    .get_or_default(direction)
                    .unwrap()
                    .insert(*value);
            }

            assert!(tile.neighbors.count() > 0);

            // todo: ?????
            // unique.insert(tile_ref.get_id(), tile_ref);
        }

        let output = unique.values().cloned().collect::<Vec<Self>>();

        for tile in output.iter() {
            assert!(tile.neighbors.count() > 0);
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
    fn test(&self, neighbors: &Neighbors<Vec<u64>>) -> bool {
        for direction in neighbors.list() {
            let tiles = neighbors.get(direction).unwrap();
            let possible = self.neighbors.get(direction).expect("Missing neighbor");

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

    fn get_id(&self) -> u64 {
        self.id
    }
}