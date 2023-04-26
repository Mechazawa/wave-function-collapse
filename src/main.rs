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
use log::warn;
use log::{info, trace};
use rand::rngs::OsRng;
use rand::Rng;
use rusttype::{Font, Scale};
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use sprite::Sprite;
use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use structopt::clap::Shell;
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};
use tile::TileConfig;

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
    context: Sdl,
    canvas: Canvas<Window>,
    fps: Option<u32>,
}

#[cfg(feature = "sdl2")]
impl SdlDraw {
    pub fn new(size: Size, fps: Option<u32>) -> Self {
        let context = sdl2::init().unwrap();
        let video = context.video().unwrap();

        let window = video
            .window("SDL2 Demo", size.width as u32, size.height as u32)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        let canvas = window
            .into_canvas()
            .target_texture()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())
            .unwrap();

        Self {
            context,
            canvas,
            fps,
        }
    }
}

#[derive(Debug)]
enum Input {
    Image(DynamicImage),
    Config(Vec<TileConfig>),
}

#[derive(Debug, StructOpt)]
struct DisplayOpt {
    #[structopt(short = "V", long, help = "Open a window to show the generation")]
    visual: bool,

    #[structopt(short = "e", long, help = "Show entropy values in visualisation")]
    visual_entropy: bool,

    #[structopt(short, long, help = "Limit frames per second")]
    fps: Option<u32>,
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
    input_size: Option<Size>,

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

    #[structopt(flatten)]
    #[cfg(feature = "sdl2")]
    display: DisplayOpt,

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
        Input::Image(value) => Tile::from_image(value, &opt.input_size.unwrap()),
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

    let mut sdlDraw = if cfg!(feature = "sdl2") && opt.display.visual {
        let (tile_width, tile_height) = tiles[0].value.image.dimensions();
        let mut size = opt.output_size.clone();

        assert_eq!(tile_width, tile_height);

        size.scale(tile_width.try_into().unwrap());

        Some(SdlDraw::new(size, opt.display.fps))
    } else {
        None
    };

    while !wfc.done() {
        progress.set_position(max_progress - wfc.remaining() as u64);
        wfc.tick();

        if let Some(mut draw) = sdlDraw.as_mut() {
            update_canvas(&wfc.grid, &mut draw);
        }
    }

    progress.finish();

    info!("Drawing output");
    if opt.output.is_none() {
        return;
    }

    // drawing
    let (tile_width, tile_height) = tiles[0].value.image.dimensions();
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
                &t.value.image,
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

    canvas.save(opt.output.unwrap().as_path()).unwrap();
}

// todo only draw updated
#[cfg(feature = "sdl2")]
fn update_canvas(grid: &Grid<SuperState<Tile<Sprite>>>, context: &mut SdlDraw) {
    use sdl2::{pixels::PixelFormatEnum, rect::Rect};

    let (tile_width, tile_height) = grid
        .get(0, 0)
        .unwrap()
        .possible[0]
        .value
        .image
        .dimensions();

    context.canvas.clear();
    let texture_creator = context.canvas.texture_creator();

    for (x, y, cell) in grid {
        if let Some(t) = cell.collapsed() {
            // todo streamline
            let mut texture = texture_creator
                .create_texture_streaming(PixelFormatEnum::RGBA32, tile_width, tile_height)
                .map_err(|e| e.to_string())
                .unwrap();

            let image_rgba = cell.collapsed().unwrap().value.image.to_rgba8();

            texture
                .with_lock(None, |buffer: &mut [u8], _: usize| {
                    buffer.copy_from_slice(&image_rgba);
                })
                .map_err(|e| e.to_string())
                .unwrap();

            let rect = Rect::new(
                x as i32 * tile_width as i32,
                y as i32 * tile_height as i32,
                tile_width,
                tile_height,
            );
            context.canvas.copy(&texture, None, Some(rect)).unwrap();
        } else {
            // let scale = Scale::uniform(8.0);
            // let color = Rgba([255, 0, 0, 255]); // red
            // let text = format!("{}", cell.entropy());

            // draw_text_mut(
            //     &mut canvas,
            //     color,
            //     x as i32 * tile_width as i32,
            //     y as i32 * tile_height as i32,
            //     scale,
            //     &font,
            //     &text,
            // );
        }
    }

    context.canvas.present();
}
