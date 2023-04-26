use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::Pixel;
use log::debug;
use log::trace;
use num_traits::cast::ToPrimitive;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str::FromStr;

#[derive(Debug, Copy, Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    pub fn get_offsets(&self) -> [(Direction, i32); 4] {
        [
            (Direction::Left, -1),
            (Direction::Right, 1),
            (Direction::Up, -(self.width as i32)),
            (Direction::Down, self.width as i32),
        ]
    }
}

impl FromStr for Size {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (raw_width, raw_height) = s.split_once('x').ok_or(format!("invalid format: {}", s))?;

        let width = raw_width
            .parse::<u32>()
            .map_err(|_| format!("invalid width: {}", raw_width))?;
        let height = raw_height
            .parse::<u32>()
            .map_err(|_| format!("invalid height: {}", raw_height))?;

        Ok(Size { width, height })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Sprite {
    /// Todo either figure out other purposes or phase out struct
    pub image: DynamicImage,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub sprite: Rc<Sprite>,
    /// todo: neighbours per side
    pub neighbors: HashMap<Direction, HashSet<u64>>,

    id: u64,
}

impl Tile {
    pub fn get_tile_set(image: &DynamicImage, grid_size: &Size) -> Vec<Self> {
        let (image_width, image_height) = image.dimensions();
        let tile_width = image_width / grid_size.width;
        let tile_height = image_height / grid_size.height;

        let mut unique: HashMap<u64, Self> = Default::default();
        let mut grid: Vec<u64> = Default::default();

        debug!("Generating tiles");

        for y in 0..grid_size.height {
            for x in 0..grid_size.width {
                let view = image.view(x * tile_width, y * tile_height, tile_width, tile_height);

                let buffer =
                    ImageBuffer::from_fn(tile_width, tile_height, |x, y| view.get_pixel(x, y));

                let new_tile = Tile::new(DynamicImage::from(buffer));
                let tile_id = new_tile.get_id();

                unique.insert(tile_id, new_tile);

                let tile = unique.get(&tile_id).unwrap();

                grid.push(tile.get_id());
            }
        }

        debug!("Populating neighbors");

        for index in 0..grid.len() {
            let tile = unique.get_mut(&grid[index]).unwrap();

            for (direction, offset) in grid_size.get_offsets() {
                let target = index as i32 + offset;

                if let Some(value) = grid.get(target as usize) {
                    tile.neighbors
                        .entry(direction)
                        .or_insert_with(HashSet::new)
                        .insert(*value);
                }
            }

            assert!(tile.neighbors.len() > 0);

            // todo: ?????
            // unique.insert(tile_ref.get_id(), tile_ref);

            trace!("{}: {:?}", tile.get_id(), tile.neighbors);
        }

        let output = unique.values().cloned().collect::<Vec<Self>>();

        for tile in output.iter() {
            assert!(tile.neighbors.len() > 0);
        }

        // todo: Keep track of rotation

        output
    }
}

impl Hash for Sprite {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for pixel in self.image.pixels() {
            for channel in pixel.2.channels() {
                if let Some(value) = channel.to_u8() {
                    state.write_u8(value)
                }
            }
        }
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

    /// Prevent us from calculating the hash all the
    ///   time and make it easier to pass around
    pub fn get_id(&self) -> u64 {
        self.id
    }
}

pub trait Collapsable {
    fn test(&self, neighbors: &HashMap<Direction, Vec<u64>>) -> bool;
}

impl Collapsable for Tile {
    fn test(&self, neighbors: &HashMap<Direction, Vec<u64>>) -> bool {
        for (direction, tiles) in neighbors {
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
}

#[derive(Debug, Clone)]
pub struct SuperState<T>
where
    T: Collapsable,
{
    pub possible: Vec<Rc<T>>,
}

impl<T> SuperState<T>
where
    T: Collapsable,
{
    pub fn entropy(&self) -> usize {
        self.possible.len()
    }

    pub fn collapsed(&self) -> Option<Rc<T>> {
        match self.possible.len() {
            1 => Some(self.possible.get(0)?.clone()),
            _ => None,
        }
    }

    pub fn collapse(&mut self, rng: &mut ThreadRng) {
        if self.entropy() > 1 {
            self.possible = vec![self.possible.choose(rng).unwrap().clone()];
        }
    }

    pub fn tick(&mut self, neighbors: &HashMap<Direction, Vec<u64>>) {
        if self.entropy() > 1 {
            self.possible.retain(|v| v.test(&neighbors));

            // assert!(self.entropy() > 0);
        }
    }
}
