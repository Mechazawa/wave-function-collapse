use crate::grid::Neighbors;
use crate::superstate::Collapsable;
use crate::wave::Set;
use enum_map::EnumMap;

#[derive(Debug, Clone)]
pub struct Tile<T> {
    pub value: T,
    /// Ids of the tiles allowed on each side of this one.
    pub neighbors: Neighbors<Set<u64>>,
    pub weight: usize,

    id: u64,
}

impl<T> Tile<T> {
    /// A tile with no allowed neighbours in any direction, which fails every
    /// constraint until `neighbors` is populated.
    #[must_use]
    pub fn new(id: u64, value: T) -> Self {
        Self {
            id,
            value,
            neighbors: EnumMap::default(),
            weight: 1,
        }
    }

    /// Whether this tile can be placed at all. A tile with no allowed neighbour on
    /// some side fails every constraint on that side, so it can never sit beside
    /// anything and only costs the solver contradictions.
    #[must_use]
    pub fn is_placeable(&self) -> bool {
        self.neighbors.values().all(|allowed| !allowed.is_empty())
    }

    #[must_use]
    pub fn with_neighbors(
        id: u64,
        value: T,
        weight: usize,
        neighbors: Neighbors<Set<u64>>,
    ) -> Self {
        Self {
            id,
            value,
            neighbors,
            weight,
        }
    }
}

impl<T: Clone + Sync + Send> Collapsable for Tile<T> {
    type Identifier = u64;

    fn test(&self, neighbors: &Neighbors<Set<Self::Identifier>>) -> bool {
        for (direction, tiles) in neighbors {
            if tiles.is_empty() {
                continue;
            }

            let possible = &self.neighbors[direction];

            if possible.is_disjoint(tiles) {
                return false;
            }
        }

        true
    }

    fn get_id(&self) -> Self::Identifier {
        self.id
    }

    fn get_weight(&self) -> usize {
        self.weight
    }
}

// Implement Collapsable for Arc<Tile<T>> to enable shared ownership
impl<T: Clone + Sync + Send> Collapsable for std::sync::Arc<Tile<T>> {
    type Identifier = u64;

    fn test(&self, neighbors: &Neighbors<Set<Self::Identifier>>) -> bool {
        self.as_ref().test(neighbors)
    }

    fn get_id(&self) -> Self::Identifier {
        self.as_ref().get_id()
    }

    fn get_weight(&self) -> usize {
        self.as_ref().get_weight()
    }
}
