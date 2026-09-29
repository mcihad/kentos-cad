//! Blocks' inserts in the store (docs/adr/0144): an insert is one object
//! drawn, picked, snapped and selected by its definition's pieces placed,
//! with its insertion point as a grip and a snap point; a change of the
//! definitions places every insert again. Worked by hand on a 2×1 block.

use super::Store;
use super::draw::{GROUP, LINE, MARKER};
use super::labels::LABEL_PIECE_TEXT;
use super::snap::{SnapHit, SnapKind};
use crate::geom::affine::translation;
use crate::geometry::Bounds;
use crate::vec2::Vec2;

/// Rögar: a 2×1 rectangle from its base point (1, 0), a red diagonal and a text “R”.
const ROGAR: &str = r##"[{"id":"r","name":"Rögar","base":{"x":1,"y":0},"entities":[
    {"kind":"polygon","id":1,"layerId":"","attrs":{},"pts":[{"x":1,"y":0},{"x":3,"y":0},{"x":3,"y":1},{"x":1,"y":1}]},
    {"kind":"line","id":2,"layerId":"","color":"#FF0000","attrs":{},"a":{"x":1,"y":0},"b":{"x":3,"y":1}},
    {"kind":"text","id":3,"layerId":"","attrs":{},"p":{"x":1.5,"y":0.25},"text":"R","height":0.5,"rotation":0}]}]"##;

/// The same block, now a single 4×1 line.
const LONGER: &str = r#"[{"id":"r","name":"Rögar","base":{"x":1,"y":0},"entities":[
    {"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":1,"y":0},"b":{"x":5,"y":0}}]}]"#;

/// Rögar at (100, 200), twice its size, not turned; and a point elsewhere.
fn store() -> Store {
    let mut s = Store::new();
    s.set_blocks_json(ROGAR).unwrap();
    s.put_json(
        r#"[{"id":1,"layerId":"a","kind":"insert","block":"r","p":{"x":100,"y":200},"scale":2,"rotation":0},
            {"id":2,"layerId":"a","kind":"point","p":{"x":90,"y":190}}]"#,
    )
    .unwrap();
    s
}

fn b(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Bounds {
    Bounds {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

#[test]
fn an_insert_has_its_pieces_box() {
    let s = store();
    // The rectangle (0,0)…(2,1) doubled: (100,200)…(104,202); the insertion point is its corner.
    let e = s.extent(Some(&[1.0])).unwrap();
    assert_eq!(
        (e.min_x, e.min_y, e.max_x, e.max_y),
        (100.0, 200.0, 104.0, 202.0)
    );
    let it = s.get(1.0).unwrap();
    assert_eq!(it.shapes().len(), 3);
    let x = it.expanded.as_ref().unwrap();
    assert_eq!(x.colors, [None, Some("#FF0000".into()), None]);
}

#[test]
fn an_insert_is_picked_by_its_pieces() {
    let s = store();
    // On the rectangle's top edge.
    assert_eq!(s.hit(Vec2::new(102.0, 202.05), 0.1), Some(1.0));
    // Inside it, away from every edge: the closed piece's interior.
    assert_eq!(s.hit(Vec2::new(103.5, 200.4), 0.1), Some(1.0));
    // Outside it.
    assert_eq!(s.hit(Vec2::new(106.0, 206.0), 0.1), None);
    // Edge tools do not act on it.
    assert!(s.hit_edge(Vec2::new(102.0, 202.0), 0.1).is_empty());
    // Its pieces are boundaries for trim and extend.
    assert!(!s.edges_in(&b(99.0, 199.0, 105.0, 203.0), None).is_empty());
}

#[test]
fn an_insert_snaps_to_its_pieces_and_its_insertion_point() {
    let s = store();
    let all = SnapKind::ALL.iter().fold(0, |k, s| k | s.bit());
    let at = |x: f64, y: f64| s.snap(Vec2::new(x, y), 0.3, all, None);
    // The rectangle's far corner: an endpoint of a piece.
    assert!(
        matches!(at(104.1, 202.1), Some(SnapHit { kind: SnapKind::Endpoint, point, id }) if point == Vec2::new(104.0, 202.0) && id == 1.0)
    );
    // The middle of its top edge.
    assert!(
        matches!(at(102.0, 202.1), Some(SnapHit { kind: SnapKind::Midpoint, point, .. }) if point == Vec2::new(102.0, 202.0))
    );
    // The insertion point is a node; the rectangle's corner is there too, and an endpoint weighs less.
    let hit = at(100.05, 200.05).unwrap();
    assert_eq!(hit.point, Vec2::new(100.0, 200.0));
}

#[test]
fn window_and_crossing_selection_by_the_pieces() {
    let s = store();
    // Wholly inside.
    assert_eq!(s.in_rect(&b(99.0, 199.0, 105.0, 203.0), false), [1.0]);
    // Only part of it: not by a window, by a crossing box through an edge.
    assert!(s.in_rect(&b(101.0, 199.0, 102.0, 203.0), false).is_empty());
    assert_eq!(s.in_rect(&b(101.0, 199.0, 102.0, 203.0), true), [1.0]);
    // A crossing box in its empty middle, away from every piece's edge, but inside the closed piece.
    assert_eq!(s.in_rect(&b(103.4, 200.3, 103.6, 200.5), true), [1.0]);
    // A fence through it.
    assert_eq!(
        s.in_fence(&[Vec2::new(102.0, 190.0), Vec2::new(102.0, 210.0)], 0.01),
        [1.0]
    );
}

#[test]
fn an_insert_draws_its_pieces_in_a_group() {
    let s = store();
    let out = s.drawn(&[1.0], false, None);
    // Three pieces; the text draws nothing, so two records: the rectangle's fill and the line.
    assert_eq!(&out[..2], [GROUP, 2.0]);
    assert_eq!(
        &out[2..13],
        [
            0.0, 3.0, 1.0, 4.0, 100.0, 200.0, 104.0, 200.0, 104.0, 202.0, 100.0
        ]
    );
    assert_eq!(&out[13..14], [202.0]);
    assert_eq!(
        &out[14..],
        [1.0, LINE, 1.0, 0.0, 2.0, 100.0, 200.0, 104.0, 202.0]
    );
    // A point is drawn as ever.
    assert_eq!(s.drawn(&[2.0], false, None), [MARKER, 90.0, 190.0]);
}

#[test]
fn a_block_text_is_listed_where_it_is_placed() {
    let s = store();
    let view = b(0.0, 0.0, 1000.0, 1000.0);
    // At 20 px/m the text is 1 m high as placed: 20 px.
    let out = s.labels(&view, 20.0, None);
    assert_eq!(
        out,
        [1.0, LABEL_PIECE_TEXT, 101.0, 200.5, 0.0, 1.0, 2.0, 0.0]
    );
    // Too small to draw at 2 px/m.
    assert!(s.labels(&view, 2.0, None).is_empty());
    // The host reads its string from the block's pieces.
    let pieces = s.block_pieces_json("r").unwrap();
    assert!(pieces.contains(r#""text":"R""#), "{pieces}");
    assert!(pieces.contains(r##""color":"#FF0000""##), "{pieces}");
    assert!(s.block_pieces_json("yok").is_none());
}

#[test]
fn a_changed_definition_places_every_insert_again() {
    let mut s = store();
    s.set_blocks_json(LONGER).unwrap();
    let e = s.extent(Some(&[1.0])).unwrap();
    assert_eq!(
        (e.min_x, e.min_y, e.max_x, e.max_y),
        (100.0, 200.0, 108.0, 200.0)
    );
    assert_eq!(
        s.drawn(&[1.0], false, None),
        [
            GROUP, 1.0, 0.0, LINE, 1.0, 0.0, 2.0, 100.0, 200.0, 108.0, 200.0
        ]
    );
    // Without the definition it is its insertion point again.
    s.set_blocks_json("[]").unwrap();
    assert_eq!(s.drawn(&[1.0], false, None), [MARKER, 100.0, 200.0]);
    assert_eq!(s.hit(Vec2::new(100.0, 200.0), 0.1), Some(1.0));
}

#[test]
fn a_moved_insert_shows_its_pieces_as_the_ghost() {
    let s = store();
    let ghost = s.transform_outlines(&[1.0], &[translation(10.0, 0.0)], 100);
    // The rectangle's ring among the paths, moved by 10.
    let ring = [
        1.0, 4.0, 110.0, 200.0, 114.0, 200.0, 114.0, 202.0, 110.0, 202.0,
    ];
    assert!(ghost.windows(ring.len()).any(|w| w == ring), "{ghost:?}");
}
