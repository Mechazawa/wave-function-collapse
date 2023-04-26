use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::Pixel;
use log::debug;
use log::trace;
use log::warn;
use num_traits::cast::ToPrimitive;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
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
pub struct Tile {
    pub image: Rc<DynamicImage>,
    /// todo: neighbours per side
    neighbors: HashMap<Direction, HashSet<Rc<Self>>>,
}

impl Tile {
    pub fn get_tile_set(image: &DynamicImage, grid_size: &Size) -> Vec<Rc<Self>> {
        let (image_width, image_height) = image.dimensions();
        let tile_width = image_width / grid_size.width;
        let tile_height = image_height / grid_size.height;

        let mut unique: HashSet<Rc<Self>> = HashSet::new();
        let mut grid: Vec<Rc<Self>> = Vec::new();

        debug!("Generating tiles");

        for y in 0..grid_size.height {
            for x in 0..grid_size.width {
                let view = image.view(x * tile_width, y * tile_height, tile_width, tile_height);

                let buffer =
                    ImageBuffer::from_fn(tile_width, tile_height, |x, y| view.get_pixel(x, y));

                let new_tile = Rc::new(Tile {
                    image: Rc::new(DynamicImage::from(buffer)),
                    neighbors: HashMap::new(),
                });

                unique.insert(new_tile.clone());

                let tile = unique.get(&new_tile).unwrap();

                grid.push(tile.clone());
            }
        }

        debug!("Populating neighbors");

        for index in 0..grid.len() {
            let mut tile_ref = grid[index].clone();

            for (direction, offset) in grid_size.get_offsets() {
                let target = index as i32 + offset;

                if let Some(value) = grid.get(target as usize) {
                    let tile = Rc::make_mut(&mut tile_ref);

                    tile.neighbors
                        .entry(direction)
                        .or_insert_with(HashSet::new)
                        .insert(Rc::clone(&value));
                }
            }

            assert!(tile_ref.neighbors.len() > 0);

            // todo: ?????
            unique.replace(tile_ref);
        }

        let output = unique.into_iter().collect::<Vec<Rc<Self>>>();

        for tile in output.iter() {
            assert!(tile.neighbors.len() > 0);
        }

        // todo: Keep track of rotation

        output
    }
}

impl Hash for Tile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for pixel in self.image.pixels() {
            for channel in pixel.2.channels() {
                if let Some(value) = channel.to_i32() {
                    state.write_i32(value)
                }
            }
        }
    }
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        let mut s1 = DefaultHasher::new();
        let mut s2 = DefaultHasher::new();

        self.hash(&mut s1);
        other.hash(&mut s2);

        s1.finish() == s2.finish()
    }
}
impl Eq for Tile {}

pub trait Collapsable {
    fn test(&self, neighbors: &HashMap<Direction, Vec<Rc<Self>>>) -> bool;
}

impl Collapsable for Tile {
    fn test(&self, neighbors: &HashMap<Direction, Vec<Rc<Self>>>) -> bool {
        let mut valid = 0;
            
        assert!(self.neighbors.len() > 0);

        for (direction, tiles) in neighbors {
            let possible = match self.neighbors.get(direction) {
                Some(v) => v,
                None => {
                    valid += 1;
                    continue;
                },
            };

            for tile in tiles {
                if possible.contains(tile) {
                    valid += 1;
                    break;
                }
            }
        }

        assert!(valid <= neighbors.len());

        valid == neighbors.len()
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
        self.possible = vec![self.possible.choose(rng).unwrap().clone()];
    }

    pub fn tick(&mut self, neighbors: &HashMap<Direction, Vec<Rc<T>>>) {
        if self.entropy() > 1 {
            self.possible.retain(|v| v.test(&neighbors));

            assert!(self.entropy() > 0);
        }
    }
}
