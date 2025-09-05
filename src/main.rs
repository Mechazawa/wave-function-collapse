mod grid;
mod renderer;
mod sprite;
mod superstate;
mod tile;
mod wave;

use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use image::{ImageError, RgbaImage};

#[cfg(feature = "cli")]
use indicatif::ProgressBar;
#[cfg(feature = "cli")]
use indicatif::ProgressStyle;
use log::warn;
use log::{info, trace};
use rand::rngs::OsRng;
use rand::Rng;

#[cfg(feature = "cli")]
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::fmt::Debug;
#[cfg(feature = "cli")]
use std::fs::File;
#[cfg(feature = "cli")]
use std::io::BufReader;
#[cfg(feature = "cli")]
use std::path::PathBuf;
use std::time::Duration;
#[cfg(feature = "cli")]
use std::{io, usize};
#[cfg(feature = "cli")]
use structopt::clap::Shell;
#[cfg(feature = "cli")]
use structopt::StructOpt;
#[cfg(feature = "cli")]
use structopt_flags::{LogLevel, QuietVerbose};
use tile::TileConfig;

use grid::Size;
use superstate::SuperState;
use tile::Tile;

use crate::grid::Grid;
use wave::Wave;

#[cfg(feature = "sdl2")]
use renderer::{Renderer, RendererConfig, SdlRenderer};
#[cfg(feature = "sdl2")]
use sprite::Sprite;

#[cfg(feature = "cli")]
fn load_image(s: &str) -> Result<DynamicImage, ImageError> {
    let path = PathBuf::from(s);
    let image = ImageReader::open(path)?.decode()?;

    Ok(image)
}

#[cfg(feature = "cli")]
fn load_config(s: &str) -> Result<Vec<TileConfig>, String> {
    let path = PathBuf::from(s);
    let file = File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?;
    let reader = BufReader::new(file);
    let configs = serde_json::from_reader(reader)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(configs)
}

#[cfg(feature = "cli")]
fn load_input(s: &str) -> Result<Input, &'static str> {
    if let Ok(image) = load_image(s) {
        Ok(Input::Image(image))
    } else if let Ok(configs) = load_config(s) {
        Ok(Input::Config(configs))
    } else {
        Err("Failed to load input")
    }
}


#[cfg(feature = "cli")]
#[derive(Debug)]
enum Input {
    Image(DynamicImage),
    Config(Vec<TileConfig>),
}

#[cfg(feature = "cli")]
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
        help = "Output image grid size"
    )]
    output_size: Size,

    #[cfg(not(feature = "threaded"))]
    #[structopt(parse(try_from_str), short, long, help = "Random seed")]
    seed: Option<u64>,

    #[cfg(feature = "sdl2")]
    #[structopt(short = "V", long, help = "Open a window to show the generation")]
    visual: bool,

    #[cfg(feature = "sdl2")]
    #[structopt(long, help = "Render every step during visualisation")]
    slow: bool,

    #[cfg(feature = "sdl2")]
    #[structopt(long, help = "Show debug info during visualisation")]
    debug: bool,

    #[cfg(feature = "sdl2")]
    #[structopt(long, help = "Turns on vsync")]
    vsync: bool,

    #[cfg(feature = "sdl2")]
    #[structopt(long, help = "Hold the image for n seconds after finishing")]
    hold: Option<f32>,

    #[cfg(feature = "sdl2")]
    #[structopt(short, long, help = "Runs the application in full screen")]
    fullscreen: bool,

    #[structopt(long, possible_values= &Shell::variants(), case_insensitive = true, help = "Generate shell completions and exit")]
    completions: Option<Shell>,
}

#[cfg(feature = "cli")]
fn main() {
    use std::sync::Arc;

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

    let base_state = SuperState::new(tiles.iter().cloned().map(Arc::new).collect());
    let grid = Grid::new(
        opt.output_size.width,
        opt.output_size.height,
        &mut |_, _| base_state.clone(),
    );
    let seed = {
        #[cfg(not(feature = "threaded"))]
        {opt.seed.unwrap_or(OsRng.gen())}

        #[cfg(feature = "threaded")]
        {OsRng.gen()}
    };

    info!("Using seed: {}", seed);

    let max_progress = grid.size() as u64;
    let progress = ProgressBar::new(grid.size() as u64);
    let mut wfc = Wave::new(grid, seed);

    progress.enable_steady_tick(Duration::from_millis(200));
    progress.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>5}/{len} {per_sec:>12}",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    #[cfg(feature = "sdl2")]
    let mut renderer = if opt.visual {
        let (tile_width, tile_height) = tiles[0].value.image.dimensions();
        let mut size = opt.output_size;

        assert_eq!(tile_width, tile_height);

        size.scale(tile_width.try_into().unwrap());
        
        let config = RendererConfig::new(opt.vsync, opt.fullscreen, (tile_width, tile_height));
        Some(SdlRenderer::new(size, &tiles, config).unwrap())
    } else {
        None
    };

    while !wfc.done() {
        progress.set_position(max_progress - wfc.remaining() as u64);

        #[cfg(feature = "sdl2")]
        if let Some(renderer) = renderer.as_mut() {
            if !renderer.handle_events() || renderer.should_quit() {
                return;
            }

            update_canvas_with_renderer(&wfc, renderer, opt.debug);
        }

        #[cfg(feature = "sdl2")]
        if opt.slow {
            wfc.tick_once();
        } else {
            wfc.tick();
        }

        #[cfg(not(feature = "sdl2"))]
        wfc.tick();
    }

    #[cfg(feature = "sdl2")]
    if let Some(renderer) = renderer.as_mut() {
        update_canvas_with_renderer(&wfc, renderer, opt.debug);
    }

    progress.finish();

    #[cfg(feature = "sdl2")]
    if let Some(delay) = opt.hold {
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

#[cfg(feature = "sdl2")]
fn update_canvas_with_renderer<R: Renderer>(wfc: &Wave<Tile<Sprite>>, renderer: &mut R, show_debug: bool) 
where
    <R as Renderer>::Error: std::fmt::Debug,
{
    renderer.clear();

    for (x, y, cell) in &wfc.grid {
        renderer.draw_cell(x, y, cell, cell.base_entropy(), show_debug && wfc.data.get(x, y).map(|x| x.is_some()).unwrap_or(false)).unwrap();
    }

    renderer.present();
}
