//! Wave function collapse as a solver a JavaScript host can drive.
//!
//! Tiles go in, grid state comes out. The solver knows nothing about images: a
//! tile is an index, a weight and the set of tiles allowed on each of its sides,
//! and what a tile looks like stays with the caller.

use js_sys::{Int32Array, Uint32Array};
use serde::Deserialize;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wave_function_collapse::{
    Collapsable, Direction, Grid, Neighbors, Set, SuperState, Tile, Wave,
};

/// `performance` and `Math` are globals in a worker as well as a window, so
/// reaching them directly keeps this module free of `web-sys` and usable off the
/// main thread.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn now() -> f64;

    #[wasm_bindgen(js_namespace = Math, js_name = random)]
    fn random() -> f64;
}

#[wasm_bindgen(start)]
fn on_load() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TILE: &str = r#"
/**
 * One tile. Each side lists the indices of the tiles allowed against it, an index
 * being a position in the array passed to the Solver. A side with an empty list
 * can never be placed, so such a tile is rejected.
 */
export interface Tile {
    readonly up: number[];
    readonly right: number[];
    readonly down: number[];
    readonly left: number[];
    /** How often this tile is drawn relative to the others. Defaults to 1. */
    readonly weight?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Tile[]")]
    pub type TileArray;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TileDefinition {
    up: Vec<i32>,
    right: Vec<i32>,
    down: Vec<i32>,
    left: Vec<i32>,
    #[serde(default = "single_weight")]
    weight: usize,
}

fn single_weight() -> usize {
    1
}

impl TileDefinition {
    fn side(&self, direction: Direction) -> &[i32] {
        match direction {
            Direction::Up => &self.up,
            Direction::Right => &self.right,
            Direction::Down => &self.down,
            Direction::Left => &self.left,
        }
    }

    fn into_tile(self, index: i32, tile_count: i32) -> Result<Tile<()>, JsError> {
        let out_of_range = [&self.up, &self.right, &self.down, &self.left]
            .into_iter()
            .flatten()
            .find(|&&neighbour| !(0..tile_count).contains(&neighbour));

        if let Some(&neighbour) = out_of_range {
            return Err(JsError::new(&format!(
                "tile {index} allows tile {neighbour}, but the tile set holds {tile_count}"
            )));
        }

        let neighbors: Neighbors<Set<u64>> = Neighbors::from_fn(|direction| {
            self.side(direction)
                .iter()
                .map(|&neighbour| neighbour as u64)
                .collect()
        });

        if neighbors.values().any(Set::is_empty) {
            return Err(JsError::new(&format!(
                "tile {index} has a side with no allowed neighbour, so it can never be placed"
            )));
        }

        Ok(Tile::with_neighbors(
            index as u64,
            (),
            self.weight,
            neighbors,
        ))
    }
}

/// A wave being solved. Step it as far as a frame can afford, read the grid, draw
/// it yourself.
#[wasm_bindgen]
pub struct Solver {
    wave: Wave<Tile<()>>,
    seed: u64,
    /// Collapsed tile index per cell, -1 while a cell is still open.
    cells: Vec<i32>,
    /// Remaining possibilities per cell.
    entropy: Vec<u32>,
    /// Cells whose entropy changed during the last step call.
    dirty: Vec<u32>,
    last_step_count: u32,
}

#[wasm_bindgen]
impl Solver {
    /// # Errors
    /// When the grid is empty, the tile array is empty or malformed, a tile names
    /// a neighbour that does not exist, or a tile has a side nothing may sit
    /// against.
    #[wasm_bindgen(constructor)]
    pub fn new(
        tiles: &TileArray,
        width: u32,
        height: u32,
        seed: Option<u64>,
    ) -> Result<Solver, JsError> {
        if width == 0 || height == 0 {
            return Err(JsError::new("the output grid needs a non-zero size"));
        }

        let definitions: Vec<TileDefinition> = serde_wasm_bindgen::from_value(tiles.into())
            .map_err(|error| JsError::new(&format!("could not read the tile array: {error}")))?;

        // A tile index has to survive the round trip through `cells`, where -1 is
        // taken to mean the cell has not collapsed.
        let tile_count = i32::try_from(definitions.len())
            .map_err(|_| JsError::new("that is more tiles than this solver can index"))?;

        if tile_count == 0 {
            return Err(JsError::new("the tile set is empty"));
        }

        let tiles = definitions
            .into_iter()
            .zip(0..tile_count)
            .map(|(definition, index)| definition.into_tile(index, tile_count))
            .collect::<Result<Vec<_>, JsError>>()?;

        let base = SuperState::new(tiles.into_iter().map(Arc::new).collect());
        let grid = Grid::new(width as usize, height as usize, &mut |_, _| base.clone());
        let seed = seed.unwrap_or_else(random_seed);
        let cell_count = grid.size();

        let mut solver = Solver {
            wave: Wave::new(grid, seed),
            seed,
            cells: vec![-1; cell_count],
            entropy: vec![0; cell_count],
            dirty: Vec::with_capacity(cell_count),
            last_step_count: 0,
        };

        solver.refresh();

        Ok(solver)
    }

    /// Solves for at most `budget` milliseconds of wall clock, then hands control back so the
    /// caller can draw and yield. `true` once the wave is finished.
    #[wasm_bindgen(js_name = stepFor)]
    pub fn step_for(&mut self, budget: f64) -> bool {
        /// Reading the clock per unit of work would cost more than the work does.
        const BETWEEN_CLOCK_READS: usize = 256;

        let deadline = now() + budget.max(0.0);
        let mut taken = 0;

        loop {
            let ran = self.wave.step(BETWEEN_CLOCK_READS);
            taken += ran;

            if ran < BETWEEN_CLOCK_READS || now() >= deadline {
                break;
            }
        }

        self.finish_step(taken)
    }

    /// Solves for at most `limit` units of work. `true` once the wave is finished.
    pub fn step(&mut self, limit: u32) -> bool {
        let taken = self.wave.step(limit as usize);

        self.finish_step(taken)
    }

    /// Solves the whole wave without yielding, giving up after `restartBudget`
    /// restarts. Call it from a worker, or for a grid small enough that one blocked
    /// frame does not show.
    pub fn run(&mut self, restart_budget: u32) -> bool {
        let before = self.wave.remaining();
        self.wave.run(restart_budget as usize);

        self.finish_step(before - self.wave.remaining())
    }

    /// Starts over, keeping the tile set. Omit `seed` to draw a new one.
    pub fn reset(&mut self, seed: Option<u64>) {
        self.seed = seed.unwrap_or_else(random_seed);
        self.wave.reseed(self.seed);
        self.invalidate();
    }

    /// Collapsed tile index per cell, -1 where the cell is still open. Row major,
    /// so a cell sits at `x + y * gridWidth`. A snapshot, safe to hold on to.
    #[must_use]
    pub fn cells(&self) -> Int32Array {
        Int32Array::from(self.cells.as_slice())
    }

    /// Remaining possibilities per cell. Divide by `baseEntropy` to shade a cell by
    /// how far it has narrowed down.
    #[must_use]
    pub fn entropy(&self) -> Uint32Array {
        Uint32Array::from(self.entropy.as_slice())
    }

    /// Cells that changed during the last step, so only those need redrawing.
    /// Lists every cell after construction, `reset` and `invalidate`.
    #[must_use]
    #[wasm_bindgen(js_name = dirtyCells)]
    pub fn dirty_cells(&self) -> Uint32Array {
        Uint32Array::from(self.dirty.as_slice())
    }

    /// Marks every cell as needing a redraw, for instance after a canvas resize.
    pub fn invalidate(&mut self) {
        self.cells.fill(-1);
        self.entropy.fill(0);
        self.refresh();
        self.dirty = (0..self.total()).collect();
    }

    #[must_use]
    #[wasm_bindgen(getter, js_name = gridWidth)]
    pub fn grid_width(&self) -> u32 {
        self.wave.width() as u32
    }

    #[must_use]
    #[wasm_bindgen(getter, js_name = gridHeight)]
    pub fn grid_height(&self) -> u32 {
        self.wave.height() as u32
    }

    /// How many possibilities a cell starts with.
    #[must_use]
    #[wasm_bindgen(getter, js_name = baseEntropy)]
    pub fn base_entropy(&self) -> u32 {
        u32::try_from(self.wave.base_entropy()).unwrap_or(u32::MAX)
    }

    /// The seed in use, so a result can be reproduced.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn done(&self) -> bool {
        self.wave.done()
    }

    /// How many times the solver gave up on rolling back and started the grid over.
    /// A tile set that cannot tile the grid drives this up and never finishes.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn restarts(&self) -> u32 {
        u32::try_from(self.wave.restarts()).unwrap_or(u32::MAX)
    }

    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn collapsed(&self) -> u32 {
        self.total() - u32::try_from(self.wave.remaining()).unwrap_or(0)
    }

    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn total(&self) -> u32 {
        u32::try_from(self.cells.len()).unwrap_or(u32::MAX)
    }

    /// Fraction of cells collapsed, 0 to 1. Moves backwards when the solver hits a
    /// contradiction and rolls back.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn progress(&self) -> f64 {
        if self.cells.is_empty() {
            return 1.0;
        }

        f64::from(self.collapsed()) / f64::from(self.total())
    }

    /// Units of work the last step call got through. Zero while `done` is false
    /// means the wave is stuck.
    #[must_use]
    #[wasm_bindgen(getter, js_name = lastStepCount)]
    pub fn last_step_count(&self) -> u32 {
        self.last_step_count
    }
}

impl Solver {
    fn finish_step(&mut self, taken: usize) -> bool {
        self.last_step_count = u32::try_from(taken).unwrap_or(u32::MAX);
        self.refresh();

        self.wave.done()
    }

    /// One pass over the grid, recording what changed. Diffing the whole grid
    /// rather than tracking writes is what keeps the dirty set right across a
    /// rollback, which moves cells this step never touched.
    fn refresh(&mut self) {
        self.dirty.clear();

        for (index, (_, _, cell)) in self.wave.grid().iter().enumerate() {
            let entropy = u32::try_from(cell.entropy()).unwrap_or(u32::MAX);

            if self.entropy[index] == entropy {
                continue;
            }

            self.entropy[index] = entropy;
            self.cells[index] = cell.collapsed().map_or(-1, |tile| {
                i32::try_from(tile.get_id()).expect(
                    "tile ids are indices bounded by the tile count the constructor checked",
                )
            });
            self.dirty
                .push(u32::try_from(index).unwrap_or_else(|_| unreachable!("index fits the grid")));
        }
    }
}

/// A seed drawn from two `Math.random()` calls. One call cannot fill 64 bits: an
/// f64 scaled to `u64::MAX` only ever lands on a multiple of 2048.
#[must_use]
#[wasm_bindgen(js_name = randomSeed)]
pub fn random_seed() -> u64 {
    let bits = |draw: f64| (draw * f64::from(u32::MAX)) as u64;

    (bits(random()) << 32) | bits(random())
}
