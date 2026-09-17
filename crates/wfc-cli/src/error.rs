use std::path::PathBuf;

/// Everything reading a tile set from disk can fail at.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "a {tile_width}x{tile_height} tile does not fit in a {sample_width}x{sample_height} sample"
    )]
    SampleTooSmall {
        sample_width: u32,
        sample_height: u32,
        tile_width: u32,
        tile_height: u32,
    },

    #[error("a tile needs 4 edge slots, up right down left, but has {found}")]
    SlotCount { found: usize },

    #[error("{path} could not be read as an image: {source}")]
    Tile {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("{path} is neither an image ({image}) nor a tile config ({config})")]
    UnreadableInput {
        path: PathBuf,
        image: String,
        config: String,
    },

    #[error("--input-size is required to cut a sample image into tiles")]
    MissingInputSize,

    #[error("the tile set is empty")]
    NoTiles,

    #[error(
        "gave up after restarting {restarts} times; this tile set may not be able to \
         fill a grid this size, or --max-restarts needs raising"
    )]
    Unsolvable { restarts: usize },

    #[error(transparent)]
    Wfc(#[from] wave_function_collapse::Error),

    #[error("the display could not be set up: {0}")]
    Display(String),

    #[error("could not write {path}: {source}")]
    Output {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
}

/// SDL2 reports every failure as a string, so `?` needs somewhere to put one.
impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::Display(message)
    }
}
