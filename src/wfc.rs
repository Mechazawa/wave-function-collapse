use crate::grid::{Direction, Neighbors};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::rc::Rc;

pub trait Collapsable {
    fn test(&self, neighbors: &Neighbors<Vec<u64>>) -> bool;
    fn get_id(&self) -> u64;
}

#[derive(Debug, Clone)]
pub struct SuperState<T>
where
    T: Collapsable,
{
    pub possible: Vec<Rc<T>>,
}

impl<T> SuperState<T>
where
    T: Collapsable,
{
    pub(crate) fn new(possible: Vec<Rc<T>>) -> Self {
        Self { possible }
    }

    pub fn entropy(&self) -> usize {
        self.possible.len()
    }

    pub fn collapsed(&self) -> Option<Rc<T>> {
        match self.possible.len() {
            1 => Some(self.possible.get(0)?.clone()),
            _ => None,
        }
    }

    pub fn collapse(&mut self, rng: &mut StdRng) {
        if self.entropy() > 1 {
            self.possible = vec![self.possible.choose(rng).unwrap().clone()];
        }
    }

    pub fn tick(&mut self, neighbors: &Neighbors<Vec<u64>>) {
        if neighbors.is_empty() == false && self.entropy() > 1 {
            self.possible.retain(|v| v.test(&neighbors));
        }
    }
}
