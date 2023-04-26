mod lib;

use image::GenericImageView;
use image::Rgba;
use image::{io::Reader as ImageReader, DynamicImage};
use image::{ImageError, RgbaImage};
use imageproc::drawing::draw_text_mut;
use log::debug;
use log::warn;
use log::{info, trace};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rusttype::{Font, Scale};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};

use lib::Direction;
use lib::Size;
use lib::SuperState;
use lib::Tile;

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
        .map(|t| t.neighbors.keys().len())
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

    let base_state = SuperState {
        possible: tiles.iter().cloned().map(Rc::new).collect(),
    };

    let mut grid = vec![base_state.clone(); opt.output_size.area() as usize];

    let mut stack: Vec<usize> = (0..grid.len()).collect();
    let mut rng = thread_rng();
    let (image_width, image_height) = opt.input.dimensions();
    let tile_width = image_width / opt.input_size.width;
    let tile_height = image_height / opt.input_size.height;
    let font_data = include_bytes!("PublicPixel-z84yD.ttf"); // Use a font file from your system or project
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();
    let mut collapse_stack: Vec<(usize, bool)> = vec![];

    {
        // todo: it always starts in left-bottom??
        stack.shuffle(&mut rng);

        let top = stack.pop().unwrap();

        grid[top].collapse(&mut rng);

        trace!("stack: {:?}", stack);
        debug!("Starting at index {}", top);
    }

    // todo rollback when tick result is 0
    // otherwise it'll cascade
    while stack.len() > 0 {
        info!("stack {}", stack.len());

        // todo: optimise to only test positions near collapsed
        // test all positions
        for index in stack.iter() {
            let mut neighbors: HashMap<Direction, Vec<u64>> = HashMap::new();

            for (direction, offset) in opt.output_size.get_offsets() {
                let target = (*index) as i32 + offset;

                if let Some(cell) = grid.get(target as usize) {
                    if cell.entropy() < base_state.entropy() {
                        neighbors.insert(
                            direction,
                            cell.possible.iter().map(|t| t.get_id()).collect(),
                        );
                    }
                }
            }

            // trace!("{}", index);
            grid[*index].tick(&neighbors);
        }

        // sort the stack; entropy ascending
        stack.sort_by(|a, b| grid[*a].entropy().cmp(&grid[*b].entropy()));

        let mut stack_next = vec![];

        for &id in &stack {
            if grid[id].collapsed().is_some() {
                collapse_stack.push((id, true));
            } else {
                stack_next.push(id);
            }
        }

        stack = stack_next;

        if let Some(&lowest) = &stack.get(0) {
            if grid[lowest].entropy() == 0 {
                loop {
                    let (last, implicit) = collapse_stack.pop().unwrap();

                    grid[last] = base_state.clone();

                    stack.push(last);

                    if implicit == false {
                        break;
                    }
                }

                // reset the entropy for other tiles
                for &id in &stack {
                    grid[id] = base_state.clone();
                }

                // sort the stack again
                stack.sort_by(|a, b| grid[*a].entropy().cmp(&grid[*b].entropy()));

                warn!("Backtracking");
            } else {
                grid[lowest].collapse(&mut rng);
                collapse_stack.push((lowest, false));
            }
        }

        // draw
        let mut canvas = RgbaImage::new(
            opt.output_size.width * tile_width,
            opt.output_size.height * tile_height,
        );

        for index in 0..grid.len() {
            let x = (index as u32) % opt.output_size.width;
            let y = (index as u32) / opt.output_size.width;

            if let Some(t) = grid[index].collapsed() {
                image::imageops::overlay(
                    &mut canvas,
                    &t.sprite.image,
                    (x * tile_width) as i64,
                    (y * tile_height) as i64,
                );
            } else {
                let scale = Scale::uniform(6.0);
                let color = Rgba([255, 0, 0, 255]); // red
                let text = format!("{}", grid[index].entropy());

                draw_text_mut(
                    &mut canvas,
                    color,
                    (x * tile_width) as i32,
                    (y * tile_height) as i32,
                    scale,
                    &font,
                    &text,
                );
            }
        }

        canvas.save(opt.output.as_path()).unwrap();
    }
}
