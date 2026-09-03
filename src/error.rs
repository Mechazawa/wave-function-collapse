/// Everything this crate can fail at.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("the tile set is empty")]
    NoTiles,

    #[error(
        "a {tile_width}x{tile_height} tile does not fit in a {sample_width}x{sample_height} sample"
    )]
    SampleTooSmall {
        sample_width: u32,
        sample_height: u32,
        tile_width: u32,
        tile_height: u32,
    },

    #[error("cell ({x}, {y}) is outside a {width}x{height} grid")]
    OutOfBounds {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    },

    #[error("a tile needs 4 edge slots, up right down left, but has {found}")]
    SlotCount { found: usize },

    #[error("{0:?} is not a WIDTHxHEIGHT size, for example 20x20")]
    InvalidSize(String),

    #[cfg(feature = "image")]
    #[error(transparent)]
    Image(#[from] image::ImageError),

    #[cfg(feature = "image")]
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
