use crate::grid::Neighbors;
use crate::wave::Set;
use rand::RngCore;
use rand::prelude::IndexedRandom;
use std::{hash::Hash, sync::Arc};

#[cfg(feature = "threaded")]
use {
    log::trace, rayon::prelude::IntoParallelRefIterator, rayon::prelude::ParallelIterator,
    std::sync::LazyLock,
};

#[cfg(feature = "threaded")]
/// Entropy below which spreading the possibility filter over threads costs more than
/// it saves. The per-thread figure is a guess, not a measurement.
static PAR_MIN_LEN: LazyLock<usize> = LazyLock::new(|| {
    let min_len = 20 * rayon::current_num_threads();

    trace!("Min workload size before threading: {min_len}");

    min_len
});

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
    #[must_use]
    pub fn new(possible: Vec<Arc<T>>) -> Self {
        let base_entropy = possible.len();

        Self {
            possible,
            base_entropy,
            entropy: base_entropy,
        }
    }

    #[must_use]
    pub fn base_entropy(&self) -> usize {
        self.base_entropy
    }

    #[must_use]
    pub fn collapsing(&self) -> bool {
        self.base_entropy != self.entropy()
    }

    #[inline]
    #[must_use]
    pub fn entropy(&self) -> usize {
        self.entropy
    }

    #[inline]
    fn update_entropy(&mut self) {
        self.entropy = self.possible.len();
    }

    #[must_use]
    pub fn collapsed(&self) -> Option<&T> {
        match self.entropy {
            1 => Some(self.possible.first()?.as_ref()),
            _ => None,
        }
    }

    /// Narrows this cell to one named tile. `false` when the cell has already
    /// collapsed or no longer holds that tile as a possibility.
    pub fn coerce(&mut self, tile_id: T::Identifier) -> bool {
        if self.entropy <= 1 {
            return false;
        }

        let Some(index) = self
            .possible
            .iter()
            .position(|tile| tile.get_id() == tile_id)
        else {
            return false;
        };

        let chosen = self.possible.swap_remove(index);

        self.possible.clear();
        self.possible.push(chosen);
        self.update_entropy();

        true
    }

    /// Narrows this cell to one tile, drawn by weight. Does nothing when the cell
    /// has already collapsed, or when every remaining tile has weight zero.
    pub fn collapse(&mut self, rng: &mut dyn RngCore) {
        if self.entropy <= 1 {
            return;
        }

        // Sorted so a seed picks the same tile whatever order propagation left
        // the possibilities in.
        self.possible.sort_by_key(|tile| tile.get_id());

        let Ok(chosen) = self.possible.choose_weighted(rng, |tile| tile.get_weight()) else {
            return;
        };
        let chosen = Arc::clone(chosen);

        self.possible.clear();
        self.possible.push(chosen);
        self.update_entropy();
    }

    /// Drops every possibility the neighbours forbid. Runs on a collapsed cell as
    /// well, so a last possibility that a neighbour has since ruled out is caught
    /// rather than left standing.
    pub fn tick(&mut self, neighbors: &Neighbors<Set<T::Identifier>>) {
        if self.entropy > 0 {
            #[cfg(feature = "threaded")]
            if self.entropy > *PAR_MIN_LEN {
                self.possible = self
                    .possible
                    .par_iter()
                    .filter(|s| s.test(neighbors))
                    .cloned()
                    .collect();
            } else {
                self.possible.retain(|s| s.test(neighbors));
            }

            #[cfg(not(feature = "threaded"))]
            self.possible.retain(|s| s.test(neighbors));

            self.update_entropy();
        }
    }
}
