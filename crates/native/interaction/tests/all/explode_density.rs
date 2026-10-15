//! Patlat on curves through the session (docs/adr/0308): with a circle selected the tool stays, shows the polyline to
//! come and its vertices, Sayı (S), Aralık (A) and Sapma (P) choose how dense it is, a number the chosen one's value;
//! Enter writes one step “Patlat”. Without a curve it acts at once, as before.

use crate::common;

use common::{Bench, E, N};
use kentos_contracts::Entity;
use kentos_domain::Slot;

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(EMPTY);
    b.draft.snap = false;
    b
}

fn polyline(b: &Bench, slot: u32) -> (Vec<[f64; 2]>, bool) {
    let Some(Entity::Polyline(p)) = b.doc.get(Slot(slot)) else {
        panic!("a polyline at {slot}: {:?}", b.doc.get(Slot(slot)));
    };
    (p.pts.iter().map(|v| [v.x - E, v.y - N]).collect(), p.closed)
}

#[test]
fn a_circle_waits_for_its_density_and_explodes_into_a_closed_polyline() {
    let mut b = bench();
    let circle = b.add_circle("cizim", [0.0, 0.0], 10.0);
    b.selection.set([circle]);
    b.start("explode");
    assert_eq!(
        b.session.tool_id(),
        "explode",
        "the tool stays for the density"
    );
    assert_eq!(b.options(), ["Enter", "S", "A", "P"]);
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Patlat: 1 nesne, 71 köşe;"),
        "{}",
        b.session.prompt().text()
    );
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert_eq!(preview.markers.len(), 71, "a ring at each vertex");
    // Sayı: 8 pieces.
    assert!(b.type_text("S"));
    assert!(b.type_text("8"));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Patlat: 1 nesne, 8 köşe;"),
        "{}",
        b.session.prompt().text()
    );
    assert_eq!(b.doc.len(), 1, "nothing written before Enter");
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    let (pts, closed) = polyline(&b, 2);
    assert!(closed);
    assert_eq!(pts.len(), 8);
    assert!((pts[0][0] - 10.0).abs() < 1e-9 && pts[0][1].abs() < 1e-9);
    assert!(b.doc.get(circle).is_none());
    assert_eq!(b.doc.undo().as_deref(), Some("Patlat"));
    assert!(b.doc.get(circle).is_some());
}

#[test]
fn an_arc_by_a_step_in_the_project_unit() {
    let mut b = bench();
    let arc = b.add_arc("cizim", [0.0, 0.0], 5.0, 0.0, std::f64::consts::FRAC_PI_2);
    b.selection.set([arc]);
    b.start("explode");
    assert!(b.type_text("A"));
    assert!(b.type_text("1"));
    b.confirm();
    let (pts, closed) = polyline(&b, 2);
    assert!(!closed);
    // 7.854 m in pieces of at most 1 m: 8 pieces, 9 vertices, the arc's ends.
    assert_eq!(pts.len(), 9);
    assert!((pts[8][0]).abs() < 1e-9 && (pts[8][1] - 5.0).abs() < 1e-9);
}

#[test]
fn a_count_out_of_range_is_said_and_kept_out() {
    let mut b = bench();
    let circle = b.add_circle("cizim", [0.0, 0.0], 1.0);
    b.selection.set([circle]);
    b.start("explode");
    assert!(b.type_text("S"));
    assert!(b.type_text("0"));
    assert_eq!(
        b.last_text(),
        Some("Bölüm sayısı 1 ile 100 000 arasında bir tam sayı olmalı.")
    );
    assert_eq!(b.session.tool_id(), "explode");
}
