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
    assert_eq!(it.shapes().count(), 3);
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
        [1.0, LABEL_PIECE_TEXT, 101.0, 200.5, 0.0, 1.0, 2.0, 1.0, 0.0]
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

#[test]
fn a_block_is_outlined_where_an_insert_would_place_it() {
    let s = store();
    // Blok ekle's ghost: the 2×1 rectangle doubled at (50, 60), then a quarter turn (exact).
    let ring = |pts: [f64; 8]| {
        let mut r = vec![1.0, 4.0];
        r.extend(pts);
        r
    };
    let flat = s.insert_outlines("r", Vec2::new(50.0, 60.0), 2.0, 0.0, false);
    let want = ring([50.0, 60.0, 54.0, 60.0, 54.0, 62.0, 50.0, 62.0]);
    assert!(flat.windows(want.len()).any(|w| w == want), "{flat:?}");
    let turned = s.insert_outlines(
        "r",
        Vec2::new(50.0, 60.0),
        2.0,
        std::f64::consts::FRAC_PI_2,
        false,
    );
    let want = ring([50.0, 60.0, 50.0, 64.0, 48.0, 64.0, 48.0, 60.0]);
    assert!(turned.windows(want.len()).any(|w| w == want), "{turned:?}");
    // Mirrored in the definition's x axis first: the rectangle hangs below.
    let mirrored = s.insert_outlines("r", Vec2::new(50.0, 60.0), 2.0, 0.0, true);
    let want = ring([50.0, 60.0, 54.0, 60.0, 54.0, 58.0, 50.0, 58.0]);
    assert!(
        mirrored.windows(want.len()).any(|w| w == want),
        "{mirrored:?}"
    );
    // A block the drawing does not define has no outline.
    assert!(
        s.insert_outlines("yok", Vec2::new(0.0, 0.0), 1.0, 0.0, false)
            .is_empty()
    );
}

#[test]
fn a_point_on_an_insert_is_found_in_its_definition() {
    let s = store();
    // Rögar at (100, 200) twice as large: its base (1, 0) is the insertion
    // point; the rectangle's far corner (3, 1) is at (104, 202).
    let local = |x, y| s.insert_local(1.0, Vec2::new(x, y));
    assert_eq!(local(100.0, 200.0), Some(Vec2::new(1.0, 0.0)));
    assert_eq!(local(104.0, 202.0), Some(Vec2::new(3.0, 1.0)));
    // Not an insert.
    assert_eq!(s.insert_local(2.0, Vec2::new(90.0, 190.0)), None);
    assert_eq!(s.insert_local(9.0, Vec2::new(0.0, 0.0)), None);
}

/// Kapı: a 1 m line from its base point (0, 0), with an attribute NO at
/// (0.5, 0.2) (default “?”, 0.25 m, level) and a hidden-by-emptiness ADI
/// (no default); Pafta holds a Kapı turned a quarter at (10, 0) whose NO is
/// “K7”, and one at (20, 0) with no value.
const WITH_ATTRIBUTES: &str = r#"[
  {"id":"k","name":"Kapı","base":{"x":0,"y":0},"entities":[
    {"kind":"line","id":1,"layerId":"","attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0}}],
   "attributes":[{"tag":"NO","value":"?","p":{"x":0.5,"y":0.2},"height":0.25,"rotation":0},
                 {"tag":"ADI","p":{"x":0,"y":-0.5},"height":0.25,"rotation":0}]},
  {"id":"p","name":"Pafta","base":{"x":0,"y":0},"entities":[
    {"kind":"insert","id":1,"layerId":"","attrs":{"NO":"K7"},"block":"k","p":{"x":10,"y":0},"scale":1,"rotation":1.5707963267948966},
    {"kind":"insert","id":2,"layerId":"","attrs":{},"block":"k","p":{"x":20,"y":0},"scale":1,"rotation":0}]}]"#;

#[test]
fn an_attribute_is_a_text_piece_of_its_tag_placed_as_the_insert_places_it() {
    let mut s = Store::new();
    s.set_blocks_json(WITH_ATTRIBUTES).unwrap();
    // Kapı at (100, 200), twice its size: NO at (101, 200.4), 0.5 m high.
    s.put_json(
        r#"[{"id":1,"layerId":"a","kind":"insert","block":"k","p":{"x":100,"y":200},"scale":2,"rotation":0}]"#,
    )
    .unwrap();
    let view = b(0.0, 0.0, 1000.0, 1000.0);
    let out = s.labels(&view, 40.0, None);
    // The line, then NO (piece 1), which the host shows with the insert's value or the default;
    // ADI (piece 2) shows nothing: no value, no default.
    assert_eq!(
        out,
        [1.0, LABEL_PIECE_TEXT, 101.0, 200.4, 0.0, 0.5, 1.0, 1.0, 0.0]
    );
    let pieces = s.block_pieces_json("k").unwrap();
    assert!(
        pieces.contains(r#""text":"?","height":0.25,"rotation":0,"attribute":"NO""#),
        "{pieces}"
    );
    assert!(pieces.contains(r#""attribute":"ADI""#), "{pieces}");
}

#[test]
fn a_nested_inserts_attributes_show_its_own_values() {
    let mut s = Store::new();
    s.set_blocks_json(WITH_ATTRIBUTES).unwrap();
    let pafta = s.blocks().get("p").unwrap();
    let texts: Vec<(String, f64, f64, Option<String>)> = pafta
        .pieces
        .iter()
        .filter_map(|p| match &p.shape {
            crate::entity::Shape::Text { p: at, text, .. } => {
                Some((text.clone(), at.x, at.y, p.attribute.clone()))
            }
            _ => None,
        })
        .collect();
    // The turned one's NO is its own “K7” at (10 − 0.2, 0.5); the other's the default “?” at (20.5, 0.2);
    // neither has an ADI to show; none is left for the outer insert to fill in.
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0].0, "K7");
    assert!((texts[0].1 - 9.8).abs() < 1e-12 && (texts[0].2 - 0.5).abs() < 1e-12);
    assert_eq!(
        (texts[1].0.as_str(), texts[1].1, texts[1].2),
        ("?", 20.5, 0.2)
    );
    assert!(texts.iter().all(|t| t.3.is_none()));
}

#[test]
fn patlat_writes_the_values_an_insert_shows() {
    let mut s = Store::new();
    s.set_blocks_json(WITH_ATTRIBUTES).unwrap();
    let blocks = s.blocks().clone();
    let insert = <crate::entity::Entity as crate::api::json::FromJson>::from_json(
        &crate::api::json::Json::parse(
            r#"{"kind":"insert","id":9,"layerId":"parsel","attrs":{"NO":"12","ADI":""},"block":"k","p":{"x":100,"y":200},"scale":2,"rotation":0}"#,
        )
        .unwrap(),
    )
    .unwrap();
    let crate::ops::curve_cuts::Cut::Pieces(pieces) = blocks.explode(&insert) else {
        panic!("pieces");
    };
    // The line, and NO's value as a text on the insert's layer; ADI shows nothing, so no text.
    assert_eq!(pieces.len(), 2);
    let crate::entity::Shape::Text {
        p, text, height, ..
    } = &pieces[1].shape
    else {
        panic!("{:?}", pieces[1].shape);
    };
    assert_eq!(
        (text.as_str(), p.x, p.y, *height),
        ("12", 101.0, 200.4, 0.5)
    );
    let layer = pieces[1].rest.iter().find(|(k, _)| k == "layerId");
    assert!(
        matches!(layer, Some((_, crate::api::json::Json::Str(l))) if l == "parsel"),
        "{:?}",
        pieces[1].rest
    );
}

/// What an insert shows of an attribute is what it is picked and measured
/// by (docs/adr/0144 §7): Kapı's NO as “K-1234567”, far longer than its default.
#[test]
fn an_attribute_is_picked_and_measured_by_the_value_it_shows() {
    let mut s = Store::new();
    s.set_blocks_json(WITH_ATTRIBUTES).unwrap();
    // Kapı at (100, 200) and at (100, 300), twice its size: NO 0.5 m high at (101, 200.4) and (101, 300.4).
    s.put_json(
        r#"[{"id":1,"layerId":"a","kind":"insert","attrs":{"NO":"K-1234567","ADI":""},"block":"k","p":{"x":100,"y":200},"scale":2,"rotation":0},
            {"id":2,"layerId":"a","kind":"insert","attrs":{},"block":"k","p":{"x":100,"y":300},"scale":2,"rotation":0}]"#,
    )
    .unwrap();
    let x = s.get(1.0).unwrap().expanded.clone().unwrap();
    assert!(
        matches!(&x.shapes[1], crate::entity::Shape::Text { text, .. } if text == "K-1234567"),
        "{:?}",
        x.shapes[1]
    );
    // 1.8 m into the value: the insert; the same place on the other, which shows “?”, is empty.
    assert_eq!(s.hit(Vec2::new(102.8, 200.6), 0.1), Some(1.0));
    assert_eq!(s.hit(Vec2::new(102.8, 300.6), 0.1), None);
    // A window around the line and the default holds the other wholly, not this one.
    assert_eq!(
        s.in_rect(&b(99.0, 199.5, 102.5, 201.5), false),
        Vec::<f64>::new()
    );
    assert_eq!(s.in_rect(&b(99.0, 299.5, 102.5, 301.5), false), [2.0]);
    let e = s.extent(Some(&[1.0])).unwrap();
    assert!(e.max_x > 102.8, "{e:?}");
}

/// An attribute that shows nothing (ADI: no value, no default) keeps its
/// place among the pieces, but is nothing to pick, snap to, label or measure.
#[test]
fn an_attribute_that_shows_nothing_is_not_there() {
    let mut s = Store::new();
    s.set_blocks_json(WITH_ATTRIBUTES).unwrap();
    s.put_json(
        r#"[{"id":1,"layerId":"a","kind":"insert","attrs":{},"block":"k","p":{"x":100,"y":200},"scale":2,"rotation":0}]"#,
    )
    .unwrap();
    let it = s.get(1.0).unwrap();
    assert_eq!(it.expanded.as_ref().unwrap().shapes.len(), 3);
    assert_eq!(it.shapes().count(), 2);
    // ADI would be at (100, 199): 1 m below the line and the insertion point.
    assert_eq!(s.hit(Vec2::new(100.05, 199.1), 0.1), None);
    let all = SnapKind::ALL.iter().fold(0, |k, s| k | s.bit());
    assert!(s.snap(Vec2::new(100.0, 199.0), 0.3, all, None).is_none());
    assert_eq!(s.extent(Some(&[1.0])).unwrap().min_y, 200.0);
    assert_eq!(s.labels(&b(0.0, 0.0, 1000.0, 1000.0), 40.0, None).len(), 9);
}
