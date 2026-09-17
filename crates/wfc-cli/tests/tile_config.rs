use wave_function_collapse::{Collapsable, Direction, Set};
use wfc_cli::tiles::{self, TileConfig};

use image::{Rgba, RgbaImage};
use std::path::Path;

/// Edge labels chosen so exactly two pairs match: B may sit above A, and C below
/// it. Every other label is unique and no label is its own reverse, so nothing
/// else can pair up by accident.
const TILES: [[&str; 4]; 3] = [
    ["ab", "r1", "cd", "l1"], // A
    ["u2", "r2", "ba", "l2"], // B, whose down edge reverses A's up edge
    ["dc", "r3", "d3", "l3"], // C, whose up edge reverses A's down edge
];

/// Distinct images, because a tile's id is a hash of its pixels and two identical
/// images would be one tile.
fn write_tiles(directory: &Path) -> Vec<TileConfig> {
    TILES
        .iter()
        .enumerate()
        .map(|(index, slots)| {
            let shade = u8::try_from(index).unwrap() * 60 + 20;
            let path = directory.join(format!("{index}.png"));

            RgbaImage::from_pixel(2, 2, Rgba([shade, shade, shade, 255]))
                .save(&path)
                .unwrap();

            TileConfig {
                image: path,
                slots: slots.iter().map(ToString::to_string).collect(),
            }
        })
        .collect()
}

#[test]
fn matching_edge_labels_face_each_other() {
    let directory = tempfile::tempdir().unwrap();
    let tiles = tiles::from_config(&write_tiles(directory.path())).unwrap();

    let ids: Vec<u64> = tiles.iter().map(Collapsable::get_id).collect();
    let [a, b, c] = tiles.as_slice() else {
        panic!("expected one tile per config, got {}", tiles.len());
    };

    assert_eq!(
        a.neighbors[Direction::Up],
        Set::from_iter([ids[1]]),
        "B's down edge reverses A's up edge, so B is what may sit above A"
    );
    assert_eq!(
        a.neighbors[Direction::Down],
        Set::from_iter([ids[2]]),
        "C's up edge reverses A's down edge, so C is what may sit below A"
    );
    assert!(a.neighbors[Direction::Left].is_empty());
    assert!(a.neighbors[Direction::Right].is_empty());

    assert_eq!(
        b.neighbors[Direction::Down],
        Set::from_iter([ids[0]]),
        "the pairing holds from both sides"
    );
    assert_eq!(c.neighbors[Direction::Up], Set::from_iter([ids[0]]));
}

#[test]
fn a_tile_needs_four_edge_labels() {
    use wfc_cli::Error;

    let directory = tempfile::tempdir().unwrap();
    let mut configs = write_tiles(directory.path());
    configs[1].slots.pop();

    assert!(matches!(
        tiles::from_config(&configs),
        Err(Error::SlotCount { found: 3 })
    ));
}

#[test]
fn a_missing_tile_image_is_reported() {
    use wfc_cli::Error;

    let configs = vec![TileConfig {
        image: Path::new("/nonexistent/tile.png").to_path_buf(),
        slots: vec!["a".into(), "b".into(), "c".into(), "d".into()],
    }];

    assert!(matches!(
        tiles::from_config(&configs),
        Err(Error::Tile { .. })
    ));
}
