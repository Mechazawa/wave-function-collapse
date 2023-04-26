use image::DynamicImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::Pixel;
use log::debug;
use num_traits::cast::ToPrimitive;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str::FromStr;
use std::collections::hash_map::DefaultHasher;

#[derive(Debug)]
pub struct Size {
    width: u32,
    height: u32,
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

#[derive(Debug, Clone)]
pub struct Tile<T>
where
    T: GenericImageView,
{
    image: T,
    /// todo: neighbours per side
    neighbors: HashSet<Rc<Tile<T>>>,
}

impl<T> Tile<T>
where
    T: GenericImageView,
    DynamicImage: From<ImageBuffer<<T as GenericImageView>::Pixel, Vec<<<T as GenericImageView>::Pixel as Pixel>::Subpixel>>>,
{
    pub fn get_tile_set(image: &T, grid_size: &Size) -> HashSet<Rc<Tile<DynamicImage>>> {
        let (image_width, image_height) = image.dimensions();
        let tile_width = image_width / grid_size.width;
        let tile_height = image_height / grid_size.height;

        let mut output: HashSet<Rc<Tile<DynamicImage>>> = HashSet::new();
        let mut grid: Vec<Rc<Tile<DynamicImage>>> = Vec::new();

        debug!("Generating tiles...");
    
        for y in 0..grid_size.height {
            for x in 0..grid_size.width {
                let view = image.view(
                    x * tile_width,
                    y * tile_height,
                    tile_width,
                    tile_height,
                );

                let buffer = ImageBuffer::from_fn(tile_width, tile_height, |x, y| view.get_pixel(x, y));

                let new_tile = Rc::new(Tile {
                    image: DynamicImage::from(buffer),
                    neighbors: HashSet::new(),
                });

                output.insert(Rc::clone(&new_tile));

                let tile = output.get(&new_tile).unwrap();

                grid.push(Rc::clone(&tile));
            }
        }

        debug!("Populating neighbors");

        let offsets: &[i32] = &[-1, 1, -(grid_size.width as i32), grid_size.width as i32];

        for index in 0..grid.len() {
            let mut tile_ref = Rc::clone(&grid[index]);
            
            for offset in offsets {
                let target = index as i32 + offset;

                if target >= 0 {
                    if let Some(value) = grid.get(target as usize) {
                        let tile = Rc::make_mut(&mut tile_ref);

                        tile.neighbors.insert(Rc::clone(&value));
                    }
                }
            }
        }

        output
    }
}

impl<T> Hash for Tile<T>
where
    T: GenericImageView,
{
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

impl<T> PartialEq for Tile<T>
where
    T: GenericImageView,
{
    fn eq(&self, other: &Self) -> bool {
        let mut s1 = DefaultHasher::new();
        let mut s2 = DefaultHasher::new();

        self.hash(&mut s1);
        other.hash(&mut s2);

        s1.finish() == s2.finish()
    }
}
impl<T> Eq for Tile<T> where T: GenericImageView {}
