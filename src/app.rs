use crate::cli::{AppConfig, Input};
use crate::render::Renderer;
use wave_function_collapse::grid::{Grid, Size};
use wave_function_collapse::superstate::SuperState;
use wave_function_collapse::tile::Tile;
use wave_function_collapse::wave::Wave;

#[cfg(feature = "visual")]
use crate::render::sdl_renderer::{SdlConfig, SdlRenderer};

#[cfg(feature = "image")]
use crate::render::image_renderer::ImageRenderer;

use image::DynamicImage;

#[cfg(feature = "visual")]
use image::GenericImageView;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;

type RendererVec = Vec<Box<dyn Renderer<DynamicImage, Error = String>>>;

pub struct WfcApp {
    config: AppConfig,
}

impl WfcApp {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut tiles = match &self.config.input {
            Input::Image(image) => {
                let tile_size = self
                    .config
                    .input_size
                    .ok_or("--input-size is required to cut a sample image into tiles")?;

                Tile::from_image(image, &Size::uniform(tile_size))
            }
            Input::Config(configs) => Tile::from_config(configs),
        }?;

        info!("{} unique tiles found", tiles.len());

        let unplaceable = tiles.iter().filter(|tile| !tile.is_placeable()).count();

        if unplaceable > 0 {
            tiles.retain(Tile::is_placeable);
            warn!("Dropped {unplaceable} tiles with no allowed neighbour on some side");
        }

        let base_state = SuperState::new(tiles.iter().cloned().map(Arc::new).collect());
        let grid = Grid::new(
            self.config.output_size.width,
            self.config.output_size.height,
            &mut |_, _| base_state.clone(),
        );

        let seed = {
            #[cfg(not(feature = "threaded"))]
            {
                self.config.seed.unwrap_or_else(|| rand::rng().random())
            }

            #[cfg(feature = "threaded")]
            {
                rand::rng().random()
            }
        };

        info!("Using seed: {seed}");

        let mut wfc = Wave::new(grid, seed);

        let mut renderers: RendererVec = Vec::new();

        #[cfg(feature = "visual")]
        if self.config.renderer.visual
            && let Some(first_tile) = tiles.first()
        {
            let (tile_width, tile_height) = first_tile.value.dimensions();

            let sdl_config = SdlConfig {
                window_size: Size {
                    width: self.config.output_size.width * tile_width as usize,
                    height: self.config.output_size.height * tile_height as usize,
                },
                vsync: self.config.renderer.vsync,
                fullscreen: self.config.renderer.fullscreen,
                show_debug: self.config.renderer.debug,
                render_every_step: self.config.renderer.slow,
            };

            if let Ok(sdl_renderer) = SdlRenderer::new(&sdl_config) {
                renderers.push(Box::new(sdl_renderer));
            }
        }

        #[cfg(feature = "image")]
        if let Some(output_path) = &self.config.output_path {
            renderers.push(Box::new(ImageRenderer::new(output_path.clone())));
        }

        for renderer in &mut renderers {
            renderer.initialize(
                &tiles,
                (
                    self.config.output_size.width,
                    self.config.output_size.height,
                ),
            )?;
        }

        let max_progress = wfc.remaining() as u64;
        let progress = ProgressBar::new(max_progress);
        progress.enable_steady_tick(Duration::from_millis(200));
        progress.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>5}/{len} {per_sec:>12}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        #[cfg(feature = "visual")]
        let render_every_step = self.config.renderer.visual && self.config.renderer.slow;
        #[cfg(not(feature = "visual"))]
        let render_every_step = false;

        while !wfc.done() {
            progress.set_position(max_progress - wfc.remaining() as u64);

            if renderers.iter_mut().any(|r| r.should_quit()) {
                return Ok(());
            }

            for renderer in &mut renderers {
                renderer.update(&wfc)?;
            }

            // One unit of work per redraw shows the propagation front moving;
            // draining the whole stack per redraw is far faster to finish.
            let progressed = if render_every_step {
                wfc.step(1) > 0
            } else {
                wfc.tick()
            };

            if !progressed {
                warn!("Unable to make progress, stopping early");
                break;
            }
        }

        progress.finish();

        #[cfg(feature = "visual")]
        if let Some(delay) = self.config.renderer.hold {
            info!("Waiting for {delay} seconds");
            std::thread::sleep(Duration::from_secs_f32(delay));
        }

        for renderer in &mut renderers {
            renderer.finalize(&wfc)?;
        }

        info!("Generation completed");
        Ok(())
    }
}
