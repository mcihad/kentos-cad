//! Blocks through the session (docs/adr/0144): Patlat opens an insert into
//! its definition's objects, one level, each on its own layer when the
//! drawing has it (else the insert's), a nested insert staying an insert
//! with its similarity composed. The drawing is the traces' `blocks.kcad`
//! (fixtures/interaction/README.md). Expected values are worked out by hand.

use crate::common;

use common::Bench;
use kentos_contracts::Entity;
use kentos_domain::Slot;

const BLOCKS: &str = include_str!("../../../../../fixtures/interaction/v1/blocks.kcad");
const LAMBA: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1003";

fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(BLOCKS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

#[test]
fn explode_opens_an_insert_one_level() {
    // The pole at (487014, 4420014.8), turned −90°: its foundation, its arm and its lamp (a nested insert).
    let mut b = bench(&[7]);
    b.start("explode");
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    assert!(b.doc.get(Slot(7)).is_none());
    assert_eq!(b.selected(), [13, 14, 15]);
    // The definition's objects are on layer 0, which the drawing lacks: the insert's layer.
    for slot in [13, 14, 15] {
        let e = b.doc.get(Slot(slot)).expect("a piece");
        assert_eq!(e.base().layer_id, "aydinlatma", "piece {slot}");
        assert_eq!(e.base().color, None, "neither the object nor the insert has a colour");
    }
    let Some(Entity::Polygon(foundation)) = b.doc.get(Slot(13)) else {
        panic!("the foundation: {:?}", b.doc.get(Slot(13)));
    };
    // A quarter turn is exact: (−0.3, −0.3) goes to (−0.3, 0.3) from the insertion point.
    assert_eq!((foundation.pts[0].x, foundation.pts[0].y), (487013.7, 4420015.1));
    let Some(Entity::Line(arm)) = b.doc.get(Slot(14)) else {
        panic!("the arm");
    };
    assert_eq!((arm.a.x, arm.a.y), (487014.0, 4420014.5));
    assert_eq!((arm.b.x, arm.b.y), (487014.0, 4420012.35));
    // The lamp stays an insert of Lamba, placed at the arm's end and turned with the pole.
    let Some(Entity::Insert(lamp)) = b.doc.get(Slot(15)) else {
        panic!("the lamp: {:?}", b.doc.get(Slot(15)));
    };
    assert_eq!(lamp.block.to_text(), LAMBA);
    assert_eq!((lamp.p.x, lamp.p.y), (487014.0, 4420012.0));
    assert_eq!(lamp.scale, 1.0);
    assert!((lamp.rotation - 3.0 * std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert_eq!(b.doc.len(), 12);
}

#[test]
fn a_nested_insert_explodes_in_turn() {
    let mut b = bench(&[7]);
    b.start("explode");
    b.selection.set([Slot(15)]);
    b.start("explode");
    // Lamba: a circle and two crossing lines.
    assert_eq!(b.last_text(), Some("1 nesne patlatıldı: 3 parça."));
    let Some(Entity::Circle(c)) = b.doc.get(Slot(16)) else {
        panic!("the lamp's circle");
    };
    assert_eq!((c.c.x, c.c.y, c.r), (487014.0, 4420012.0, 0.35));
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert!(matches!(b.doc.get(Slot(15)), Some(Entity::Insert(_))));
}
