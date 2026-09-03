use crate::grid::Neighbors;
use crate::superstate::Collapsable;
use crate::wave::Set;
use enum_map::EnumMap;

#[cfg(feature = "image")]
use {
    crate::error::Error,
    crate::grid::{Direction, Grid, Size},
    enum_map::enum_map,
    fxhash::FxHashMap,
    image::{DynamicImage, GenericImageView, ImageBuffer, ImageReader, Pixel},
    log::debug,
    num_traits::cast::ToPrimitive,
    serde::Deserialize,
    std::collections::hash_map::{DefaultHasher, Entry},
    std::hash::Hasher,
    std::path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct Tile<T> {
    pub value: T,
    /// Ids of the tiles allowed on each side of this one.
    pub neighbors: Neighbors<Set<u64>>,
    pub weight: usize,

    id: u64,
}

#[cfg(feature = "image")]
#[derive(Debug, Deserialize)]
pub struct TileConfig {
    pub image: PathBuf,
    pub slots: Vec<String>,
}

#[cfg(feature = "image")]
impl Tile<DynamicImage> {
    /// Builds a tile set from per-tile images and edge labels. Two tiles may sit
    /// beside each other when one's edge label reads as the reverse of the other's.
    ///
    /// # Errors
    /// [`Error::SlotCount`] when a config does not carry exactly four edge labels,
    /// and [`Error::Io`] or [`Error::Image`] when one of the images will not load.
    pub fn from_config(configs: &[TileConfig]) -> Result<Vec<Self>, Error> {
        let mut output = Vec::new();
        let mut slots: Vec<(u64, Neighbors<String>)> = Vec::new();

        output.reserve_exact(configs.len());
        slots.reserve_exact(configs.len());

        for config in configs {
            let [up, right, down, left] = config.slots.as_slice() else {
                return Err(Error::SlotCount {
                    found: config.slots.len(),
                });
            };

            let neighbors = enum_map! {
                Direction::Up => up.clone(),
                Direction::Right => right.clone(),
                Direction::Down => down.clone(),
                Direction::Left => left.clone(),
            };

            let image = ImageReader::open(config.image.as_path())?.decode()?;
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
                        output[index].neighbors[direction].insert(*id);
                    }
                }
            }
        }

        Ok(output)
    }

    /// Cuts a sample image into a grid of tiles, keeping one of each distinct tile
    /// and weighting it by how often it appears. Adjacency comes from which tiles
    /// touch in the sample. A sample whose sides are not whole multiples of the
    /// tile size is cut short at the right and bottom edges.
    ///
    /// # Errors
    /// [`Error::SampleTooSmall`] when the sample cannot hold even one tile.
    pub fn from_image(image: &DynamicImage, tile_size: &Size) -> Result<Vec<Self>, Error> {
        let (image_width, image_height) = image.dimensions();
        let grid_width = image_width as usize / tile_size.width;
        let grid_height = image_height as usize / tile_size.height;

        if grid_width == 0 || grid_height == 0 {
            return Err(Error::SampleTooSmall {
                sample_width: image_width,
                sample_height: image_height,
                tile_width: tile_size.width as u32,
                tile_height: tile_size.height as u32,
            });
        }

        let mut unique: FxHashMap<u64, Self> = FxHashMap::default();

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

            match unique.entry(tile_id) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().weight += 1;
                }
                Entry::Vacant(entry) => {
                    entry.insert(new_tile);
                }
            }

            tile_id
        });

        debug!("Populating neighbors");

        for (x, y, tile_id) in &grid {
            let tile = unique
                .get_mut(tile_id)
                .expect("every id in the grid was inserted while building it");

            for (direction, maybe) in grid.get_neighbors(x, y) {
                if let Some(value) = maybe {
                    tile.neighbors[direction].insert(*value);
                }
            }
        }

        // todo: Keep track of rotation

        Ok(unique.into_values().collect())
    }

    #[must_use]
    pub fn new_image_tile(image: DynamicImage) -> Self {
        let mut hasher = DefaultHasher::new();

        for pixel in image.pixels() {
            for channel in pixel.2.channels() {
                if let Some(value) = channel.to_u8() {
                    hasher.write_u8(value);
                }
            }
        }

        Self::new(hasher.finish(), image)
    }
}

impl<T> Tile<T> {
    /// A tile with no allowed neighbours in any direction, which fails every
    /// constraint until `neighbors` is populated.
    #[must_use]
    pub fn new(id: u64, value: T) -> Self {
        Self {
            id,
            value,
            neighbors: EnumMap::default(),
            weight: 1,
        }
    }

    #[must_use]
    pub fn with_neighbors(
        id: u64,
        value: T,
        weight: usize,
        neighbors: Neighbors<Set<u64>>,
    ) -> Self {
        Self {
            id,
            value,
            neighbors,
            weight,
        }
    }
}

impl<T: Clone + Sync + Send> Collapsable for Tile<T> {
    type Identifier = u64;

    fn test(&self, neighbors: &Neighbors<Set<Self::Identifier>>) -> bool {
        for (direction, tiles) in neighbors {
            if tiles.is_empty() {
                continue;
            }

            let possible = &self.neighbors[direction];

            if possible.is_disjoint(tiles) {
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

// Implement Collapsable for Arc<Tile<T>> to enable shared ownership
impl<T: Clone + Sync + Send> Collapsable for std::sync::Arc<Tile<T>> {
    type Identifier = u64;

    fn test(&self, neighbors: &Neighbors<Set<Self::Identifier>>) -> bool {
        self.as_ref().test(neighbors)
    }

    fn get_id(&self) -> Self::Identifier {
        self.as_ref().get_id()
    }

    fn get_weight(&self) -> usize {
        self.as_ref().get_weight()
    }
}
