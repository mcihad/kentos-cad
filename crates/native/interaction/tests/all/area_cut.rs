//! Alan böl's Hedef alan (H, docs/adr/1069) on a 10 m square area: the edge clicked, 40 m² typed, the cut 4 m from it;
//! Karşı taraf gives the 40 m² to the far side; Enter cuts it in one undo step; what cannot be cut is said and nothing
//! is written.

use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use kentos_interaction::Level;

fn square() -> (Bench, kentos_domain::Slot) {
    let mut b = Bench::on(crate::common::EMPTY);
    b.draft.snap = false;
    let slot = b.add_path("cizim", &[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], true);
    hedef_alan(&mut b, slot);
    (b, slot)
}

/// Alan böl on the area at `slot`, then Hedef alan.
fn hedef_alan(b: &mut Bench, slot: kentos_domain::Slot) {
    b.selection.set([slot]);
    b.start("areaSplit");
    assert!(b.session.prompt().text().contains("Hedef alan (H)"), "{}", b.session.prompt().text());
    assert!(b.type_text("H"));
}

/// The areas of the drawing's areas, smallest first, and their lowest north (relative to N).
fn areas(b: &Bench) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polygon(p) => {
                let n = p.pts.len();
                let twice: f64 = (0..n)
                    .map(|i| {
                        let (a, c) = (&p.pts[i], &p.pts[(i + 1) % n]);
                        (a.x - E) * (c.y - N) - (c.x - E) * (a.y - N)
                    })
                    .sum();
                let low = p.pts.iter().map(|q| q.y - N).fold(f64::INFINITY, f64::min);
                Some(((twice / 2.0).abs(), low))
            }
            _ => None,
        })
        .collect();
    out.sort_by(|a, c| a.0.total_cmp(&c.0));
    out
}

#[test]
fn the_piece_asked_for_lies_beside_the_edge() {
    let (mut b, _) = square();
    b.click(5.0, 0.0);
    assert!(b.run(|s, _| s.prompt().step.contains("ayrılacak alanı")), "the edge taken");
    assert!(b.type_text("40"));
    b.confirm();
    let got = areas(&b);
    assert_eq!(got.len(), 2, "{got:?}");
    assert!((got[0].0 - 40.0).abs() < 1e-9 && (got[1].0 - 60.0).abs() < 1e-9, "{got:?}");
    assert!(got[0].1.abs() < 1e-9, "the 40 m² piece is the one on the south edge: {got:?}");
    assert_eq!(b.last_level(), Some(Level::Success), "{:?}", b.last_text());
    // One undo step takes the cut back.
    assert!(b.doc.undo().is_some());
    assert_eq!(areas(&b).len(), 1);
}

#[test]
fn karsi_taraf_gives_the_target_to_the_far_side() {
    let (mut b, _) = square();
    b.click(5.0, 0.0);
    assert!(b.type_text("30"));
    assert!(b.type_text("T"));
    b.confirm();
    let got = areas(&b);
    assert!((got[0].0 - 30.0).abs() < 1e-9 && (got[0].1 - 7.0).abs() < 1e-9, "the 30 m² piece is the north one: {got:?}");
}

#[test]
fn a_target_the_area_cannot_give_is_said_and_nothing_is_cut() {
    let (mut b, _) = square();
    b.click(5.0, 0.0);
    let revision = b.doc.revision();
    let before = b.log.len();
    assert!(b.type_text("120"));
    b.confirm();
    assert_eq!(b.doc.revision(), revision);
    let said: Vec<_> = b.said(before).into_iter().map(|(_, t)| t.to_owned()).collect();
    assert!(said.iter().any(|t| t.contains("alanın tamamından küçük")), "{said:?}");
}

#[test]
fn an_area_with_a_hole_is_left_to_alan_bol() {
    let mut b = Bench::on(crate::common::EMPTY);
    let w = |q: [f64; 2]| kentos_commands::Vec2 { x: E + q[0], y: N + q[1] };
    let path = kentos_commands::PathEntity {
        space_separator: None,
        ms: None,
        base: crate::common::base("cizim"),
        pts: [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]].into_iter().map(w).collect(),
        bulges: None,
        holes: Some(vec![kentos_commands::RingGeometry {
            pts: [[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0]].into_iter().map(w).collect(),
            bulges: None,
            zs: None,
        }]),
        zs: None,
        parts: None,
        closed: false,
        widths: None,
    };
    let slot = b.doc.add(Entity::Polygon(path)).expect("a slot");
    b.draft.snap = false;
    b.selection.set([slot]);
    b.start("areaSplit");
    assert!(b.type_text("H"));
    b.click(5.0, 0.0);
    assert!(b.last_text().is_some_and(|t| t.contains("deliksiz")), "{:?}", b.last_text());
}
