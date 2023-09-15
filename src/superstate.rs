use crate::grid::Neighbors;
use crate::wave::Set;
use rand::seq::SliceRandom;
use rand::RngCore;
use std::{hash::Hash, sync::Arc};

#[cfg(feature = "threaded")]
use {
    rayon::prelude::IntoParallelRefIterator,
    rayon::prelude::ParallelIterator,
    rayon::prelude::IndexedParallelIterator,
    log::trace,
    lazy_static::lazy_static,
};

#[cfg(feature = "threaded")]
lazy_static! {
    static ref PAR_MIN_LEN: usize = {
        let workload_size: f32 = 20.0; /// todo tune
        let num_threads = rayon::current_num_threads();
        let min_len = (workload_size * num_threads as f32).ceil() as usize;

        trace!("Min workload size before threading: {min_len}");

        min_len        
    };
}

pub trait Collapsable: Clone + Sync + Send {
    type Identifier: Clone + Eq + Hash + Ord + Sync + Send;
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

    pub fn collapsed(&self) -> Option<Arc<&T>> {
        match self.possible.len() {
            1 => Some(Arc::new(self.possible.get(0)?.as_ref())),
            _ => None,
        }
    }

    pub fn collapse(&mut self, rng: &mut dyn RngCore) {
        if self.possible.len() > 1 {
            self.possible.sort_by_key(|a| a.get_id());

            let chosen_id = self
                .possible
                .choose_weighted(rng, |v| v.get_weight())
                .unwrap()
                .get_id();

            let chosen_index = self.possible.iter().position(|v| v.get_id() == chosen_id);

            if let Some(pos) = chosen_index {
                self.possible = vec![self.possible.swap_remove(pos)];
            }

            self.update_entropy();
        }
    }

    pub fn tick(&mut self, neighbors: &Neighbors<Set<T::Identifier>>) {
        if self.entropy() > 1 {
            #[cfg(feature = "threaded")]
            {
                let ids: Vec<T::Identifier> = self
                    .possible
                    .par_iter()
                    .with_min_len(*PAR_MIN_LEN)
                    .filter(|s| s.test(neighbors))
                    .map(|s| s.get_id())
                    .collect();

                self.possible.retain(|s| ids.contains(&s.get_id()))
            }

            #[cfg(not(feature = "threaded"))]
            {
                self.possible.retain(|s| s.test(neighbors));
            }

            self.update_entropy();
        }
    }
}
