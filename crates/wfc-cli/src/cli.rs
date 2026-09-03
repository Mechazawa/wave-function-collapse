use crate::error::Error;
use crate::tiles::TileConfig;

use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use image::{DynamicImage, ImageReader};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use wave_function_collapse::Size;

#[derive(Debug)]
pub enum Input {
    Image(DynamicImage),
    Config(Vec<TileConfig>),
}

impl Input {
    /// Reads whichever of the two input shapes the path holds: a sample image to
    /// cut into tiles, or a JSON list of tiles with their edge labels.
    fn load(path: &Path) -> Result<Self, Error> {
        let image_error = match ImageReader::open(path) {
            Err(error) => error.to_string(),
            Ok(reader) => match reader.decode() {
                Ok(image) => return Ok(Self::Image(image)),
                Err(error) => error.to_string(),
            },
        };

        File::open(path)
            .map_err(|error| error.to_string())
            .and_then(|file| {
                serde_json::from_reader(BufReader::new(file)).map_err(|error| error.to_string())
            })
            .map(Self::Config)
            .map_err(|config_error| Error::UnreadableInput {
                path: path.to_path_buf(),
                image: image_error,
                config: config_error,
            })
    }
}

#[cfg(feature = "visual")]
#[derive(Debug)]
pub struct RendererConfig {
    pub visual: bool,
    pub slow: bool,
    pub debug: bool,
    pub vsync: bool,
    pub fullscreen: bool,
    pub hold: Option<f32>,
}

#[derive(Debug)]
pub struct AppConfig {
    pub input: Input,
    /// Only a sample image needs cutting into tiles; a tile config brings its own.
    pub input_size: Option<usize>,
    pub output_size: Size,
    pub output_path: Option<PathBuf>,
    #[cfg(not(feature = "threaded"))]
    pub seed: Option<u64>,
    #[cfg(feature = "visual")]
    pub renderer: RendererConfig,
}

#[derive(Debug, Parser)]
#[command(
    name = "wave-function-collapse",
    about = "Generate images using wfc from input images",
    version,
    // -V belongs to --visual, so --version is long only.
    disable_version_flag = true
)]
pub struct Opt {
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Sample image, or a JSON tile config
    #[arg(required_unless_present = "completions")]
    input: Option<PathBuf>,

    /// Output image
    output: Option<PathBuf>,

    /// Tile size to cut the sample image into
    #[arg(short, long)]
    input_size: Option<usize>,

    /// Output image grid size
    #[arg(short, long, default_value = "20x20")]
    output_size: Size,

    /// Random seed
    #[cfg(not(feature = "threaded"))]
    #[arg(short, long)]
    seed: Option<u64>,

    /// Open a window to show the generation
    #[cfg(feature = "visual")]
    #[arg(short = 'V', long)]
    visual: bool,

    /// Render every step during visualisation
    #[cfg(feature = "visual")]
    #[arg(long)]
    slow: bool,

    /// Show debug info during visualisation
    #[cfg(feature = "visual")]
    #[arg(long)]
    debug: bool,

    /// Turn on vsync
    #[cfg(feature = "visual")]
    #[arg(long)]
    vsync: bool,

    /// Hold the image for n seconds after finishing
    #[cfg(feature = "visual")]
    #[arg(long)]
    hold: Option<f32>,

    /// Run the application in full screen
    #[cfg(feature = "visual")]
    #[arg(short, long)]
    fullscreen: bool,

    /// Generate shell completions and exit
    #[arg(long, value_enum)]
    pub completions: Option<clap_complete::Shell>,

    /// Print version
    #[arg(long, action = clap::ArgAction::Version)]
    version: Option<bool>,
}

impl Opt {
    /// # Errors
    /// When the file behind the input path will not load.
    ///
    /// # Panics
    /// When no input path was given. clap requires one unless --completions was
    /// asked for, which main handles before reaching here.
    pub fn into_app_config(self) -> Result<AppConfig, Error> {
        let path = self
            .input
            .expect("clap requires an input unless --completions");

        Ok(AppConfig {
            input: Input::load(&path)?,
            input_size: self.input_size,
            output_size: self.output_size,
            output_path: self.output,
            #[cfg(not(feature = "threaded"))]
            seed: self.seed,
            #[cfg(feature = "visual")]
            renderer: RendererConfig {
                visual: self.visual,
                slow: self.slow,
                debug: self.debug,
                vsync: self.vsync,
                fullscreen: self.fullscreen,
                hold: self.hold,
            },
        })
    }
}
