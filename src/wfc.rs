use log::{debug, trace, warn};
use rand::prelude::SliceRandom;
use rand::{RngCore, SeedableRng};
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

    updated: Vec<Position>,
}

impl<T> WaveFuncCollapse<T>
where
    T: Collapsable + Clone,
{
    pub fn new(grid: Grid<SuperState<T>>, seed: u64) -> Self {
        let mut rng = XorShiftRng::seed_from_u64(seed);

        let mut stack: Vec<Position> = grid.iter().map(|(x, y, _)| (x, y)).collect();
        let collapse_stack = Vec::with_capacity(stack.len());

        stack.shuffle(&mut rng);

        Self {
            base: grid.clone(),
            updated: Vec::with_capacity(grid.size()),
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

    fn tick_cell(&mut self, x: usize, y: usize, force: bool) {
        let mut neighbors: Neighbors<Vec<T::Identifier>> = Default::default();
        let mut do_test: bool = force;

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
            let cell = self.grid
                .get_mut(x, y)
                .unwrap();

            if self.ticks != cell.last_tick {
                self.updated.push((x, y));
            }

            cell.tick(self.ticks, &neighbors);
        }
    }

    pub fn tick(&mut self) {
        self.updated.clear();
        self.ticks += 1;

        // redo how this works, we need to wave through the grid not iterate
        // if entropy changes
        //   - get list of neighbors
        //   - push to stack
        //   - tick only updated side with new value
        // repeat untill the stack is empty
        // We can prepare the tick neighbor list argument by keeping those in a grid.
        // This way we can also prevent the stack getting filled with duplicate cells to tick
        // Make sure we don't use recursions
        // We can thread this if we want in the future
        for index in 0..self.stack.len() {
            let (x, y) = self.stack[index];

            self.tick_cell(x, y, false);
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
        self.fixup();

        let index = self.pick_next();

        if index.is_none() {
            assert_eq!(self.stack.len(), 0);
            trace!("Nothing to collapse");
            return;
        }

        // todo pick index on edge (so with neighborsl)
        // Either rollback if lowest entropy is zero or collapse it.
        if let Some(&(x, y)) = self.stack.get(index.unwrap()) {
            // todo: we need to remove the used stack item!
            self.tick_cell(x, y, true);

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

    // Assumes a sorted stack
    fn stack_floor_size(&self) -> usize {
        let stack_size = self.stack.len();

        if stack_size == 0 {
            return 0;
        }

        let (x, y) = self.stack[0];
        let value = self.grid.get(x, y).unwrap().entropy();

        for index in 0..(stack_size - 1) {
            let (x, y) = self.stack[index];

            if self.grid.get(x, y).unwrap().entropy() != value {
                return index;
            }
        }

        stack_size
    }

    fn pick_next(&mut self) -> Option<usize> {
        let floor = self.stack_floor_size();

        if floor == 0 {
           return None;
        }

        let mut options: Vec<usize> = (0..floor).collect();

        options.shuffle(&mut self.rng);

        let fallback = options[0];

        for index in options {
            let (x, y) = self.stack[index];

            for (_, maybe) in self.grid.get_neighbors(x, y) {
                if let Some(neighbor) = maybe {
                    if neighbor.entropy() == 1 {
                        return Some(index);
                    }
                }
            }
        }

        Some(fallback)
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

        self.fixup();

        assert!(self.stack.len() <= self.grid.size());

        // reset the entropy for other tiles
        for &(x, y) in &self.stack {
            self.grid.set(x, y, self.new_base(x, y)).unwrap();
        }
    }

    pub fn rollback(&mut self) {
        self.rollbacks += 1;

        let mut steps = 1 + (self.rollbacks / 5);

        if steps > 1 {
            trace!(
                "Rollback score: {}, steps: {}, stack sizes: ({}, {})",
                self.rollbacks,
                steps,
                self.stack.len(),
                self.collapse_stack.len()
            );
        }

        if steps > 10 {
            warn!("Lockup detected, resetting...");

            self.reset();

            return;
        }

        loop {
            let (x, y, implicit) = match self.collapse_stack.pop() {
                None => break,
                Some(v) => v,
            };

            self.stack.push((x, y));

            if !implicit {
                steps -= 1;
            }

            if steps == 0 {
                break;
            }
        }

        // reset the entropy all tiles
        for &(x, y) in &self.stack {
            if self.grid.get(x, y).unwrap().collapsing() {
                self.grid.set(x, y, self.new_base(x, y)).unwrap();
            }
        }

        // sort the stack again
        self.fixup();

        for (x, y) in self.stack.clone() {
            self.tick_cell(x, y, true);
        }
    }

    fn fixup(&mut self) {
        let get_index = |x, y| x + (y * self.grid.width());

        self.stack
            .sort_by(|a, b| get_index(a.0, a.1).cmp(&get_index(b.0, b.1)));

        self.stack.dedup();

        self.stack.sort_unstable_by(|a, b| {
            self.grid
                .get(a.0, a.1)
                .unwrap()
                .entropy()
                .cmp(&self.grid.get(b.0, b.1).unwrap().entropy())
        });
    }

    pub fn get_updated(&self) -> &Vec<Position> {
        &self.updated
    }
}
