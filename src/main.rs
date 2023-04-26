mod grid;
mod wfc;
mod tile;
mod sprite;

use image::GenericImageView;
use image::Rgba;
use image::{io::Reader as ImageReader, DynamicImage};
use image::{ImageError, RgbaImage};
use imageproc::drawing::draw_text_mut;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::debug;
use log::warn;
use log::info;
use rand::{Rng, SeedableRng, rngs::StdRng};
use rand::rngs::OsRng;
use rand::prelude::SliceRandom;
use rusttype::{Font, Scale};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};

use grid::Direction;
use grid::Size;
use wfc::SuperState;
use tile::Tile;

use crate::grid::Grid;
use crate::grid::Neighbors;
use crate::wfc::Collapsable;

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

    #[structopt(parse(from_os_str), help = "Output image")]
    output: PathBuf,

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

    let mut tiles = Tile::get_tile_set(&opt.input, &opt.input_size);

    info!("{} unique tiles found", tiles.len());

    let invalid_neighbors = tiles
        .iter()
        .map(|t| t.neighbors.count())
        .filter(|c| *c != 4)
        .collect::<Vec<usize>>();

    if invalid_neighbors.len() > 0 {
        warn!(
            "Found {} tiles with invalid amount of neighbors: {:?}",
            invalid_neighbors.len(),
            invalid_neighbors
        );

        tiles.retain(|t| t.neighbors.count() == 4);

        warn!("Retained {} tiles", tiles.len());
    }

    let base_state = SuperState::new(tiles.iter().cloned().map(Rc::new).collect());

    // let mut grid = vec![base_state.clone(); opt.output_size.area() as usize];
    let mut grid = Grid::new(
        opt.output_size.width,
        opt.output_size.height,
        &mut |_, _| base_state.clone(),
    );

    let mut stack: Vec<(usize, usize)> = grid.iter().map(|(x, y, _)| (x, y)).collect();

    let seed = opt.seed.unwrap_or(OsRng.gen());

    info!("Using seed: {}", seed);

    let mut rng = StdRng::seed_from_u64(seed);

    let (image_width, image_height) = opt.input.dimensions();
    let tile_width = image_width / opt.input_size.width as u32;
    let tile_height = image_height / opt.input_size.height as u32;
    let font_data = include_bytes!("PublicPixel-z84yD.ttf"); // Use a font file from your system or project
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
    let mut collapse_stack: Vec<(usize, usize, bool)> = vec![];

    collapse_stack.reserve_exact(grid.size());

    {
        // todo: it always starts in left-bottom??
        stack.shuffle(&mut rng);

        let (x, y) = stack.pop().unwrap();

        grid.get_mut(x, y).unwrap().collapse(&mut rng);

        debug!("Starting at ({}, {})", x, y);
    }

    let progress = ProgressBar::new(stack.len() as u64);

    progress.enable_steady_tick(Duration::from_millis(200));
    progress.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {per_sec}")
        .unwrap()
        .progress_chars("#>-"));


    // todo rollback when tick result is 0
    // otherwise it'll cascade
    let max_progress = stack.len() as u64;
    while stack.len() > 0 {
        progress.set_position(max_progress - stack.len() as u64);

        // todo: optimise to only test positions near collapsed
        // test all positions
        for &(x, y) in &stack {
            let mut neighbors: Neighbors<Vec<u64>> = Default::default();

            for (direction, cell) in grid.get_neighbors(x, y) {
                if cell.entropy() < base_state.entropy() {
                    neighbors.set(
                        direction,
                        cell.possible.iter().map(|t| t.get_id()).collect(),
                    );
                }
            }

            // trace!("{}", index);
            grid.get_mut(x, y).unwrap().tick(&neighbors);
        }

        let mut stack_next = Vec::new();

        stack_next.reserve_exact(stack.len());

        for (x, y) in stack {
            match grid.get(x, y).unwrap().entropy() {
                1 => collapse_stack.push((x, y, true)),
                _ => stack_next.push((x, y)),
            }
        }

        stack = stack_next;

        // sort the stack; entropy ascending
        // todo wrap in some sort of helper?
        stack.sort_by(|a, b| {
            grid.get(a.0, a.1)
                .unwrap()
                .entropy()
                .cmp(&grid.get(b.0, b.1).unwrap().entropy())
        });

        if let Some(&(x, y)) = stack.get(0) {
            if grid.get(x, y).unwrap().entropy() == 0 {
                loop {
                    let (lx, ly, implicit) = match collapse_stack.pop() {
                        None => break,
                        Some(v) => v,
                    };

                    grid.set(lx, ly, base_state.clone()).unwrap();

                    stack.push((lx, ly));

                    if implicit == false {
                        break;
                    }
                }

                // reset the entropy for other tiles
                for &(x, y) in &stack {
                    grid.set(x, y, base_state.clone()).unwrap();
                }

                // sort the stack again
                stack.sort_by(|a, b| {
                    grid.get(a.0, a.1)
                        .unwrap()
                        .entropy()
                        .cmp(&grid.get(b.0, b.1).unwrap().entropy())
                });

                // warn!("Backtracking");
            } else {
                grid.get_mut(x, y).unwrap().collapse(&mut rng);
                collapse_stack.push((x, y, false));
            }
        }
    }

    let mut canvas = RgbaImage::new(
        opt.output_size.width as u32 * tile_width,
        opt.output_size.height as u32 * tile_height,
    );

    for (x, y, cell) in &grid {
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
        let new_name = file_name.replace("{}", format!("{:05}", stack.len()).as_str());

        path.set_file_name(new_name);

        canvas.save(path).unwrap();
    } else {
        canvas.save(opt.output.as_path()).unwrap();
    }
}
