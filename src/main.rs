mod lib;

use image::{ImageError, RgbaImage};
use image::{io::Reader as ImageReader, DynamicImage};
use log::{debug, info, trace};
use rand::thread_rng;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;
use structopt::StructOpt;
use structopt_flags::{LogLevel, QuietVerbose};
use image::GenericImageView;
use rand::prelude::SliceRandom;

use lib::Collapsable;
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

    let tiles = Tile::get_tile_set(&opt.input, &opt.input_size);

    info!("{} unique tiles found", tiles.len());

    let base_state = SuperState {
        possible: tiles.clone(),
    };

    let mut grid = vec![base_state.clone(); opt.output_size.area() as usize];

    let mut stack: Vec<usize> = (0..grid.len()).collect();
    let mut rng = thread_rng();
    let (image_width, image_height) = opt.input.dimensions();
    let tile_width = image_width / opt.input_size.width;
    let tile_height = image_height / opt.input_size.height;
    let mut canvas = RgbaImage::new(
        opt.output_size.width * tile_width, 
        opt.output_size.height * tile_height,
    );

    stack.shuffle(&mut rng);

    grid[stack.pop().unwrap()].collapse(&mut rng);

    while stack.len() > 0 {
        info!("stack {}", stack.len());

        // todo: optimise to only test top x positions
        // test all positions
        for index in stack.iter() {
            let mut neighbors: HashMap<Direction, Vec<Rc<Tile>>> = HashMap::new();

            for (direction, offset) in opt.output_size.get_offsets() {
                let target = (*index) as i32 + offset;
                
                if let Some(cell) = grid.get(target as usize) {
                    neighbors.insert(direction, cell.possible.clone());
                }
            }

            trace!("{}", index);
            grid[*index].tick(&neighbors);
        }

        // sort the stack; entropy ascending
        stack.sort_by(|a, b| grid[*b].entropy().cmp(&grid[*a].entropy()));

        // draw
        // todo only draw recently collapsed
        // todo broken :(
        for index in 0..grid.len() {
            let x = index as u32 % opt.input_size.width;
            let y = index as u32 / opt.input_size.width;

            if let Some(t) = grid[index].collapsed() {
                trace!("draw {}, {}", x, y);
                image::imageops::overlay(&mut canvas, t.image.as_ref(), (x * tile_width) as i64, (y * tile_height) as i64);
            }
        }

        canvas.save(opt.output.as_path()).unwrap();
    }
}
