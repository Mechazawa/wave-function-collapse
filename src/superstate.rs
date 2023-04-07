use crate::grid::Neighbors;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::rc::Rc;

pub trait Collapsable
{
    type Key;
    fn test(&self, neighbors: &Neighbors<Vec<Self::Key>>) -> bool;
    fn get_id(&self) -> Self::Key;
}

#[derive(Debug, Clone)]
pub struct SuperState<T>
where
    T: Collapsable,
{
    pub possible: Vec<Rc<T>>,
    pub last_tick: u32,
}

impl<T> SuperState<T>
where
    T: Collapsable,
{
    pub(crate) fn new(possible: Vec<Rc<T>>) -> Self {
        Self { possible, last_tick: 0 }
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

    pub fn collapse(&mut self, tick: u32, rng: &mut StdRng) {
        if self.entropy() > 1 {
            self.last_tick = tick;
            self.possible = vec![self.possible.choose(rng).unwrap().clone()];
        }
    }

    pub fn tick(&mut self, tick: u32, neighbors: &Neighbors<Vec<<T as Collapsable>::Key>>) {
        let entropy = self.entropy();

        if neighbors.len() > 0 && entropy > 1 {
            self.possible.retain(|v| v.test(&neighbors));

            if entropy != self.entropy() {
                self.last_tick = tick;
            }
        }
    }
}
