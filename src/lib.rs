use image::GenericImageView;
use std::collections::HashSet;
use std::rc::Rc;
use std::str::FromStr;
use std::hash::{Hash, Hasher};
use image::Pixel;

#[derive(Debug)]
struct Size {
    width: i32,
    height: i32,
}

impl FromStr for Size {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (raw_width, raw_height) = s.split_once('x').ok_or(format!("invalid format: {}", s))?;

        let width = raw_width
            .parse::<i32>()
            .map_err(|_| format!("invalid width: {}", raw_width))?;
        let height = raw_height
            .parse::<i32>()
            .map_err(|_| format!("invalid height: {}", raw_height))?;

        Ok(Size { width, height })
    }
}


#[derive(Debug)]
struct Tile<T>
where
    T: GenericImageView,
{
    image: T,
    neighbours: HashSet<Rc<Tile<T>>>,
}

impl<T> Tile<T>
where
    T: GenericImageView,
{

    pub fn get_tile_set<T>(image: T, grid_size: Size) -> Tile<T> {
        let tiles: Vec<Tile<T>> = Vec::new();


    }
}

impl<T> Hash for Tile<T> where T: GenericImageView {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for pixel in self.image.pixels() {
            let rgba = pixel.2.to_rgba();

            state.write(rgba.);
        }
    }
}