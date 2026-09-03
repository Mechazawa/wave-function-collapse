/// Everything this crate can fail at.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("cell ({x}, {y}) is outside a {width}x{height} grid")]
    OutOfBounds {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    },

    #[error("{0:?} is not a WIDTHxHEIGHT size, for example 20x20")]
    InvalidSize(String),
}
