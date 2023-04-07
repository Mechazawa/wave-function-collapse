mod grid;
mod sprite;
mod superstate;
mod tile;
mod wfc;

use image::Rgba;
use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use image::{ImageError, RgbaImage};
use imageproc::drawing::draw_text_mut;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::{info, trace};
use log::warn;
use rand::rngs::OsRng;
use rand::Rng;
use rusttype::{Font, Scale};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use tile::TileConfig;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};

use grid::Size;
use superstate::SuperState;
use tile::Tile;

use crate::grid::Grid;
use crate::wfc::WaveFuncCollapse;

fn load_image(s: &str) -> Result<DynamicImage, ImageError> {
    let path = PathBuf::from(s);
    let image = ImageReader::open(path)?.decode()?;

    Ok(image)
}

fn load_config(s: &str) -> Result<Vec<TileConfig>, String> {
    let path = PathBuf::from(s);
    let file = File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?;
    let reader = BufReader::new(file);
    let configs = serde_json::from_reader(reader).map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(configs)
}

fn load_input(s: &str) -> Result<Input, &'static str> {
    if let Ok(image) = load_image(s) {
        Ok(Input::Image(image))
    } else if let Ok(configs) = load_config(s) {
        Ok(Input::Config(configs))
    } else {
        Err("Failed to load input")
    }
}


#[derive(Debug)]
enum Input {
    Image(DynamicImage),
    Config(Vec<TileConfig>),
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Wave Function Collapse",
    about = "Generate images using wfc from input images"
)]
struct Opt {
    #[structopt(flatten)]
    verbose: QuietVerbose,

    #[structopt(parse(try_from_str=load_input), help = "Input")]
    input: Input,

    #[structopt(
        parse(try_from_str),
        short,
        long,
        required_if("input-type", "config"),
        help = "Input image grid size"
    )]
    input_size: Option<Size>,

    #[structopt(parse(from_os_str), help = "Output image")]
    output: PathBuf,

    #[structopt(
        parse(try_from_str),
        short,
        long,
        default_value = "20x20",
        help = "Output image grid size"
    )]
    output_size: Size,

    #[structopt(parse(try_from_str), short, long, help = "Random seed")]
    seed: Option<u64>,
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

    let mut tiles = match &opt.input {
        Input::Image(value) => Tile::from_image(value, &opt.input_size.unwrap()),
        Input::Config(value) => Tile::from_config(value),
    };

    info!("{} unique tiles found", tiles.len());

    let invalid_neighbors = tiles
        .iter()
        .map(|t| t.neighbors.len())
        .filter(|c| *c != 4)
        .collect::<Vec<usize>>();

    if invalid_neighbors.len() > 0 {
        warn!(
            "Found {} tiles with invalid amount of neighbors: {:?}",
            invalid_neighbors.len(),
            invalid_neighbors
        );

        tiles.retain(|t| t.neighbors.len() == 4);

        warn!("Retained {} tiles", tiles.len());
    }

    let base_state = SuperState::new(tiles.iter().cloned().map(Rc::new).collect());
    let grid = Grid::new(
        opt.output_size.width,
        opt.output_size.height,
        &mut |_, _| base_state.clone(),
    );
    let seed = opt.seed.unwrap_or(OsRng.gen());

    info!("Using seed: {}", seed);

    let max_progress = grid.size() as u64;
    let progress = ProgressBar::new(grid.size() as u64);
    let mut wfc = WaveFuncCollapse::new(grid, seed);

    progress.enable_steady_tick(Duration::from_millis(200));
    progress.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {per_sec}",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    while !wfc.done() {
        progress.set_position(max_progress - wfc.remaining() as u64);
        wfc.tick();
    }

    progress.finish();

    info!("Drawing output");

    // drawing
    let (tile_width, tile_height) = tiles[0].sprite.image.dimensions();
    let font_data = include_bytes!("PublicPixel-z84yD.ttf"); // Use a font file from your system or project
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

    trace!("Tile size: {tile_width}x{tile_height}");

    let mut canvas = RgbaImage::new(
        opt.output_size.width as u32 * tile_width,
        opt.output_size.height as u32 * tile_height,
    );

    for (x, y, cell) in &wfc.grid {
        if let Some(t) = cell.collapsed() {
            image::imageops::overlay(
                &mut canvas,
                &t.sprite.image,
                x as i64 * tile_width as i64,
                y as i64 * tile_height as i64,
            );
        } else {
            let scale = Scale::uniform(8.0);
            let color = Rgba([255, 0, 0, 255]); // red
            let text = format!("{}", cell.entropy());

            draw_text_mut(
                &mut canvas,
                color,
                x as i32 * tile_width as i32,
                y as i32 * tile_height as i32,
                scale,
                &font,
                &text,
            );
        }
    }

    trace!("Writing output");

    // draw
    // todo temporary for making animation
    let file_name: String = opt
        .output
        .as_path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into();

    if file_name.contains("{}") {
        let mut path = opt.output.clone();
        let new_name = file_name.replace("{}", format!("{:05}", wfc.remaining()).as_str());

        path.set_file_name(new_name);

        canvas.save(path).unwrap();
    } else {
        canvas.save(opt.output.as_path()).unwrap();
    }
}
