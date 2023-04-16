mod grid;
mod sprite;
mod superstate;
mod tile;
mod wave;
mod window;

use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use image::{ImageError, RgbaImage};

use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::warn;
use log::{info, trace};
use rand::rngs::OsRng;
use rand::Rng;

use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use window::WindowConfig;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use std::{io, usize};
use structopt::clap::Shell;
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};
use tile::TileConfig;

use grid::Size;
use superstate::SuperState;
use tile::Tile;

use crate::grid::Grid;
use wave::Wave;

fn load_image(s: &str) -> Result<DynamicImage, ImageError> {
    let path = PathBuf::from(s);
    let image = ImageReader::open(path)?.decode()?;

    Ok(image)
}

fn load_config(s: &str) -> Result<Vec<TileConfig>, String> {
    let path = PathBuf::from(s);
    let file = File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?;
    let reader = BufReader::new(file);
    let configs = serde_json::from_reader(reader)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

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

    #[structopt(
        parse(from_os_str),
        help = "Output image",
    )]
    output: Option<PathBuf>,

    #[structopt(
        parse(try_from_str),
        short,
        long,
        default_value = "20x20",
        help = "Output image grid size",
    )]
    output_size: Size,

    #[structopt(parse(try_from_str), short, long, help = "Random seed (unstable)")]
    seed: Option<u64>,

    #[cfg(feature = "display")]
    #[structopt(short = "V", long, help = "Open a window to show the generation")]
    visual: bool,

    #[cfg(feature = "display")]
    #[structopt(flatten)]
    window: WindowConfig,

    #[structopt(long, possible_values= &Shell::variants(), case_insensitive = true, help = "Generate shell completions and exit")]
    completions: Option<Shell>,
}

#[cfg(feature = "image")]
fn main() {
    use ggez::{conf, event};

    use crate::window::Window;

    let opt: Opt = Opt::from_args();

    if let Some(shell) = opt.completions {
        Opt::clap().gen_completions_to(env!("CARGO_PKG_NAME"), shell, &mut io::stdout());
        return;
    }

    TermLogger::init(
        opt.verbose.get_level_filter(),
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let mut tiles = match &opt.input.unwrap() {
        Input::Image(value) => Tile::from_image(value, &Size::uniform(opt.input_size.unwrap())),
        Input::Config(value) => Tile::from_config(value),
    };

    info!("{} unique tiles found", tiles.len());

    let invalid_neighbors = tiles
        .iter()
        .map(|t| t.neighbors.len())
        .filter(|c| *c != 4)
        .collect::<Vec<usize>>();

    if !invalid_neighbors.is_empty() {
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

    let wfc = Wave::new(grid, seed);

    #[cfg(feature = "display")]
    let draw_context = if opt.visual {
        let (tile_width, tile_height) = tiles[0].value.image.dimensions();
        let mut size = opt.output_size;

        assert_eq!(tile_width, tile_height);

        size.scale(tile_width.try_into().unwrap());

        println!("{:?}", size);

        let builder = ggez::ContextBuilder::new("Wave Function Collapse", "Mechazawa").window_mode(
            conf::WindowMode::default()
                .dimensions(size.width as f32, size.height as f32)
                .resizable(false),
        );

        Some(builder.build().unwrap())
    } else {
        None
    };


    #[cfg(feature = "display")]
    if let Some((mut context, event_loop)) = draw_context {
        let window = Window::new(&mut context, &tiles, wfc, opt.window);

        event::run(context, event_loop, window);
    }

    let max_progress = wfc.grid.size() as u64;
    let progress = ProgressBar::new(max_progress);

    progress.enable_steady_tick(Duration::from_millis(200));
    progress.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {per_sec}",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    #[cfg(not(feature = "display"))]
    while !wfc.done() {
        progress.set_position(max_progress - wfc.remaining() as u64);

        wfc.tick();
    }

    #[cfg(not(feature = "display"))]
    if opt.slow {
        wfc.tick_once();
    } else {
        wfc.tick();
    }

    progress.finish();

    #[cfg(feature = "display")]
    if let Some(delay) = opt.window.hold {
        info!("Waiting for {} seconds", delay);

        std::thread::sleep(Duration::from_secs_f32(delay));
    }

    info!("Drawing output");
    if opt.output.is_none() {
        return;
    }

    // drawing
    let (tile_width, tile_height) = tiles[0].value.image.dimensions();

    trace!("Tile size: {tile_width}x{tile_height}");

    let mut canvas = RgbaImage::new(
        opt.output_size.width as u32 * tile_width,
        opt.output_size.height as u32 * tile_height,
    );

    for (x, y, cell) in &wfc.grid {
        if let Some(t) = cell.collapsed() {
            image::imageops::overlay(
                &mut canvas,
                &t.value.image,
                x as i64 * tile_width as i64,
                y as i64 * tile_height as i64,
            );
        }
    }

    trace!("Writing output");

    canvas.save(opt.output.unwrap().as_path()).unwrap();
}
