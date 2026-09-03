use fxhash::FxHashSet;
use std::collections::VecDeque;

use log::{trace, warn};
use rand::prelude::IndexedRandom;
use rand::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;

use crate::grid::{Grid, Neighbors, Position};
use crate::superstate::{Collapsable, SuperState};

type CellNeighbors<T> = Option<Neighbors<Set<<T as Collapsable>::Identifier>>>;
pub type Set<T> = FxHashSet<T>;

#[derive(Debug, PartialEq, Eq)]
enum CollapseReason {
    Implicit,
    Explicit,
}

pub struct Wave<T>
where
    T: Collapsable,
{
    grid: Grid<SuperState<T>>,
    grid_base: Grid<SuperState<T>>,
    stack: VecDeque<Position>,
    data: Grid<CellNeighbors<T>>,
    // todo remove the CollapseReason because it's unused
    collapsed: Vec<(Position, CollapseReason)>,
    rng: Box<dyn RngCore>,
    seed: u64,
    last_rollback: usize,
    rollback_penalty: f64,
}

impl<T> Wave<T>
where
    T: Collapsable,
{
    #[must_use]
    pub fn new(grid: Grid<SuperState<T>>, seed: u64) -> Self {
        Self {
            stack: VecDeque::with_capacity(grid.size()),
            collapsed: Vec::with_capacity(grid.size()),
            data: Grid::new(grid.width(), grid.height(), &mut |_, _| Option::default()),
            grid_base: grid.clone(),
            grid,
            rng: Box::new(XorShiftRng::seed_from_u64(seed)),
            seed,
            last_rollback: 0,
            rollback_penalty: 0.0,
        }
    }

    #[must_use]
    pub fn grid(&self) -> &Grid<SuperState<T>> {
        &self.grid
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.grid.width()
    }

    #[must_use]
    pub fn height(&self) -> usize {
        self.grid.height()
    }

    /// The state every cell starts in, holding the whole tile set. `None` for an
    /// empty grid.
    #[must_use]
    pub fn base_state(&self) -> Option<&SuperState<T>> {
        self.grid_base.get(0, 0)
    }

    /// How many possibilities a cell starts with. Zero for an empty grid.
    #[must_use]
    pub fn base_entropy(&self) -> usize {
        self.base_state().map_or(0, SuperState::base_entropy)
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub fn done(&self) -> bool {
        self.remaining() == 0
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.grid.size() - self.collapsed.len()
    }

    pub fn tick(&mut self) -> bool {
        let mut worked = false;

        while let Some((x, y)) = self.stack.pop_front() {
            self.tick_cell(x, y);
            worked = true;
        }

        worked || self.maybe_collapse().is_some()
    }

    /// Advances at most `limit` units of work and returns how many ran. A shortfall
    /// means the wave either finished or cannot collapse any cell.
    pub fn step(&mut self, limit: usize) -> usize {
        (0..limit)
            .take_while(|_| self.tick_once().is_some())
            .count()
    }

    /// Collapses the whole wave. Returns early when no cell can be collapsed, so
    /// check `done` to tell a solved wave from a stuck one.
    pub fn run(&mut self) {
        while !self.done() && self.tick() {}
    }

    pub fn tick_once(&mut self) -> Option<Position> {
        if let Some((x, y)) = self.stack.pop_front() {
            self.tick_cell(x, y);

            Some((x, y))
        } else {
            self.maybe_collapse()
        }
    }

    fn tick_cell(&mut self, x: usize, y: usize) {
        if self
            .grid
            .get(x, y)
            .expect("position came from the grid")
            .entropy()
            == 1
        {
            return;
        }

        if self
            .data
            .get(x, y)
            .expect("position came from the grid")
            .is_none()
        {
            let data = self.grid.get_neighbors(x, y).map(|_, v| match v {
                None => Set::default(),
                Some(neighbor) => neighbor.possible.iter().map(|x| x.get_id()).collect(),
            });

            self.data
                .set(x, y, Some(data))
                .expect("position came from the grid");
        }

        let neighbors = self
            .data
            .replace(x, y, None)
            .expect("position came from the grid")
            .expect("the block above stores the neighbours when they are missing");

        let cell = self
            .grid
            .get_mut(x, y)
            .expect("position came from the grid");
        let old_entropy = cell.entropy();

        cell.tick(&neighbors);

        if cell.entropy() <= 1 {
            self.collapsed.push(((x, y), CollapseReason::Implicit));
        }

        if cell.entropy() == 0 {
            self.smart_rollback();
        } else if old_entropy != cell.entropy() {
            if cell.collapsing()
                && self
                    .grid
                    .get_neighbors(x, y)
                    .values()
                    .all(|v| v.map(|v| !v.collapsing()).unwrap_or(true))
            {
                self.collapse(x, y);
            } else {
                self.mark(x, y);
            }
        }
    }

    fn collapse(&mut self, x: usize, y: usize) {
        self.grid
            .get_mut(x, y)
            .expect("position came from the grid")
            .collapse(&mut self.rng);
        self.collapsed.push(((x, y), CollapseReason::Explicit));
        self.mark(x, y);
    }

    /// Attempts to collapse a cell with the lowest entropy in the smallest collapsable area.
    /// Returns the position of the collapsed cell, or None if no such cell exists.
    pub fn maybe_collapse(&mut self) -> Option<Position> {
        let areas = self.collapsable_areas();
        let first_area = areas.first()?;

        // Single-pass algorithm to find minimum entropy and collect candidates
        let mut min_entropy = usize::MAX;
        let mut candidates = Vec::new();

        for &(x, y) in first_area {
            let entropy = self.grid.get(x, y).map_or(1, SuperState::entropy);

            if entropy <= 1 {
                continue; // Skip collapsed/invalid cells
            }

            if entropy < min_entropy {
                min_entropy = entropy;
                candidates.clear();
                candidates.push((x, y));
            } else if entropy == min_entropy {
                candidates.push((x, y));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        candidates.choose(&mut self.rng).map(|&(x, y)| {
            self.collapse(x, y);
            (x, y)
        })
    }

    fn mark(&mut self, cx: usize, cy: usize) {
        let possible_states: Set<T::Identifier> = self
            .grid
            .get(cx, cy)
            .expect("position came from the grid")
            .possible
            .iter()
            .map(|t| t.get_id())
            .collect();

        // Collected up front so the loop below can borrow self.data mutably.
        let neighbor_positions: Vec<_> = self
            .data
            .get_neighbor_positions(cx, cy)
            .into_iter()
            .filter_map(|(dir, pos)| pos.map(|p| (dir, p)))
            .collect();

        for (direction, (x, y)) in neighbor_positions {
            match self
                .data
                .get_mut(x, y)
                .expect("position came from the grid")
            {
                None => {
                    let mut neighbors: Neighbors<Set<T::Identifier>> = Neighbors::default();
                    neighbors[direction.invert()].clone_from(&possible_states);
                    self.data
                        .set(x, y, Some(neighbors))
                        .expect("position came from the grid");
                    self.stack.push_back((x, y));
                }
                Some(neighbors) => {
                    neighbors[direction.invert()].clone_from(&possible_states);
                }
            }
        }
    }

    fn smart_rollback(&mut self) {
        let collapsed_count = self.grid.size() - self.remaining();

        trace!("Collapsed: {collapsed_count}");

        if collapsed_count <= self.last_rollback {
            self.rollback_penalty += 1.0;
        } else {
            self.last_rollback = collapsed_count;
            self.rollback_penalty = 0.1;
        }

        let collapsed_count = self
            .collapsed
            .iter()
            .filter(|((_, _), c)| *c == CollapseReason::Explicit)
            .count();

        // Todo replace the rollback_penalty with a usize instead of using floats
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        if collapsed_count < self.rollback_penalty.ceil() as usize {
            warn!("Unable to solve, resetting...");

            self.reset();
            self.reset_rollback_penalty();
        } else {
            // Todo replace the rollback_penalty with a usize instead of using floats
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let rollback_amount = self.rollback_penalty.ceil() as usize;
            self.rollback(rollback_amount);
        }
    }

    fn reset(&mut self) {
        self.grid.clone_from(&self.grid_base);
        self.data.reset_to_default();
        self.collapsed.clear();
        self.stack.clear();
    }

    fn reset_rollback_penalty(&mut self) {
        self.rollback_penalty = 0.5;
        self.last_rollback = 0;
    }

    fn rollback(&mut self, mut count: usize) {
        trace!("Rollback {count}");

        if count == 0 {
            return;
        }

        self.data.reset_to_default();

        // revert last step of collapse stack
        while let Some((_, reason)) = self.collapsed.pop() {
            if reason == CollapseReason::Explicit {
                count -= 1;

                if count == 0 {
                    break;
                }
            }
        }

        self.resettle();
    }

    fn collapsable_areas(&self) -> Vec<Vec<Position>> {
        let mut board = Grid::<bool>::new(self.grid.width(), self.grid.height(), &mut |x, y| {
            self.grid.get(x, y).is_some_and(|cell| cell.entropy() == 1)
        });

        let mut stack: Vec<Position> = Vec::default();
        let mut output: Vec<Vec<Position>> = Vec::default();

        for bx in 0..board.width() {
            for by in 0..board.height() {
                if *board.get(bx, by).unwrap_or(&true) {
                    continue;
                }

                stack.push((bx, by));

                let mut area: Vec<Position> = Vec::new();

                while let Some((x, y)) = stack.pop() {
                    if *board.get(x, y).unwrap_or(&true) {
                        continue;
                    }

                    board.set(x, y, true).expect("position came from the grid");

                    board
                        .get_neighbor_positions(x, y)
                        .values()
                        .filter_map(|v| *v)
                        .for_each(|v| stack.push(v));

                    area.push((x, y));
                }

                output.push(area);
            }
        }

        output.sort_by_key(Vec::len);

        output
    }

    fn resettle(&mut self) {
        let explicit_collapsed: Vec<(Position, T::Identifier)> = self
            .collapsed
            .iter()
            .filter(|(_, reason)| *reason == CollapseReason::Explicit)
            .filter_map(|&((x, y), _)| Some(((x, y), self.grid.get(x, y)?.collapsed()?.get_id())))
            .collect();

        self.reset();

        for ((x, y), id) in explicit_collapsed {
            let coerced = self.grid.get_mut(x, y).is_some_and(|cell| cell.coerce(id));

            if !coerced {
                warn!("Failed to coerce cell at ({x}, {y})");
                continue;
            }

            self.collapsed.push(((x, y), CollapseReason::Explicit));
            self.mark(x, y);
        }
    }
}
