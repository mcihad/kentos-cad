//! Kır, Buda and Uzat on a multi-part polyline through the session (docs/adr/0307): the pick acts on the part it is
//! near, the pieces stay parts of the one object, its attributes and elevations kept. The drawing: a three-part line
//! (its first part elevated) on `cizim` and a boundary across its first two parts at 6 m east.

use crate::common;

use common::{Bench, E, N, base};
use kentos_contracts::{AreaPart, Entity, PathEntity, Vec2};
use kentos_domain::Slot;

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn at(x: f64, y: f64) -> Vec2 {
    Vec2 { x: E + x, y: N + y }
}

fn part(pts: &[[f64; 2]]) -> AreaPart {
    AreaPart {
        pts: pts.iter().map(|p| at(p[0], p[1])).collect(),
        bulges: None,
        holes: None,
        zs: None,
        ms: None,
    }
}

/// The three-part line at slot 1 and the boundary at slot 2.
fn bench() -> Bench {
    let mut b = Bench::on(EMPTY);
    b.draft.snap = false;
    let mut own = base("cizim");
    own.attrs.insert("Ad".into(), "Hat 1".into());
    let line = Entity::Polyline(PathEntity {
        base: own,
        pts: vec![at(0.0, 0.0), at(10.0, 0.0)],
        bulges: None,
        holes: None,
        zs: Some(vec![Some(10.0), Some(20.0)]),
        ms: None,
        parts: Some(vec![
            part(&[[0.0, 5.0], [10.0, 5.0], [10.0, 9.0]]),
            part(&[[0.0, 12.0], [4.0, 12.0]]),
        ]),
        closed: false,
        widths: None,
    });
    assert_eq!(b.doc.add(line).ok(), Some(Slot(1)));
    b.add_line("cizim", [6.0, -2.0], [6.0, 14.0]);
    b
}

fn line_of(b: &Bench) -> PathEntity {
    let Some(Entity::Polyline(p)) = b.doc.get(Slot(1)) else {
        panic!("the line: {:?}", b.doc.get(Slot(1)));
    };
    p.clone()
}

fn rel(p: &[Vec2]) -> Vec<[f64; 2]> {
    p.iter().map(|q| [q.x - E, q.y - N]).collect()
}

#[test]
fn buda_cuts_the_part_picked_and_the_line_stays_one_object() {
    let mut b = bench();
    let count = b.doc.len();
    b.start("trim");
    b.click(8.0, 0.0);
    assert_eq!(b.last_text(), Some("Budandı: çizgi 3 parçalı."));
    let line = line_of(&b);
    assert_eq!(rel(&line.pts), [[0.0, 0.0], [6.0, 0.0]]);
    assert_eq!(line.zs, Some(vec![Some(10.0), Some(16.0)]));
    assert_eq!(line.base.attrs.get("Ad").map(String::as_str), Some("Hat 1"));
    let parts = line.parts.expect("its parts");
    assert_eq!(parts.len(), 2, "the other parts stay");
    assert_eq!(rel(&parts[0].pts), [[0.0, 5.0], [10.0, 5.0], [10.0, 9.0]]);
    assert_eq!(b.doc.len(), count, "no new object");
    // The second part from the boundary to its end, its corner with it.
    b.click(8.0, 5.0);
    let parts = line_of(&b).parts.expect("its parts");
    assert_eq!(parts.len(), 2);
    assert_eq!(rel(&parts[0].pts), [[0.0, 5.0], [6.0, 5.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Buda"));
    assert_eq!(b.doc.undo().as_deref(), Some("Buda"));
    assert!(line_of(&b).parts.is_some_and(|p| p.len() == 2));
}

#[test]
fn uzat_grows_the_end_of_the_part_picked() {
    let mut b = bench();
    b.start("extend");
    b.click(3.0, 12.0);
    let parts = line_of(&b).parts.expect("its parts");
    assert_eq!(rel(&parts[1].pts), [[0.0, 12.0], [6.0, 12.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Uzat"));
}

#[test]
fn kir_splits_the_part_into_two_parts_of_the_same_line() {
    let mut b = bench();
    let count = b.doc.len();
    b.start("break");
    b.click(4.0, 0.0);
    b.confirm();
    assert_eq!(b.last_text(), Some("Parça bölündü: çizgi 4 parçalı."));
    let line = line_of(&b);
    assert_eq!(rel(&line.pts), [[0.0, 0.0], [4.0, 0.0]]);
    assert_eq!(line.zs, Some(vec![Some(10.0), Some(14.0)]));
    let parts = line.parts.expect("its parts");
    assert_eq!(rel(&parts[0].pts), [[4.0, 0.0], [10.0, 0.0]]);
    assert_eq!(parts[0].zs, Some(vec![Some(14.0), Some(20.0)]));
    assert_eq!(parts.len(), 3);
    assert_eq!(b.doc.len(), count, "no new object");
    assert_eq!(b.doc.undo().as_deref(), Some("Kır"));
}
