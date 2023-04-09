use std::collections::VecDeque;

use log::{trace, warn};
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{thread_rng, RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;

use crate::grid::{Grid, Neighbors, Position};
use crate::superstate::{Collapsable, SuperState};

type CellNeighbors<T> = Option<Neighbors<Vec<<T as Collapsable>::Identifier>>>;

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
}

impl<T> Wave<T>
where
    T: Collapsable,
{
    pub fn new(grid: Grid<SuperState<T>>) -> Self {
        Self {
            stack: VecDeque::with_capacity(grid.size()),
            collapsed: Vec::with_capacity(grid.size()),
            data: Grid::new(grid.width(), grid.height(), &mut |_, _| Default::default()),
            grid_base: grid.clone(),
            grid,
            rng: Box::new(XorShiftRng::from_rng(thread_rng()).unwrap()),
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

    pub fn tick(&mut self) -> bool {
        if self.stack.is_empty() && self.maybe_collapse().is_none() {
            return false;
        }

        while let Some((x, y)) = self.stack.pop_front() {
            self.tick_cell(x, y);
        }

        true
    }

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
                None => Vec::new(),
                Some(neighbor) => neighbor.possible.iter().map(|x| x.get_id()).collect(),
            });

            self.data.set(x, y, Some(data)).unwrap();
        }

        let cell = self.grid.get_mut(x, y).unwrap();

        let neighbors = self.data.get(x, y).unwrap().clone().unwrap();

        self.data.set(x, y, None).unwrap();
        let old_entropy = cell.entropy();

        cell.tick(0, &neighbors);

        if cell.entropy() <= 1 {
            self.collapsed.push(((x, y), CollapseReason::Implicit));
        }

        if cell.entropy() == 0 {
            self.rollback(1);
        } else if old_entropy != cell.entropy() {
            self.mark(x, y);
        }
    }

    fn collapse(&mut self, x: usize, y: usize) {
        self.grid.get_mut(x, y).unwrap().collapse(0, &mut self.rng);
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
        let choices: Vec<Position> = positions
            .iter()
            .filter(|(_, _, entropy)| entropy == lowest)
            .map(|&(x, y, _)| (x, y))
            .filter(|(x, y)| {
                self.grid
                    .get_neighbors(*x, *y)
                    .values()
                    .any(|&cell| cell.is_some() && cell.unwrap().entropy() == 1)
            })
            .collect();

        if choices.is_empty() {
            None
        } else {
            let &(x, y) = choices.choose(&mut self.rng).unwrap();

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

                    neighbors[direction.invert()] = possible_states.clone();

                    self.data.set(x, y, Some(neighbors)).unwrap();

                    self.stack.push_back((x, y));
                }
                Some(neighbors) => {
                    neighbors[direction.invert()] = possible_states.clone();
                }
            }
        }
    }

    fn rollback(&mut self, mut count: usize) {
        assert!(count > 0, "Rollback count must be at least 1");
        warn!("Rollback {count}");

        // empty stack
        self.stack.clear();
        self.data = Grid::new(self.grid.width(), self.grid.height(), &mut |_, _| {
            Default::default()
        });

        // revert last step of collapse stack
        while let Some(((x, y), reason)) = self.collapsed.pop() {
            let value = self.grid_base.get(x, y).unwrap().clone();
            self.grid.set(x, y, value).unwrap();

            if reason == CollapseReason::Explicit {
                count -= 1;

                if count == 0 {
                    break;
                }
            }
        }

        let positions: Vec<Position> = self
            .grid
            .iter()
            .filter(|&(_, _, cell)| cell.entropy() > 1 && cell.collapsing())
            .map(|(x, y, _)| (x, y))
            .collect();

        for (x, y) in positions {
            let value = self.grid_base.get(x, y).unwrap().clone();

            self.grid.set(x, y, value).unwrap();

            let data = self.grid.get_neighbors(x, y).map(|_, v| match v {
                None => Vec::new(),
                Some(neighbor) => neighbor.possible.iter().map(|x| x.get_id()).collect(),
            });

            self.mark(x, y);
            self.data.set(x, y, Some(data)).unwrap();
            self.stack.push_back((x, y));
        }
    }
}
