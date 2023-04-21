use std::collections::{HashSet, VecDeque};
use std::hash::{Hasher, BuildHasher};

use log::trace;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;

use crate::grid::{Direction, Grid, Neighbors, Position};
use crate::superstate::{Collapsable, SuperState};

/// https://github.com/chris-morgan/anymap/blob/2e9a5704/src/lib.rs#L599
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpHasher(u64);

impl Hasher for NoOpHasher {
    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("This NoOpHasher can only handle u64s")
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl BuildHasher for NoOpHasher {
    type Hasher = Self;

    fn build_hasher(&self) -> Self::Hasher {
        self.clone()
    }
}

type CellNeighbors<T> = Option<Neighbors<Set<<T as Collapsable>::Identifier>>>;
pub type Set<T> = HashSet<T, NoOpHasher>;


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
    data: Grid<CellNeighbors<T>>,
    collapsed: Vec<(Position, CollapseReason)>,
    rng: Box<dyn RngCore>,
    last_rollback: usize,
    rollback_penalty: usize,
}

impl<T> Wave<T>
where
    T: Collapsable,
{
    pub fn new(grid: Grid<SuperState<T>>, seed: u64) -> Self {
        Self {
            stack: VecDeque::with_capacity(grid.size()),
            collapsed: Vec::with_capacity(grid.size()),
            data: Grid::new(grid.width(), grid.height(), &mut |_, _| Default::default()),
            grid_base: grid.clone(),
            grid,
            rng: Box::new(XorShiftRng::seed_from_u64(seed)),
            last_rollback: 0,
            rollback_penalty: 0,
        }
    }

    pub fn done(&self) -> bool {
        self.remaining() == 0
    }

    pub fn remaining(&self) -> usize {
        self.grid.size() - self.collapsed.len()
    }

    pub fn maybe_collapse(&mut self) -> Option<Position> {
        let pos = self.collapse_edge();

        if pos.is_none() {
            trace!("Failed to find edge to collapse");

            self.collapse_any()
        } else {
            pos
        }
    }

    #[allow(dead_code)]
    pub fn tick(&mut self) -> bool {
        if self.stack.is_empty() && self.maybe_collapse().is_none() {
            return false;
        }

        while let Some((x, y)) = self.stack.pop_front() {
            self.tick_cell(x, y);
        }

        true
    }

    #[allow(dead_code)]
    pub fn tick_once(&mut self) -> Option<Position> {
        if let Some((x, y)) = self.stack.pop_front() {
            self.tick_cell(x, y);

            Some((x, y))
        } else if let Some(value) = self.maybe_collapse() {
            return Some(value);
        } else {
            None
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

        let neighbors = self.data.get(x, y).unwrap().clone().unwrap();

        self.data.set(x, y, None).unwrap();
        let old_entropy = cell.entropy();

        cell.tick(&neighbors);

        if cell.entropy() <= 1 {
            self.collapsed.push(((x, y), CollapseReason::Implicit));
        }

        if cell.entropy() == 0 {
            self.smart_rollback();
        } else if old_entropy != cell.entropy() {
            self.mark(x, y);
        }
    }

    fn collapse(&mut self, x: usize, y: usize) {
        self.grid.get_mut(x, y).unwrap().collapse(&mut self.rng);
        self.collapsed.push(((x, y), CollapseReason::Explicit));
        self.mark(x, y);
    }

    pub fn collapse_any(&mut self) -> Option<Position> {
        let maybe = self
            .grid
            .iter()
            .filter(|(_, _, cell)| cell.entropy() > 1)
            .map(|(x, y, _)| (x, y))
            .choose_stable(&mut self.rng);

        match maybe {
            Some((x, y)) => {
                self.collapse(x, y);
                Some((x, y))
            }
            None => None,
        }
    }

    pub fn collapse_edge(&mut self) -> Option<Position> {
        // get lowest entropy
        let positions: Vec<_> = self
            .grid
            .iter()
            .map(|(x, y, cell)| (x, y, cell.entropy()))
            .collect();

        let lowest = positions
            .iter()
            .map(|(_, _, e)| e)
            .filter(|&&e| e > 1)
            .min()
            .unwrap_or(&0);

        if *lowest == 0 {
            return None;
        }

        // filter for edge
        let mut choices: Vec<_> = positions
            .iter()
            .filter(|(_, _, entropy)| entropy == lowest)
            .map(|&(x, y, _)| {
                (
                    x,
                    y,
                    self.grid
                        .get_neighbors(x, y)
                        .iter()
                        .filter(|(_, s)| s.is_some() && s.unwrap().entropy() == 1)
                        .count(),
                )
            })
            .filter(|(_, _, n)| *n > 0)
            .collect();

        if choices.is_empty() {
            None
        } else {
            choices.sort_by(|(_, _, a), (_, _, b)| b.cmp(a));

            let &(x, y, _) = choices
                .choose_weighted(&mut self.rng, |(_, _, n)| *n)
                .unwrap();

            self.collapse(x, y);

            Some((x, y))
        }
    }

    fn mark(&mut self, cx: usize, cy: usize) {
        let possible_states: Vec<T::Identifier> = self
            .grid
            .get(cx, cy)
            .unwrap()
            .possible
            .iter()
            .map(|t| t.get_id())
            .collect();

        for (direction, pos) in self.data.get_neighbor_positions(cx, cy) {
            if pos.is_none() {
                continue;
            }

            let (x, y) = pos.unwrap();
            match self.data.get_mut(x, y).unwrap() {
                None => {
                    let mut neighbors = Neighbors::default();

                    neighbors[direction.invert()] = possible_states.clone().into_iter().collect();

                    self.data.set(x, y, Some(neighbors)).unwrap();

                    self.stack.push_back((x, y));
                }
                Some(neighbors) => {
                    neighbors[direction.invert()] = possible_states.clone().into_iter().collect();
                }
            }
        }
    }

    fn smart_rollback(&mut self) {
        let collapsed_count = self.grid.size() - self.remaining();

        trace!("Collapsed: {}", collapsed_count);

        if collapsed_count <= self.last_rollback {
            self.rollback_penalty += 1;
        } else {
            self.last_rollback = collapsed_count;
            self.rollback_penalty = 1;
        }

        self.rollback(self.rollback_penalty);

        if self.collapsed.len() == 0 {
            self.rollback_penalty = 1;
        }
    }

    fn rollback(&mut self, mut count: usize) {
        trace!("Rollback {count}");

        if count == 0 {
            return;
        }

        // empty stack
        self.stack.clear();
        self.data = Grid::new(self.grid.width(), self.grid.height(), &mut |_, _| {
            Default::default()
        });

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
                    Some(neighbor) => Set::from_iter(neighbor.possible.iter().map(|x| x.get_id())),
                });

                base.tick(&neighbors);

                let new_entropy = base.entropy();

                if entropy != new_entropy {
                    self.rollback_propegate(nx, ny, Some(direction.invert()));
                }
            }
        }
    }
}
