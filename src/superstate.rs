use crate::{grid::Neighbors, wave::NoOpHasher};
use bit_set::BitSet;
use bit_vec::BitBlock;
use num_traits::{NumCast, Num};
use rand::seq::SliceRandom;
use rand::RngCore;
use std::{sync::Arc, collections::HashSet, fmt::Debug};

pub trait Collapsable: Clone + Default {
    type Identifier: BitBlock;
    fn test(&self, neighbors: &Neighbors<StateSet<Self::Identifier>>) -> bool;
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

    pub fn tick(&mut self, neighbors: &Neighbors<StateSet<T::Identifier>>) {
        if self.entropy() > 1 {
            self.possible = self
                .possible
                .iter()
                .filter(|s| s.test(neighbors))
                .cloned()
                .collect();
            
            self.update_entropy();
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct StateSet<T: BitBlock> {
    states: HashSet<T, NoOpHasher>,
    mask: BitSet<T>,
}

impl<T: BitBlock> StateSet<T> {
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn is_disjoint(&self, set: &StateSet<T>) -> bool {
        self.mask.is_disjoint(set.get_mask_ref())
    }

    pub fn get_mask_ref(&self) -> &BitSet<T> {
        &self.mask
    }
}

impl<T: BitBlock + NumCast> StateSet<T> {
    pub fn insert(&mut self, value: T) {
        self.mask.insert(value.to_usize().unwrap());
        self.states.insert(value);
    }
}

impl<T: BitBlock> IntoIterator for StateSet<T> {
    type Item = T;

    type IntoIter = <HashSet<T, NoOpHasher> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.states.into_iter()
    }
}

impl<T: BitBlock + NumCast> FromIterator<T> for StateSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let states = HashSet::from_iter(iter);
        let mut mask = BitSet::<T>::default();

        states.iter().for_each(|&x| {mask.insert(x.to_usize().unwrap());});

        Self {
            mask,
            states,
        }
    }
}