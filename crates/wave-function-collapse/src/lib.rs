//! Wave function collapse over an arbitrary tile set.
//!
//! A tile is an id, a weight and the set of tile ids allowed on each of its four
//! sides. What a tile *is* stays with the caller: [`Tile`] carries a value of any
//! type and the algorithm never reads it, so a caller that only needs the layout
//! can solve over `Tile<()>`.

pub mod error;
pub mod grid;
pub mod superstate;
pub mod tile;
pub mod wave;

pub use error::Error;
pub use grid::{Direction, Grid, Neighbors, Position, Size};
pub use superstate::{Collapsable, SuperState};
pub use tile::Tile;
pub use wave::{Set, Wave};
