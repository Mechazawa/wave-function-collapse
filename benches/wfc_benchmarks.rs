use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use rand::SeedableRng;

use wave_function_collapse::{
    grid::{Grid, Size},
    superstate::SuperState,
    tile::Tile,
    wave::Wave,
};

#[cfg(feature = "image")]
use wave_function_collapse::sprite::Sprite;

// Fixed seed for deterministic benchmarks
const BENCHMARK_SEED: u64 = 12345;

fn create_test_tiles() -> Vec<Tile<u32>> {
    // Create simple test tiles with deterministic properties
    let mut tiles = Vec::with_capacity(16);
    for i in 0..16 {
        let mut tile = Tile::new(i as u64, i as u32);
        
        // Add some neighbor relationships for realistic constraints
        for neighbor_id in 0..4 {
            tile.neighbors[wave_function_collapse::grid::Direction::Up].insert((i + neighbor_id) % 16);
            tile.neighbors[wave_function_collapse::grid::Direction::Right].insert((i + neighbor_id + 1) % 16);
            tile.neighbors[wave_function_collapse::grid::Direction::Down].insert((i + neighbor_id + 2) % 16);
            tile.neighbors[wave_function_collapse::grid::Direction::Left].insert((i + neighbor_id + 3) % 16);
        }
        
        tile.weight = ((i % 4) + 1) as usize * 10; // Varied weights for realistic collapse behavior
        tiles.push(tile);
    }
    tiles
}

fn create_test_wave(size: usize) -> Wave<Tile<u32>> {
    let tiles = create_test_tiles();
    let base_state = SuperState::new(tiles.into_iter().map(Arc::new).collect());
    let grid = Grid::new(size, size, &mut |_, _| base_state.clone());
    Wave::new(grid, BENCHMARK_SEED)
}

fn create_partially_collapsed_wave(size: usize, collapse_ratio: f32) -> Wave<Tile<u32>> {
    let mut wave = create_test_wave(size);
    let total_cells = size * size;
    let cells_to_collapse = (total_cells as f32 * collapse_ratio) as usize;
    
    // Collapse some cells to create a realistic intermediate state
    for _ in 0..cells_to_collapse {
        if wave.done() { break; }
        wave.tick_once();
    }
    
    wave
}

fn bench_maybe_collapse(c: &mut Criterion) {
    let mut group = c.benchmark_group("maybe_collapse");
    
    for size in [10, 20, 50].iter() {
        group.bench_with_input(format!("size_{}", size), size, |b, &size| {
            b.iter_batched(
                || create_partially_collapsed_wave(size, 0.3), // 30% collapsed
                |mut wave| black_box(wave.maybe_collapse()),
                criterion::BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

fn bench_superstate_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("superstate_tick");
    
    let tiles = create_test_tiles();
    let neighbors = wave_function_collapse::grid::Neighbors::default(); // Empty neighbors for consistent timing
    
    group.bench_function("tick_many_possibilities", |b| {
        b.iter_batched(
            || SuperState::new(tiles.iter().cloned().map(Arc::new).collect()),
            |mut state| black_box(state.tick(&neighbors)),
            criterion::BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

fn bench_superstate_collapse(c: &mut Criterion) {
    let mut group = c.benchmark_group("superstate_collapse");
    
    let tiles = create_test_tiles();
    
    group.bench_function("collapse_weighted", |b| {
        b.iter_batched(
            || {
                let rng = rand_xorshift::XorShiftRng::seed_from_u64(BENCHMARK_SEED);
                let state = SuperState::new(tiles.iter().cloned().map(Arc::new).collect());
                (state, rng)
            },
            |(mut state, mut rng)| black_box(state.collapse(&mut rng)),
            criterion::BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

fn bench_grid_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_operations");
    
    for size in [25, 50, 100].iter() {
        group.bench_with_input(format!("grid_creation_{}", size), size, |b, &size| {
            b.iter(|| {
                black_box(Grid::new(size, size, &mut |x, y| x + y))
            });
        });
        
        group.bench_with_input(format!("grid_neighbors_{}", size), size, |b, &size| {
            let grid = Grid::new(size, size, &mut |x, y| x + y);
            b.iter(|| {
                for x in 0..size {
                    for y in 0..size {
                        black_box(grid.get_neighbors(x, y));
                    }
                }
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "image")]
fn bench_tile_from_image(c: &mut Criterion) {
    use image::{DynamicImage, RgbaImage};
    
    let mut group = c.benchmark_group("tile_from_image");
    
    // Create test images of various sizes
    for (name, img_size, tile_size) in [
        ("small", 64, 8),
        ("medium", 128, 16),
        ("large", 256, 32),
    ].iter() {
        let image = DynamicImage::ImageRgba8(RgbaImage::new(*img_size, *img_size));
        let size = Size::uniform(*tile_size);
        
        group.bench_with_input(*name, &(image, size), |b, (img, tile_size)| {
            b.iter(|| black_box(Tile::<Sprite>::from_image(img, tile_size)));
        });
    }
    
    group.finish();
}

fn bench_wave_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("wave_tick");
    
    for size in [15, 25, 35].iter() {
        group.bench_with_input(format!("single_tick_{}", size), size, |b, &size| {
            b.iter_batched(
                || create_partially_collapsed_wave(size, 0.2), // 20% collapsed
                |mut wave| black_box(wave.tick_once()),
                criterion::BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3))
}

#[cfg(feature = "image")]
criterion_group!(
    name = benches;
    config = configure_criterion();
    targets = 
        bench_maybe_collapse,
        bench_superstate_tick,
        bench_superstate_collapse,
        bench_grid_operations,
        bench_tile_from_image,
        bench_wave_tick
);

#[cfg(not(feature = "image"))]
criterion_group!(
    name = benches;
    config = configure_criterion();
    targets = 
        bench_maybe_collapse,
        bench_superstate_tick,
        bench_superstate_collapse,
        bench_grid_operations,
        bench_wave_tick
);

criterion_main!(benches);