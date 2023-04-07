use log::debug;
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::grid::{Grid, Neighbors};
use crate::superstate::{Collapsable, SuperState};

type CollapsedItem = (usize, usize, bool);
type Position = (usize, usize);

pub struct WaveFuncCollapse<T>
where
    T: Collapsable + Clone,
{
    pub grid: Grid<SuperState<T>>,
    rng: StdRng,
    stack: Vec<Position>,
    collapse_stack: Vec<CollapsedItem>,
    base: Grid<SuperState<T>>,
}

impl<T> WaveFuncCollapse<T>
where
    T: Collapsable + Clone,
{
    pub fn new(mut grid: Grid<SuperState<T>>, seed: u64) -> Self {
        let mut stack: Vec<Position> = grid.iter().map(|(x, y, _)| (x, y)).collect();
        let mut rng = StdRng::seed_from_u64(seed);

        stack.shuffle(&mut rng);

        let (x, y) = stack.pop().unwrap();

        grid.get_mut(x, y).unwrap().collapse(&mut rng);

        debug!("Starting at ({}, {})", x, y);

        Self {
            base: grid.clone(),
            grid,
            stack,
            rng,
            collapse_stack: Vec::new(),
        }
    }

    pub fn done(&self) -> bool {
        self.stack.len() == 0
    }

    pub fn remaining(&self) -> usize {
        self.stack.len()
    }

    pub fn tick(&mut self) {
        // todo: optimise to only test positions near collapsed
        // test all positions
        for &(x, y) in &self.stack {
            let mut neighbors: Neighbors<Vec<u64>> = Default::default();

            for (direction, maybe_cell) in self.grid.get_neighbors(x, y) {
                if let Some(cell) = maybe_cell {
                    let base_entropy = self.base.get(x, y).unwrap().entropy();

                    if cell.entropy() < base_entropy {
                        neighbors[direction] = cell.possible.iter().map(|t| t.get_id()).collect();
                    }
                }
            }

            self.grid.get_mut(x, y).unwrap().tick(&neighbors);
        }

        let mut stack_next = Vec::new();

        stack_next.reserve_exact(self.stack.len());

        for (x, y) in &self.stack {
            match self.grid.get(*x, *y).unwrap().entropy() {
                1 => self.collapse_stack.push((*x, *y, true)),
                _ => stack_next.push((*x, *y)),
            }
        }

        // sort the stack; entropy ascending
        self.sort();

        // Either rollback if lowest entropy is zero or collapse it.
        if let Some(&(x, y)) = self.stack.get(0) {
            if self.grid.get(x, y).unwrap().entropy() == 0 {
                self.rollback();
            } else {
                self.grid.get_mut(x, y).unwrap().collapse(&mut self.rng);
                self.collapse_stack.push((x, y, false));
            }
        }
    }

    pub fn rollback(&mut self) {
        loop {
            let (lx, ly, implicit) = match self.collapse_stack.pop() {
                None => break,
                Some(v) => v,
            };

            let base_state = self.base.get(lx, ly).unwrap().clone();

            self.grid.set(lx, ly, base_state).unwrap();

            self.stack.push((lx, ly));

            if implicit == false {
                break;
            }
        }

        // reset the entropy for other tiles
        for &(x, y) in &self.stack {
            let base_state = self.base.get(x, y).unwrap().clone();

            self.grid.set(x, y, base_state).unwrap();
        }

        // sort the stack again
        self.sort();
    }

    fn sort(&mut self) {
        self.stack.sort_by(|a, b| {
            self.grid
                .get(a.0, a.1)
                .unwrap()
                .entropy()
                .cmp(&self.grid.get(b.0, b.1).unwrap().entropy())
        });
    }
}
