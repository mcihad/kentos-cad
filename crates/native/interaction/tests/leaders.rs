//! Leaders through the session (docs/adr/0146 §4): Patlat takes a leader
//! apart into its line on to its landing's end, its arrowhead and its note,
//! one undo step, the pieces on its layer with its colour. The drawing is the
//! leaders' picture scene (fixtures/interaction/README.md); expected values
//! are worked out by hand from the layout (§2).

mod common;

use common::Bench;
use kentos_contracts::{Entity, HatchPatternType, TextAlign};
use kentos_domain::Slot;

const LEADERS: &str = include_str!("../../../../fixtures/interaction/v1/leaders.kcad");

/// A point from the scene's origin, (500000, 4400000).
fn rel(p: kentos_contracts::Vec2) -> [f64; 2] {
    [p.x - 500_000.0, p.y - 4_400_000.0]
}

fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(LEADERS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

#[test]
fn explode_takes_a_leader_apart_into_its_line_arrowhead_and_note() {
    // “Mevcut bina”, 2.5 m high, from (0, 0) to (6, 5) with a filled arrow: the landing 5 m east, the note 1.25 m on.
    let mut b = bench(&[1]);
    b.start("explode");
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    assert!(b.doc.get(Slot(1)).is_none());
    assert_eq!(b.selected(), [6, 7, 8]);
    let Some(Entity::Polyline(line)) = b.doc.get(Slot(6)) else {
        panic!("its line: {:?}", b.doc.get(Slot(6)));
    };
    let pts: Vec<[f64; 2]> = line.pts.iter().map(|q| rel(*q)).collect();
    assert_eq!(pts, [[0.0, 0.0], [6.0, 5.0], [11.0, 5.0]]);
    assert_eq!(line.base.layer_id, "kilavuz", "the pieces keep the layer");
    let Some(Entity::Hatch(head)) = b.doc.get(Slot(7)) else {
        panic!("its arrowhead: {:?}", b.doc.get(Slot(7)));
    };
    assert_eq!(head.pattern.kind, HatchPatternType::Solid);
    assert_eq!(head.ring.len(), 3);
    assert_eq!(rel(head.ring[0]), [0.0, 0.0], "the tip first");
    let Some(Entity::Text(note)) = b.doc.get(Slot(8)) else {
        panic!("its note: {:?}", b.doc.get(Slot(8)));
    };
    assert_eq!(note.text, "Mevcut bina");
    assert_eq!(rel(note.p), [12.25, 5.0]);
    assert_eq!((note.height, note.rotation), (2.5, 0.0));
    assert_eq!(note.align, Some(TextAlign::MiddleLeft));
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert!(matches!(b.doc.get(Slot(1)), Some(Entity::Leader(_))));
}

#[test]
fn an_open_arrow_comes_apart_as_a_path_and_a_masked_note_keeps_its_mask() {
    // “Ø150 PVC”, 2 m high, its landing to the left from (36, −7): the note's middle right at (31, −7).
    let mut b = bench(&[2]);
    b.start("explode");
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    let Some(Entity::Polyline(sides)) = b.doc.get(Slot(7)) else {
        panic!("its arrowhead's sides: {:?}", b.doc.get(Slot(7)));
    };
    assert_eq!(sides.pts.len(), 3);
    assert_eq!(rel(sides.pts[1]), [44.0, -12.0], "through the tip");
    let Some(Entity::Text(note)) = b.doc.get(Slot(8)) else {
        panic!("its note");
    };
    assert_eq!(rel(note.p), [31.0, -7.0]);
    assert_eq!(note.align, Some(TextAlign::MiddleRight));
    assert!(note.mask, "the mask goes with the note");
}
