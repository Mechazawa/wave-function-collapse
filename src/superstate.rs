use crate::grid::Neighbors;
use rand::seq::SliceRandom;
use rand::RngCore;
use std::rc::Rc;

pub trait Collapsable {
    type Identifier;
    fn test(&self, neighbors: &Neighbors<Vec<Self::Identifier>>) -> bool;
    fn get_id(&self) -> Self::Identifier;
}

#[derive(Debug, Clone)]
pub struct SuperState<T>
where
    T: Collapsable,
{
    pub possible: Vec<Rc<T>>,
    pub last_tick: u32,
    base_entropy: usize,
}

impl<T> SuperState<T>
where
    T: Collapsable,
{
    pub fn new(possible: Vec<Rc<T>>) -> Self {
        let base_entropy = possible.len();

        Self {
            possible,
            last_tick: 0,
            base_entropy,
        }
    }

    pub fn base_entropy(&self) -> usize {
        self.base_entropy
    }

    pub fn collapsing(&self) -> bool {
        self.base_entropy != self.entropy()
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

    pub fn collapse(&mut self, tick: u32, rng: &mut dyn RngCore) {
        if self.entropy() > 1 {
            self.last_tick = tick;
            self.possible = vec![self.possible.choose(rng).unwrap().clone()];
        }
    }

    pub fn tick(&mut self, tick: u32, neighbors: &Neighbors<Vec<T::Identifier>>) {
        let entropy = self.entropy();

        if neighbors.len() > 0 && entropy > 1 {
            self.possible.retain(|v| v.test(neighbors));

            if entropy != self.entropy() {
                self.last_tick = tick;
            }
        }
    }
}
