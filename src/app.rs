use crate::cli::{AppConfig, Input};
use crate::grid::{Grid, Size};
use crate::render::{Renderer, RenderEvent};
use crate::superstate::SuperState;
use crate::tile::Tile;
use crate::wave::Wave;

#[cfg(feature = "visual")]
use crate::render::sdl_renderer::{SdlRenderer, SdlConfig};

#[cfg(feature = "image-output")]
use crate::render::image_renderer::ImageRenderer;

use image::{DynamicImage, GenericImageView};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rand::rngs::OsRng;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;

pub struct WfcApp {
    config: AppConfig,
}

impl WfcApp {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Load tiles
        let mut tiles = match &self.config.input {
            Input::Image(value) => Tile::from_image(value, &Size::uniform(self.config.input_size)),
            Input::Config(value) => Tile::from_config(value),
        };

        info!("{} unique tiles found", tiles.len());

        // Filter invalid tiles
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

        // Create WFC state
        let base_state = SuperState::new(tiles.iter().cloned().map(Arc::new).collect());
        let grid = Grid::new(
            self.config.output_size.width,
            self.config.output_size.height,
            &mut |_, _| base_state.clone(),
        );

        let seed = {
            #[cfg(not(feature = "threaded"))]
            { self.config.seed.unwrap_or_else(|| OsRng.gen()) }

            #[cfg(feature = "threaded")]
            { OsRng.gen() }
        };

        info!("Using seed: {}", seed);

        let mut wfc = Wave::new(grid, seed);

        // Set up renderers (now that we have tiles with actual dimensions)
        let mut renderers = self.create_renderers(&tiles)?;

        // Initialize all renderers
        for renderer in &mut renderers {
            renderer.initialize(&tiles, (self.config.output_size.width, self.config.output_size.height))?;
        }

        // Progress bar
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

        // Emit start event
        let start_event = RenderEvent::Started;
        
        for renderer in &mut renderers {
            renderer.handle_event(&start_event)?;
        }

        // Main generation loop
        while !wfc.done() {
            progress.set_position(max_progress - wfc.remaining() as u64);

            // Check if any renderer wants to quit
            if renderers.iter_mut().any(|r| r.should_quit()) {
                return Ok(());
            }

            // Emit progress event
            let progress_event = RenderEvent::Progress;
            
            for renderer in &mut renderers {
                renderer.handle_event(&progress_event)?;
                renderer.update(&wfc)?;
            }

            // Perform WFC step
            #[cfg(feature = "visual")]
            if self.config.renderer.visual && self.config.renderer.slow {
                wfc.tick_once();
            } else {
                wfc.tick();
            }

            #[cfg(not(feature = "visual"))]
            wfc.tick();
        }

        // Emit completion event
        let completion_event = RenderEvent::Completed;
        
        for renderer in &mut renderers {
            renderer.handle_event(&completion_event)?;
        }

        progress.finish();

        // Hold visualization if requested
        #[cfg(feature = "visual")]
        if let Some(delay) = self.config.renderer.hold {
            info!("Waiting for {} seconds", delay);
            std::thread::sleep(Duration::from_secs_f32(delay));
        }

        // Finalize all renderers
        for renderer in &mut renderers {
            renderer.finalize(&wfc)?;
        }

        info!("Generation completed");
        Ok(())
    }

    fn create_renderers(&self, tiles: &[Tile<DynamicImage>]) -> Result<Vec<Box<dyn Renderer<DynamicImage, Error = String>>>, Box<dyn std::error::Error>> {
        let mut renderers: Vec<Box<dyn Renderer<DynamicImage, Error = String>>> = Vec::new();

        // Add SDL2 renderer if requested
        #[cfg(feature = "visual")]
        if self.config.renderer.visual {
            if let Some(first_tile) = tiles.first() {
                // Get the actual tile dimensions
                let (tile_width, tile_height) = first_tile.value.as_ref().dimensions();
                
                // Calculate window size based on actual tile size
                let window_width = self.config.output_size.width * tile_width as usize;
                let window_height = self.config.output_size.height * tile_height as usize;
                
                let window_size = Size {
                    width: window_width,
                    height: window_height,
                };

                let sdl_config = SdlConfig {
                    window_size,
                    vsync: self.config.renderer.vsync,
                    fullscreen: self.config.renderer.fullscreen,
                    show_debug: self.config.renderer.debug,
                    render_every_step: self.config.renderer.slow,
                };

                let sdl_renderer = SdlRenderer::new(sdl_config)?;
                renderers.push(Box::new(sdl_renderer));
            }
        }

        // Add image renderer if output path is specified
        #[cfg(feature = "image-output")]
        if let Some(output_path) = &self.config.output_path {
            let image_renderer = ImageRenderer::new(output_path.clone());
            renderers.push(Box::new(image_renderer));
        }

        Ok(renderers)
    }

}