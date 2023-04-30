use crate::grid::Neighbors;
use crate::wave::Set;
use rand::seq::SliceRandom;
use rand::RngCore;
use std::{hash::Hash, sync::Arc};

#[cfg(feature = "threaded")]
use {
    crate::MIN_LEN,
    rayon::prelude::IntoParallelRefIterator,
    rayon::prelude::ParallelIterator,
    rayon::prelude::IndexedParallelIterator,
};

pub trait Collapsable: Clone + Sync + Send {
    type Identifier: Clone + Eq + Hash + Ord + Sync;
    fn test(&self, neighbors: &Neighbors<Set<Self::Identifier>>) -> bool;
    fn get_id(&self) -> Self::Identifier;
    fn get_weight(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct SuperState<T>
where
    T: Collapsable,
{
    pub possible: Vec<Arc<T>>,
    base_entropy: usize,
    entropy: usize,
}

impl<T> SuperState<T>
where
    T: Collapsable,
{
    pub fn new(possible: Vec<Arc<T>>) -> Self {
        let base_entropy = possible.len();

        Self {
            possible,
            base_entropy,
            entropy: base_entropy,
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

    pub fn collapsed(&self) -> Option<Arc<T>> {
        match self.possible.len() {
            1 => Some(self.possible.get(0)?.clone()),
            _ => None,
        }
    }

    pub fn collapse(&mut self, rng: &mut dyn RngCore) {
        if self.possible.len() > 1 {
            self.possible.sort_by_key(|a| a.get_id());

            self.possible = vec![self
                .possible
                .choose_weighted(rng, |v| v.get_weight())
                .unwrap()
                .clone()];

            self.update_entropy();
        }
    }

    pub fn tick(&mut self, neighbors: &Neighbors<Set<T::Identifier>>) {
        if self.entropy() > 1 {
            // self.possible.retain(|v| v.test(neighbors));
            #[cfg(feature = "threaded")]
            {
                self.possible = self
                    .possible
                    .par_iter()
                    .with_min_len(*MIN_LEN)
                    .filter(|s| s.test(neighbors))
                    .cloned()
                    .collect();
            }

            #[cfg(not(feature = "threaded"))]
            {
                self.possible = self
                    .possible
                    .iter()
                    .filter(|s| s.test(neighbors))
                    .cloned()
                    .collect();
            }

            self.update_entropy();
        }
    }
}
