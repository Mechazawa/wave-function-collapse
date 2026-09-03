use wave_function_collapse::{Collapsable, Direction, Neighbors, Set, Tile};

fn allowing(open_sides: &[Direction]) -> Neighbors<Set<u64>> {
    Neighbors::from_fn(|direction| {
        if open_sides.contains(&direction) {
            Set::from_iter([7])
        } else {
            Set::default()
        }
    })
}

#[test]
fn a_tile_with_an_empty_side_fails_every_constraint_on_that_side() {
    const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Right,
        Direction::Down,
        Direction::Left,
    ];

    let closed_up = Tile::with_neighbors(1, (), 1, allowing(&[Direction::Right]));

    assert!(
        !closed_up.test(&allowing(&[Direction::Up])),
        "nothing is allowed above this tile, so a neighbour above must be rejected"
    );
    assert!(
        closed_up.test(&allowing(&[Direction::Right])),
        "a neighbour on the one open side is still accepted"
    );
    assert!(!closed_up.is_placeable());

    assert!(Tile::with_neighbors(2, (), 1, allowing(&ALL)).is_placeable());
}

#[test]
fn an_unconstrained_side_accepts_anything() {
    let closed_up = Tile::with_neighbors(1, (), 1, allowing(&[Direction::Right]));

    assert!(
        closed_up.test(&Neighbors::default()),
        "a neighbour that has not narrowed down yet constrains nothing"
    );
}
