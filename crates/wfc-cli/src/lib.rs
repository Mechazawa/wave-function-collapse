//! Command line wave function collapse image generator.
//!
//! Everything that knows what a tile looks like lives here: cutting a sample image
//! into tiles, reading a tile config, drawing a finished wave, and showing one as
//! it collapses. The algorithm itself is in [`wave_function_collapse`].

pub mod app;
pub mod cli;
pub mod compose;
pub mod error;
pub mod render;
pub mod tiles;

pub use error::Error;
