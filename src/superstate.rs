use crate::grid::Neighbors;
use crate::wave::Set;
use rand::seq::SliceRandom;
use rand::RngCore;
use range_set_blaze::Integer;
use std::hash::Hash;
use std::rc::Rc;

pub trait Collapsable: Clone {
    type Identifier: Integer + Default;
    fn test(&self, neighbors: &Neighbors<Set<Self::Identifier>>) -> bool;
    fn get_id(&self) -> Self::Identifier;
    fn get_weight(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct SuperState<T>
where
    T: Collapsable,
{
    pub possible: Vec<Rc<T>>,
    base_entropy: usize,
    entropy: usize,
}

impl<T> SuperState<T>
where
    T: Collapsable,
{
    pub fn new(possible: Vec<Rc<T>>) -> Self {
        let base_entropy = possible.len();

        Self {
            possible,
            base_entropy,
            entropy: base_entropy
        }
    }

    pub fn base_entropy(&self) -> usize {
        self.base_entropy
    }

    pub fn collapsing(&self) -> bool {
        self.base_entropy != self.entropy()
    }

    #[inline]
    pub fn entropy(&self) -> usize {
        self.entropy
    }

    #[inline]
    fn update_entropy(&mut self) {
        self.entropy = self.possible.len();
    }

    pub fn collapsed(&self) -> Option<Rc<T>> {
        match self.possible.len() {
            1 => Some(self.possible.get(0)?.clone()),
            _ => None,
        }
    }

    pub fn collapse(&mut self, rng: &mut dyn RngCore) {
        if self.possible.len() > 1 {
            self.possible = vec![self
                .possible
                .choose_weighted(rng, |v| v.get_weight())
                .unwrap()
                .clone()];

            self.update_entropy();
        }
    }

    pub fn tick(&mut self, neighbors: &Neighbors<Set<T::Identifier>>) {
        if neighbors.len() > 0 && self.entropy() > 1 {
            // self.possible.retain(|v| v.test(neighbors));

            // This is faster than retaining
            let mut i = 0;
            while i < self.possible.len() {
                if !self.possible[i].test(neighbors) {
                    self.possible.remove(i);
                } else {
                    i += 1;
                }
            }

            self.update_entropy();
        }
    }
}
