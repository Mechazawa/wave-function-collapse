//! Building a tile set out of images.
//!
//! The algorithm carries a tile's value without ever reading it, so what a tile
//! looks like is settled here rather than in the library.

use crate::error::Error;

use fxhash::FxHashMap;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageReader, Pixel};
use log::debug;
use num_traits::cast::ToPrimitive;
use serde::Deserialize;
use std::collections::hash_map::{DefaultHasher, Entry};
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use wave_function_collapse::{Collapsable, Direction, Grid, Neighbors, Size, Tile};

pub type ImageTile = Tile<DynamicImage>;

#[derive(Debug, Deserialize)]
pub struct TileConfig {
    pub image: PathBuf,
    /// Edge labels in up, right, down, left order.
    pub slots: Vec<String>,
}

/// A tile identified by its pixels, so the same picture is always the same tile.
fn identify(image: DynamicImage) -> ImageTile {
    let mut hasher = DefaultHasher::new();

    for pixel in image.pixels() {
        for channel in pixel.2.channels() {
            if let Some(value) = channel.to_u8() {
                hasher.write_u8(value);
            }
        }
    }

    Tile::new(hasher.finish(), image)
}

/// Builds a tile set from per-tile images and edge labels. Two tiles may sit
/// beside each other when one's edge label reads as the reverse of the other's.
///
/// # Errors
/// [`Error::SlotCount`] when a config does not carry exactly four edge labels, and
/// [`Error::Tile`] when one of the images will not load.
pub fn from_config(configs: &[TileConfig]) -> Result<Vec<ImageTile>, Error> {
    let mut output = Vec::with_capacity(configs.len());
    let mut labels: Vec<(u64, Neighbors<String>)> = Vec::with_capacity(configs.len());

    for config in configs {
        let [up, right, down, left] = config.slots.as_slice() else {
            return Err(Error::SlotCount {
                found: config.slots.len(),
            });
        };

        let sides = Neighbors::from_fn(|direction| {
            match direction {
                Direction::Up => up,
                Direction::Right => right,
                Direction::Down => down,
                Direction::Left => left,
            }
            .clone()
        });

        let tile = identify(load_image(&config.image)?);

        labels.push((tile.get_id(), sides));
        output.push(tile);
    }

    // An edge label read backwards is what the tile facing it has to show.
    let facing: Vec<Neighbors<String>> = labels
        .iter()
        .map(|(_, sides)| Neighbors::from_fn(|direction| sides[direction].chars().rev().collect()))
        .collect();

    for (tile, wanted) in output.iter_mut().zip(&facing) {
        for (candidate_id, candidate_labels) in &labels {
            for (side, candidate_label) in candidate_labels {
                // `side` is the candidate's own edge, so it touches this tile's
                // opposite side and belongs in that slot.
                if *candidate_label == wanted[side.invert()] {
                    tile.neighbors[side.invert()].insert(*candidate_id);
                }
            }
        }
    }

    Ok(output)
}

/// Cuts a sample image into a grid of tiles, keeping one of each distinct tile and
/// weighting it by how often it appears. Adjacency comes from which tiles touch in
/// the sample. A sample whose sides are not whole multiples of the tile size is cut
/// short at the right and bottom edges.
///
/// # Errors
/// [`Error::SampleTooSmall`] when the sample cannot hold even one tile.
pub fn from_image(sample: &DynamicImage, tile_size: &Size) -> Result<Vec<ImageTile>, Error> {
    let (sample_width, sample_height) = sample.dimensions();
    let grid_width = sample_width as usize / tile_size.width;
    let grid_height = sample_height as usize / tile_size.height;

    if grid_width == 0 || grid_height == 0 {
        return Err(Error::SampleTooSmall {
            sample_width,
            sample_height,
            tile_width: tile_size.width as u32,
            tile_height: tile_size.height as u32,
        });
    }

    debug!("Input grid: {grid_width}x{grid_height}");

    let mut unique: FxHashMap<u64, ImageTile> = FxHashMap::default();

    let grid = Grid::new(grid_width, grid_height, &mut |x, y| {
        let view = sample.view(
            x as u32 * tile_size.width as u32,
            y as u32 * tile_size.height as u32,
            tile_size.width as u32,
            tile_size.height as u32,
        );

        let buffer =
            ImageBuffer::from_fn(tile_size.width as u32, tile_size.height as u32, |ix, iy| {
                view.get_pixel(ix, iy)
            });

        let tile = identify(DynamicImage::from(buffer));
        let id = tile.get_id();

        match unique.entry(id) {
            Entry::Occupied(mut entry) => entry.get_mut().weight += 1,
            Entry::Vacant(entry) => {
                entry.insert(tile);
            }
        }

        id
    });

    for (x, y, id) in &grid {
        let tile = unique
            .get_mut(id)
            .expect("every id in the grid was inserted while building it");

        for (direction, neighbour) in grid.get_neighbors(x, y) {
            if let Some(&neighbour_id) = neighbour {
                tile.neighbors[direction].insert(neighbour_id);
            }
        }
    }

    // todo: Keep track of rotation

    Ok(unique.into_values().collect())
}

fn load_image(path: &Path) -> Result<DynamicImage, Error> {
    ImageReader::open(path)
        .map_err(image::ImageError::IoError)
        .and_then(ImageReader::decode)
        .map_err(|source| Error::Tile {
            path: path.to_path_buf(),
            source,
        })
}
