//! Wave function collapse over an arbitrary tile set.
//!
//! A tile is an id, a weight and the set of tile ids allowed on each of its four
//! sides. What a tile *is* stays with the caller: [`Tile`] carries a value of any
//! type and the algorithm never reads it, so a caller that only needs the layout
//! can solve over `Tile<()>`.
//!
//! # Examples
//!
//! ```
//! use std::sync::Arc;
//! use wave_function_collapse::{Grid, Neighbors, Set, SuperState, Tile, Wave};
//!
//! // Land never touches sea; shore goes between. The rules have to read the same
//! // from both sides, or nothing can satisfy them.
//! let tiles: Vec<Tile<&str>> = [
//!     ("land", 0u64, [0, 1].as_slice()),
//!     ("shore", 1, &[0, 1, 2]),
//!     ("sea", 2, &[1, 2]),
//! ]
//! .into_iter()
//! .map(|(name, id, allowed)| {
//!     let allowed: Set<u64> = allowed.iter().copied().collect();
//!
//!     Tile::with_neighbors(id, name, 1, Neighbors::from_fn(|_| allowed.clone()))
//! })
//! .collect();
//!
//! let base = SuperState::new(tiles.into_iter().map(Arc::new).collect());
//! let grid = Grid::new(40, 24, &mut |_, _| base.clone());
//! let mut wave = Wave::new(grid, 20260903);
//!
//! wave.run(100);
//! assert!(wave.done(), "gave up after {} restarts", wave.restarts());
//!
//! for (_x, _y, cell) in wave.grid().iter() {
//!     let tile = cell.collapsed().expect("a finished wave has no open cell");
//!
//!     assert!(["land", "shore", "sea"].contains(&tile.value));
//! }
//! ```

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
