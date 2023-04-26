use log::{debug, trace, warn};
use rand::{RngCore, SeedableRng};
use rand::prelude::SliceRandom;
use rand_xorshift::XorShiftRng;

use crate::grid::{Grid, Neighbors};
use crate::superstate::{Collapsable, SuperState};

type CollapsedItem = (usize, usize, bool);
type Position = (usize, usize);

pub struct WaveFuncCollapse<T>
where
    T: Collapsable + Clone,
{
    pub grid: Grid<SuperState<T>>,
    rng: Box<dyn RngCore>,
    stack: Vec<Position>,
    collapse_stack: Vec<CollapsedItem>,
    base: Grid<SuperState<T>>,
    ticks: u32,
    rollbacks: u16,
}

impl<T> WaveFuncCollapse<T>
where
    T: Collapsable + Clone,
{
    pub fn new(mut grid: Grid<SuperState<T>>, seed: u64) -> Self {
        let mut rng = XorShiftRng::seed_from_u64(seed);

        let mut stack: Vec<Position> = grid.iter().map(|(x, y, _)| (x, y)).collect();
        let collapse_stack = Vec::with_capacity(stack.len());
        
        stack.shuffle(&mut rng);
        
        let (x, y) = stack.pop().unwrap();
        grid.get_mut(x, y).unwrap().collapse(0, &mut rng);

        debug!("Starting at ({}, {})", x, y);

        Self {
            base: grid.clone(),
            grid,
            stack,
            rng: Box::new(rng),
            collapse_stack,
            ticks: 0,
            rollbacks: 0,
        }
    }

    pub fn done(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.stack.len()
    }

    pub fn tick(&mut self) {
        self.ticks += 1;

        // todo: optimise to only consider near previously changed, fixes large grid performance
        // test all positions
        for &(x, y) in &self.stack {
            let mut neighbors: Neighbors<Vec<T::Identifier>> = Default::default();
            let mut do_test: bool = false;

            for (direction, maybe_cell) in self.grid.get_neighbors(x, y) {
                if let Some(cell) = maybe_cell {
                    let base_entropy = self.base.get(x, y).unwrap().entropy();

                    if cell.entropy() < base_entropy {
                        do_test = do_test || cell.last_tick >= self.ticks - 1;
                        neighbors[direction] = cell.possible.iter().map(|t| t.get_id()).collect();
                    }
                }
            }

            if do_test {
                self.grid
                    .get_mut(x, y)
                    .unwrap()
                    .tick(self.ticks, &neighbors);
            }
        }

        self.stack
            .retain(|(x, y)| match self.grid.get(*x, *y).unwrap().entropy() {
                1 => {
                    self.collapse_stack.push((*x, *y, true));
                    false
                }
                _ => true,
            });

        // sort the stack; entropy ascending
        self.sort();

        // Either rollback if lowest entropy is zero or collapse it.
        if let Some(&(x, y)) = self.stack.first() {
            if self.grid.get(x, y).unwrap().entropy() == 0 {
                self.collapse_stack.push((x, y, true));
                self.rollback();
            } else {
                self.grid
                    .get_mut(x, y)
                    .unwrap()
                    .collapse(self.ticks, &mut self.rng);
                self.collapse_stack.push((x, y, false));

                if self.rollbacks > 0 {
                    self.rollbacks -= 1;
                }
            }
        }
    }

    pub fn new_base(&self, x: usize, y: usize) -> SuperState<T> {
        let mut base_state = self.base.get(x, y).unwrap().clone();

        base_state.last_tick = self.ticks;

        base_state
    }

    pub fn reset(&mut self) {
        for &(x, y, _) in &self.collapse_stack {
            self.grid.set(x, y, self.new_base(x, y)).unwrap();

            self.stack.push((x, y));
        }

        self.collapse_stack = Vec::with_capacity(self.stack.len());
        self.rollbacks = 0;

        assert!(self.stack.len() <= self.grid.size());
    }

    pub fn rollback(&mut self) {
        self.rollbacks += 5;

        let mut steps = 1 + (self.rollbacks / 100);

        if steps > 1 {
            trace!("Rollback score: {}, steps: {steps}", self.rollbacks);
        }

        if steps > 5 {
            warn!("Lockup detected, resetting...");
            
            self.reset();
        }

        loop {
            let (x, y, implicit) = match self.collapse_stack.pop() {
                None => break,
                Some(v) => v,
            };

            self.grid.set(x, y, self.new_base(x, y)).unwrap();

            self.stack.push((x, y));

            if implicit {
                steps -= 1;
            }

            if steps == 0 {
                break;
            }
        }

        // reset the entropy for other tiles
        for &(x, y) in &self.stack {
            self.grid.set(x, y, self.new_base(x, y)).unwrap();
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
