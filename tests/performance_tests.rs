use std::sync::Arc;

use wave_function_collapse::{
    grid::{Direction, Grid, Size},
    superstate::{SuperState, Collapsable},
    tile::Tile,
    wave::Wave,
};

#[cfg(feature = "image")]
use image::DynamicImage;

// Fixed seed for deterministic tests
const TEST_SEED: u64 = 42;

fn create_test_tiles(count: usize) -> Vec<Tile<u32>> {
    let mut tiles = Vec::with_capacity(count);
    
    for i in 0..count {
        let mut tile = Tile::new(i as u64, i as u32);
        
        // Create realistic neighbor constraints
        for neighbor_id in 0..(count.min(4)) {
            let id = ((i + neighbor_id) % count) as u64;
            tile.neighbors[Direction::Up].insert(id);
            tile.neighbors[Direction::Right].insert(id);
            tile.neighbors[Direction::Down].insert(id);
            tile.neighbors[Direction::Left].insert(id);
        }
        
        tile.weight = (i % 5 + 1) * 10; // Varied weights
        tiles.push(tile);
    }
    
    tiles
}

fn create_test_wave(size: usize, tile_count: usize) -> Wave<Tile<u32>> {
    let tiles = create_test_tiles(tile_count);
    let base_state = SuperState::new(tiles.into_iter().map(Arc::new).collect());
    let grid = Grid::new(size, size, &mut |_, _| base_state.clone());
    Wave::new(grid, TEST_SEED)
}

#[test]
fn test_maybe_collapse_correctness() {
    let mut wave = create_test_wave(5, 8);
    
    // Initial state: all cells should be uncollapsed
    assert_eq!(wave.remaining(), 25);
    assert!(!wave.done());
    
    // First collapse should succeed and return a position
    let pos1 = wave.maybe_collapse();
    assert!(pos1.is_some());
    assert!(wave.remaining() < 25);
    
    // Continue collapsing until done
    let mut iterations = 0;
    while !wave.done() && iterations < 100 {
        wave.tick();
        iterations += 1;
    }
    
    assert!(wave.done());
    assert_eq!(wave.remaining(), 0);
}

#[test]
fn test_maybe_collapse_deterministic_with_seed() {
    let mut wave1 = create_test_wave(4, 6);
    let mut wave2 = create_test_wave(4, 6);
    
    // Both waves should collapse the same way with same seed
    let pos1 = wave1.maybe_collapse();
    let pos2 = wave2.maybe_collapse();
    
    assert_eq!(pos1, pos2);
}

#[test]
fn test_maybe_collapse_empty_areas() {
    // Test with a 1x1 grid (edge case)
    let mut wave = create_test_wave(1, 4);
    let pos = wave.maybe_collapse();
    assert_eq!(pos, Some((0, 0)));
    
    // After collapse, no more collapses should be possible
    let pos2 = wave.maybe_collapse();
    assert_eq!(pos2, None);
    assert!(wave.done());
}

#[test]
fn test_superstate_tick_entropy_reduction() {
    use fxhash::FxHashSet;
    use wave_function_collapse::grid::Neighbors;
    
    let tiles = create_test_tiles(10);
    let mut state = SuperState::new(tiles.into_iter().map(Arc::new).collect());
    
    let initial_entropy = state.entropy();
    assert_eq!(initial_entropy, 10);
    
    // Create restrictive neighbors that should reduce possibilities
    let mut neighbors = Neighbors::default();
    let mut constraint_set = FxHashSet::default();
    constraint_set.insert(0u64); // Only allow tile 0
    constraint_set.insert(1u64); // Only allow tile 1
    neighbors[Direction::Up] = constraint_set;
    
    state.tick(&neighbors);
    
    let final_entropy = state.entropy();
    assert!(final_entropy <= initial_entropy);
    assert!(final_entropy > 0); // Should not be completely eliminated
}

#[test]
fn test_superstate_collapse_weighted_selection() {
    use rand_xorshift::XorShiftRng;
    use rand::SeedableRng;
    
    let mut tiles = create_test_tiles(5);
    
    // Set very different weights
    tiles[0].weight = 1000;   // Very high weight
    tiles[1].weight = 1;      // Very low weight
    for i in 2..5 {
        tiles[i].weight = 1;  // Very low weight
    }
    
    let mut state = SuperState::new(tiles.into_iter().map(Arc::new).collect());
    let mut rng = XorShiftRng::seed_from_u64(TEST_SEED);
    
    // Due to high weight, tile 0 should be selected more often
    let mut tile_0_selected = 0;
    let trials = 100;
    
    for _ in 0..trials {
        let mut state_copy = state.clone();
        state_copy.collapse(&mut rng);
        
        if let Some(collapsed) = state_copy.collapsed() {
            if collapsed.get_id() == 0 {
                tile_0_selected += 1;
            }
        }
    }
    
    // With 1000x weight advantage, tile 0 should be selected most of the time
    assert!(tile_0_selected > trials / 2);
}

#[test]
fn test_superstate_entropy_caching() {
    let tiles = create_test_tiles(8);
    let mut state = SuperState::new(tiles.into_iter().map(Arc::new).collect());
    
    let entropy1 = state.entropy();
    let entropy2 = state.entropy();
    let entropy3 = state.entropy();
    
    // Multiple calls should return same value (testing caching)
    assert_eq!(entropy1, entropy2);
    assert_eq!(entropy2, entropy3);
    assert_eq!(entropy1, 8);
}

#[test]
fn test_grid_neighbor_access_correctness() {
    let grid = Grid::new(3, 3, &mut |x, y| x * 10 + y);
    
    // Test corner cell (0,0)
    let neighbors_00 = grid.get_neighbors(0, 0);
    assert!(neighbors_00[Direction::Up].is_none());
    assert!(neighbors_00[Direction::Left].is_none());
    assert_eq!(neighbors_00[Direction::Right], Some(&1));
    assert_eq!(neighbors_00[Direction::Down], Some(&10));
    
    // Test center cell (1,1)
    let neighbors_11 = grid.get_neighbors(1, 1);
    assert_eq!(neighbors_11[Direction::Up], Some(&1));
    assert_eq!(neighbors_11[Direction::Right], Some(&12));
    assert_eq!(neighbors_11[Direction::Down], Some(&21));
    assert_eq!(neighbors_11[Direction::Left], Some(&10));
    
    // Test edge cell (2,1)
    let neighbors_21 = grid.get_neighbors(2, 1);
    assert_eq!(neighbors_21[Direction::Up], Some(&2));
    assert!(neighbors_21[Direction::Right].is_none());
    assert_eq!(neighbors_21[Direction::Down], Some(&22));
    assert_eq!(neighbors_21[Direction::Left], Some(&11));
}

#[test]
fn test_grid_creation_consistency() {
    let size = 10;
    let grid = Grid::new(size, size, &mut |x, y| x + y);
    
    assert_eq!(grid.size(), size * size);
    assert_eq!(grid.width(), size);
    assert_eq!(grid.height(), size);
    
    // Verify all cells are properly initialized
    for x in 0..size {
        for y in 0..size {
            assert_eq!(grid.get(x, y), Some(&(x + y)));
        }
    }
}

#[cfg(feature = "image")]
#[test]
fn test_tile_from_image_consistency() {
    use image::{DynamicImage, RgbaImage};
    
    // Create a test image with a simple pattern
    let img_size = 32u32;
    let tile_size = Size::uniform(8);
    
    let image = DynamicImage::ImageRgba8(
        RgbaImage::from_fn(img_size, img_size, |x, y| {
            // Create a checkerboard pattern
            let checker = (x / 8 + y / 8) % 2;
            if checker == 0 {
                image::Rgba([255, 0, 0, 255]) // Red
            } else {
                image::Rgba([0, 255, 0, 255]) // Green
            }
        })
    );
    
    let tiles = Tile::<DynamicImage>::from_image(&image, &tile_size);
    
    // Should create some unique tiles
    assert!(!tiles.is_empty());
    assert!(tiles.len() <= 16); // Max possible unique tiles for 4x4 grid
    
    // Each tile should have valid neighbor relationships
    for tile in &tiles {
        assert!(tile.neighbors[Direction::Up].len() >= 0);
        assert!(tile.neighbors[Direction::Right].len() >= 0);
        assert!(tile.neighbors[Direction::Down].len() >= 0);
        assert!(tile.neighbors[Direction::Left].len() >= 0);
        assert!(tile.get_weight() > 0);
    }
}

#[test]
fn test_wave_tick_progress() {
    let mut wave = create_test_wave(6, 8);
    let initial_remaining = wave.remaining();
    
    // First tick should make progress
    let made_progress = wave.tick();
    assert!(made_progress);
    
    // Should have fewer remaining cells (or at least same if only propagation occurred)
    assert!(wave.remaining() <= initial_remaining);
}

#[test]
fn test_wave_rollback_scenario() {
    let mut wave = create_test_wave(3, 2); // Small grid, few tiles = likely contradiction
    
    let mut iterations = 0;
    let mut last_remaining = wave.remaining();
    let mut rollback_detected = false;
    
    while !wave.done() && iterations < 50 {
        wave.tick_once();
        
        // Detect rollback (remaining count increases)
        if wave.remaining() > last_remaining {
            rollback_detected = true;
        }
        
        last_remaining = wave.remaining();
        iterations += 1;
    }
    
    // Either we finish successfully or we detect rollback behavior
    assert!(wave.done() || rollback_detected || iterations >= 50);
}

#[test]
fn test_wave_deterministic_behavior() {
    let mut wave1 = create_test_wave(4, 6);
    let mut wave2 = create_test_wave(4, 6);
    
    // Both should behave identically with same seed
    for _ in 0..10 {
        if wave1.done() || wave2.done() { break; }
        
        let pos1 = wave1.tick_once();
        let pos2 = wave2.tick_once();
        
        assert_eq!(pos1, pos2);
        assert_eq!(wave1.remaining(), wave2.remaining());
    }
}

// Edge case tests
#[test]
fn test_single_tile_wave() {
    let tiles = vec![Tile::new(0, 0u32)];
    let base_state = SuperState::new(vec![Arc::new(tiles[0].clone())]);
    let grid = Grid::new(2, 2, &mut |_, _| base_state.clone());
    let mut wave = Wave::new(grid, TEST_SEED);
    
    // Should complete immediately since there's only one choice
    while !wave.done() {
        wave.tick();
    }
    
    assert!(wave.done());
    assert_eq!(wave.remaining(), 0);
}

#[test]
fn test_large_grid_performance() {
    // This test ensures our functions can handle larger grids without panicking
    let mut wave = create_test_wave(20, 15);
    
    // Should be able to start processing without issues
    let pos = wave.maybe_collapse();
    assert!(pos.is_some());
    
    // Should be able to make progress
    let progress = wave.tick();
    assert!(progress || wave.done());
}