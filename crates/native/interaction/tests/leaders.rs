//! Leaders through the session (docs/adr/0146 §4): Patlat takes a leader
//! apart into its line on to its landing's end, its arrowhead and its note,
//! one undo step, the pieces on its layer with its colour. The drawing is the
//! leaders' picture scene (fixtures/interaction/README.md); expected values
//! are worked out by hand from the layout (§2).

mod common;

use common::{Bench, E, N, base};
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

/// A leader added to the grips' drawing (fixtures/interaction/v1/objects.kcad), near (E, N):
/// from (0, 20) up to (6, 25), its note 2 m high.
fn with_leader() -> (Bench, Slot) {
    let mut b = Bench::on(include_str!("../../../../fixtures/interaction/v1/objects.kcad"));
    b.draft.snap = false;
    let at = |x: f64, y: f64| kentos_contracts::Vec2 { x: E + x, y: N + y };
    let leader = Entity::Leader(kentos_contracts::LeaderEntity {
        base: base("cizim"),
        pts: vec![at(0.0, 20.0), at(6.0, 25.0)],
        text: Some("Mevcut bina".into()),
        height: 2.0,
        rotation: 0.0,
        arrow: None,
        mask: false,
    });
    let slot = b.doc.add(leader).expect("a slot");
    b.selection.set([slot]);
    (b, slot)
}

fn leader_pts(b: &Bench, slot: Slot) -> Vec<[f64; 2]> {
    let Some(Entity::Leader(l)) = b.doc.get(slot) else {
        panic!("a leader: {:?}", b.doc.get(slot));
    };
    l.pts.iter().map(|q| [q.x - E, q.y - N]).collect()
}

#[test]
fn a_dragged_grip_moves_a_leader_s_vertex_and_a_mid_grip_adds_one() {
    let (mut b, slot) = with_leader();
    // Its last vertex, where the landing starts: the note goes with it.
    b.drag([6.0, 25.0], [8.0, 27.0]);
    assert_eq!(leader_pts(&b, slot), [[0.0, 20.0], [8.0, 27.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Tutamaçla düzenle"));
    // The segment's middle grip adds a vertex there.
    b.drag([3.0, 22.5], [2.0, 24.0]);
    assert_eq!(leader_pts(&b, slot), [[0.0, 20.0], [2.0, 24.0], [6.0, 25.0]]);
    let Some(Entity::Leader(l)) = b.doc.get(slot) else {
        panic!("a leader");
    };
    assert_eq!(l.text.as_deref(), Some("Mevcut bina"), "the note stays");
}
