mod grid;
mod sprite;
mod superstate;
mod tile;
mod wave;

use image::{io::Reader as ImageReader, DynamicImage, GenericImageView};
use image::{ImageError, RgbaImage};

use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::warn;
use log::{info, trace};
use rand::rngs::OsRng;
use rand::Rng;

use sdl2::mouse::Cursor;
use sdl2::video::FullscreenType;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
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

#[cfg(feature = "sdl2")]
use {
    sdl2::event::Event,
    sdl2::keyboard::Keycode,
    sdl2::pixels::{Color, PixelFormatEnum},
    sdl2::rect::Rect,
    sdl2::render::{Canvas, Texture},
    sdl2::video::Window,
    sdl2::EventPump,
    sprite::Sprite,
    std::collections::HashMap,
    superstate::Collapsable,
};

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

#[cfg(feature = "sdl2")]
struct SdlDraw {
    canvas: Canvas<Window>,
    events: EventPump,
    pub textures: HashMap<u64, Texture>,
}

#[cfg(feature = "sdl2")]
impl SdlDraw {
    pub fn new(size: Size, tiles: &Vec<Tile<Sprite>>, vsync: bool) -> Self {
        let context = sdl2::init().unwrap();
        let video = context.video().unwrap();

        let window = video
            .window("Wave Function Collapse", size.width as u32, size.height as u32)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        if window.fullscreen_state() != FullscreenType::Off {
            context.mouse().show_cursor(false);
        }

        let mut builder = window.into_canvas().target_texture();

        if vsync {
            builder = builder.present_vsync();
        }

        let canvas = builder.build().map_err(|e| e.to_string()).unwrap();

        let events = context.event_pump().unwrap();
        let texture_creator = canvas.texture_creator();
        let mut textures = HashMap::new();
        
        for tile in tiles {
            if textures.contains_key(&tile.get_id()) {
                continue;
            }

            let rgba = tile.value.image.to_rgba8();
            let (width, height) = tile.value.image.dimensions();

            let mut texture = texture_creator
                .create_texture_streaming(PixelFormatEnum::RGBA32, width, height)
                .map_err(|e| e.to_string())
                .unwrap();

            texture
                .with_lock(None, |buffer: &mut [u8], _: usize| {
                    buffer.copy_from_slice(&rgba);
                })
                .unwrap();

            textures.insert(tile.get_id(), texture);
        }

        Self {
            canvas,
            events,
            textures,
        }
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
        required_unless = "completions"
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

    #[structopt(parse(try_from_str), short, long, help = "Random seed (unstable)")]
    seed: Option<u64>,

    #[cfg(feature = "sdl2")]
    #[structopt(short = "V", long, help = "Open a window to show the generation")]
    visual: bool,

    #[cfg(feature = "sdl2")]
    #[structopt(long, help = "Render every step during visualisation")]
    slow: bool,

    #[cfg(feature = "sdl2")]
    #[structopt(long, help = "Turns on vsync")]
    vsync: bool,

    #[cfg(feature = "sdl2")]
    #[structopt(long, help = "Hold the image for n seconds after finishing")]
    hold: Option<f32>,

    #[structopt(long, possible_values= &Shell::variants(), case_insensitive = true, help = "Generate shell completions and exit")]
    completions: Option<Shell>,
}

#[cfg(feature = "image")]
fn main() {
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
    let mut sdl_draw = if opt.visual {
        let (tile_width, tile_height) = tiles[0].value.image.dimensions();
        let mut size = opt.output_size;

        assert_eq!(tile_width, tile_height);

        size.scale(tile_width.try_into().unwrap());

        Some(SdlDraw::new(
            size,
            &tiles,
            opt.vsync,
        ))
    } else {
        None
    };

    while !wfc.done() {
        progress.set_position(max_progress - wfc.remaining() as u64);

        #[cfg(feature = "sdl2")]
        if let Some(draw) = sdl_draw.as_mut() {
            for event in draw.events.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => return,
                    _ => {}
                }
            }

            update_canvas(&wfc, draw);
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
    if let Some(draw) = sdl_draw.as_mut() {
        update_canvas(&wfc, draw);
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

// todo only draw updated
#[cfg(feature = "sdl2")]
fn update_canvas(wfc: &Wave<Tile<Sprite>>, context: &mut SdlDraw) {
    let (tile_width, tile_height) = wfc.grid.get(0, 0).unwrap().possible[0]
        .value
        .image
        .dimensions();

    context.canvas.clear();

    for (x, y, cell) in &wfc.grid {
        if let Some(tile) = cell.collapsed() {
            // todo streamline
            let texture = context.textures.get(&tile.get_id()).unwrap();
            let rect = Rect::new(
                x as i32 * tile_width as i32,
                y as i32 * tile_height as i32,
                tile_width,
                tile_height,
            );
            context.canvas.set_draw_color(Color::GRAY);
            context.canvas.fill_rect(rect).unwrap();
            context.canvas.copy(texture, None, Some(rect)).unwrap();
        } else {
            let color = if cell.entropy() > 0 {
                let ratio = cell.entropy() as f32 / cell.base_entropy() as f32;
                let value = (255.0 * (1.0 - ratio)) as u8;

                Color::RGB(0, value / 3, value / 2)
            } else {
                Color::RED
            };

            let rect = Rect::new(
                x as i32 * tile_width as i32,
                y as i32 * tile_height as i32,
                tile_width,
                tile_height,
            );

            context.canvas.set_draw_color(color);
            context.canvas.fill_rect(rect).unwrap();
        }
    }

    context.canvas.present();
}
