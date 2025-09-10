use std::collections::VecDeque;
use fxhash::FxHashSet;

use log::{trace, warn};
use rand::seq::SliceRandom;
use rand::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;

use crate::grid::{Direction, Grid, Neighbors, Position};
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
    pub grid: Grid<SuperState<T>>,
    grid_base: Grid<SuperState<T>>,
    stack: VecDeque<Position>,
    // todo tmp pub
    pub data: Grid<CellNeighbors<T>>,
    collapsed: Vec<(Position, CollapseReason)>,
    rng: Box<dyn RngCore>,
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
            data: Grid::new(grid.width(), grid.height(), &mut |_, _| Default::default()),
            grid_base: grid.clone(),
            grid,
            rng: Box::new(XorShiftRng::seed_from_u64(seed)),
            last_rollback: 0,
            rollback_penalty: 0.0,
        }
    }

    pub fn done(&self) -> bool {
        self.remaining() == 0
    }

    pub fn remaining(&self) -> usize {
        self.grid.size() - self.collapsed.len()
    }

    pub fn tick(&mut self) -> bool {
        let mut worked = false;

        while let Some((x, y)) = self.stack.pop_front() {
            self.tick_cell(x, y);
            worked = true;
        }

        worked || self.maybe_collapse().is_none()
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
        if self.grid.get(x, y).unwrap().entropy() == 1 {
            return;
        }

        if self.data.get(x, y).unwrap().is_none() {
            let data = self.grid.get_neighbors(x, y).map(|_, v| match v {
                None => Set::default(),
                Some(neighbor) => Set::from_iter(neighbor.possible.iter().map(|x| x.get_id())),
            });

            self.data.set(x, y, Some(data)).unwrap();
        }

        let cell = self.grid.get_mut(x, y).unwrap();

        let neighbors = self.data.replace(x, y, None).unwrap().unwrap();

        self.data.set(x, y, None).unwrap();
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
        self.grid.get_mut(x, y).unwrap().collapse(&mut self.rng);
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
            let entropy = self.grid.get(x, y).map_or(1, |cell| cell.entropy());
            
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
        
        candidates
            .choose(&mut self.rng)
            .map(|&(x, y)| {
                self.collapse(x, y);
                (x, y)
            })
    }

    fn mark(&mut self, cx: usize, cy: usize) {
        let possible_states: Set<T::Identifier> = self
            .grid
            .get(cx, cy)
            .unwrap()
            .possible
            .iter()
            .map(|t| t.get_id())
            .collect();

        // Collect neighbor positions to avoid borrowing conflicts
        let neighbor_positions: Vec<_> = self.data
            .get_neighbor_positions(cx, cy)
            .into_iter()
            .filter_map(|(dir, pos)| pos.map(|p| (dir, p)))
            .collect();

        for (direction, (x, y)) in neighbor_positions {
            match self.data.get_mut(x, y).unwrap() {
                None => {
                    let mut neighbors: Neighbors<Set<T::Identifier>> = Neighbors::default();
                    neighbors[direction.invert()].clone_from(&possible_states);
                    self.data.set(x, y, Some(neighbors)).unwrap();
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
            self.rollback_penalty += 0.5;
        } else {
            self.last_rollback = collapsed_count;
            self.rollback_penalty = 0.5;
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
            for (x, y, cell) in &self.grid_base {
                self.grid.set(x, y, cell.clone()).unwrap();
                self.data.set(x, y, None).unwrap();
            }

            self.collapsed.clear();
            self.stack.clear();
            self.rollback_penalty = 0.5;
            self.last_rollback = 0;
        } else {
            // Todo replace the rollback_penalty with a usize instead of using floats
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let rollback_amount = self.rollback_penalty.ceil() as usize;
            self.rollback(rollback_amount);

            // tmp hack, shouldn't have to do this...
            self.stack.clear();
            for (x, y, _) in &self.grid {
                self.data.set(x, y, None).unwrap();
                self.stack.push_back((x, y));
            }
        }
    }

    fn rollback(&mut self, mut count: usize) {
        trace!("Rollback {count}");

        if count == 0 {
            return;
        }

        self.data.reset_to_default();

        // revert last step of collapse stack
        while let Some(((x, y), reason)) = self.collapsed.pop() {
            self.rollback_propegate(x, y, None);

            self.stack.push_front((x, y));

            if reason == CollapseReason::Explicit {
                count -= 1;

                if count == 0 {
                    break;
                }
            }
        }
    }

    fn rollback_propegate(&mut self, x: usize, y: usize, from: Option<Direction>) {
        // set state to base state
        let base = self.grid_base.get(x, y).unwrap().clone();
        self.grid.set(x, y, base).unwrap();
        self.stack.push_back((x, y));

        // for each neighbor (skipping "from" direction)
        //  - get entropy
        //  - set to base
        //  - tick
        //  - if entropy changed recurse

        for (direction, value) in self.grid.get_neighbor_positions(x, y) {
            if direction == from.unwrap_or(direction.invert()) {
                continue;
            }

            if let Some((nx, ny)) = value {
                let cell = self.grid.get(nx, ny).unwrap();
                let entropy = cell.entropy();

                if entropy == 1 || !cell.collapsing() {
                    continue;
                }

                let mut base = self.grid_base.get(nx, ny).unwrap().clone();

                let neighbors = self.grid.get_neighbors(nx, ny).map(|_, v| match v {
                    None => Set::default(),
                    Some(neighbor) => neighbor.possible.iter().map(|x| x.get_id()).collect::<Set<_>>(),
                });

                base.tick(&neighbors);

                let new_entropy = base.entropy();

                if entropy != new_entropy {
                    // todo: Remove recursion
                    self.rollback_propegate(nx, ny, Some(direction.invert()));
                }
            }
        }
    }

    fn collapsable_areas(&self) -> Vec<Vec<Position>> {
        let mut board = Grid::<bool>::new(self.grid.width(), self.grid.height(), &mut |x, y| {
            let item = self.grid.get(x, y).unwrap();

            item.entropy() == 1
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

                    board.set(x, y, true).unwrap();

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

        output.sort_by_key(std::vec::Vec::len);

        output
    }
}
