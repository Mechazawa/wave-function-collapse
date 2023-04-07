mod lib;

use image::{ImageError, GenericImageView};
use image::{io::Reader as ImageReader, DynamicImage};
use log::{debug, info};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::fmt::Debug;
use std::path::PathBuf;
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};

use lib::Tile;
use lib::Size;

fn load_image(s: &str) -> Result<DynamicImage, ImageError> {
    let path = PathBuf::from(s);
    let image = ImageReader::open(path)?.decode()?;

    Ok(image)
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Wave Function Collapse",
    about = "Generate images using wfc from input images"
)]
struct Opt {
    #[structopt(flatten)]
    verbose: QuietVerbose,

    #[structopt(parse(try_from_str=load_image), help = "Input image")]
    input: DynamicImage,

    #[structopt(parse(try_from_str=load_image), help = "Output image")]
    ouput: Option<DynamicImage>,

    #[structopt(
        parse(try_from_str),
        short,
        long,
        default_value = "10x10",
        help = "Input image grid size"
    )]
    input_size: Size,

    #[structopt(
        parse(try_from_str),
        short,
        long,
        default_value = "20x20",
        help = "Output image grid size"
    )]
    output_size: Size,
}

fn main() {
    let opt: Opt = Opt::from_args();

    TermLogger::init(
        opt.verbose.get_level_filter(),
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let tiles = Tile::get_tile_set(&opt.input, &opt.input_size);

    info!("{} unique tiles found", tiles.len());
}
