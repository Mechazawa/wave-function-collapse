use image::ImageReader;
use image::{DynamicImage, ImageError};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use structopt::StructOpt;
use structopt::clap::Shell;
use structopt_flags::QuietVerbose;
use wave_function_collapse::grid::Size;
use wave_function_collapse::tile::TileConfig;

fn load_image(s: &str) -> Result<DynamicImage, ImageError> {
    let path = PathBuf::from(s);
    let image = ImageReader::open(path)?.decode()?;
    Ok(image)
}

fn load_config(s: &str) -> Result<Vec<TileConfig>, String> {
    let path = PathBuf::from(s);
    let file = File::open(path).map_err(|e| format!("Failed to open config file: {e}"))?;
    let reader = BufReader::new(file);
    let configs =
        serde_json::from_reader(reader).map_err(|e| format!("Failed to parse config file: {e}"))?;
    Ok(configs)
}

fn load_input(s: &str) -> Result<Input, String> {
    let image_error = match load_image(s) {
        Ok(image) => return Ok(Input::Image(image)),
        Err(error) => error,
    };

    load_config(s).map(Input::Config).map_err(|config_error| {
        format!("{s} is neither an image ({image_error}) nor a tile config ({config_error})")
    })
}

#[derive(Debug)]
pub enum Input {
    Image(DynamicImage),
    Config(Vec<TileConfig>),
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
    pub input_size: usize,
    pub output_size: Size,
    pub output_path: Option<PathBuf>,
    #[cfg(not(feature = "threaded"))]
    pub seed: Option<u64>,
    #[cfg(feature = "visual")]
    pub renderer: RendererConfig,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Wave Function Collapse",
    about = "Generate images using wfc from input images"
)]
pub struct Opt {
    #[structopt(flatten)]
    pub verbose: QuietVerbose,

    #[structopt(parse(try_from_str=load_input), help = "Input", required_unless="completions")]
    input: Option<Input>,

    #[structopt(
        parse(try_from_str),
        short,
        long,
        required_if("input", "config"),
        help = "Input image grid size"
    )]
    input_size: Option<usize>,

    #[structopt(parse(from_os_str), help = "Output image")]
    output: Option<PathBuf>,

    #[structopt(
        parse(try_from_str),
        short,
        long,
        default_value = "20x20",
        help = "Output image grid size"
    )]
    output_size: Size,

    #[cfg(not(feature = "threaded"))]
    #[structopt(parse(try_from_str), short, long, help = "Random seed")]
    seed: Option<u64>,

    #[cfg(feature = "visual")]
    #[structopt(short = "V", long, help = "Open a window to show the generation")]
    visual: bool,

    #[cfg(feature = "visual")]
    #[structopt(long, help = "Render every step during visualisation")]
    slow: bool,

    #[cfg(feature = "visual")]
    #[structopt(long, help = "Show debug info during visualisation")]
    debug: bool,

    #[cfg(feature = "visual")]
    #[structopt(long, help = "Turns on vsync")]
    vsync: bool,

    #[cfg(feature = "visual")]
    #[structopt(long, help = "Hold the image for n seconds after finishing")]
    hold: Option<f32>,

    #[cfg(feature = "visual")]
    #[structopt(short, long, help = "Runs the application in full screen")]
    fullscreen: bool,

    #[structopt(long, possible_values= &Shell::variants(), case_insensitive = true, help = "Generate shell completions and exit")]
    pub completions: Option<Shell>,
}

impl Opt {
    pub fn into_app_config(self) -> Result<AppConfig, &'static str> {
        Ok(AppConfig {
            input: self.input.ok_or("Input is required")?,
            input_size: self.input_size.ok_or("Input size is required")?,
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
