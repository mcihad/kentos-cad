//! Çoklu çizgi's AutoCAD PLINE pairs, its arc-mode Kapat and Genişlik, tapered widths, and Kenar düzenle on an
//! edge (docs/adr/0288). The values are worked out by hand from the geometry.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::{EntitiesVertex, Entity, SubPlace, VertexAction};
use kentos_interaction::segment_edit::{Ask, SegmentEdit};
use kentos_interaction::{Level, Vec2};

const QUARTER: f64 = 0.414_213_562_373_095_1; // tan(π/8): a quarter circle's bulge

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn newest(b: &Bench) -> kentos_contracts::PathEntity {
    let Entity::Polyline(path) = b.newest() else {
        panic!("a polyline, not {:?}", b.newest());
    };
    path.clone()
}

fn pts(p: &kentos_contracts::PathEntity) -> Vec<[f64; 2]> {
    p.pts.iter().map(|q| rel(*q)).collect()
}

/// A polyline from (0, 0) to (10, 0), arc mode on.
fn arcs() -> Bench {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.type_text("Y"));
    b
}

fn ends_near(p: &[[f64; 2]], q: [f64; 2]) -> bool {
    let last = p[p.len() - 1];
    near(last[0], q[0]) && near(last[1], q[1])
}

/// Açı, then Merkez: the centre clicked turns the start by the angle.
#[test]
fn angle_then_centre() {
    let mut b = arcs();
    assert!(b.type_text("A"));
    assert!(b.type_text("90"));
    assert!(b.type_text("M"));
    assert_eq!(
        b.session.prompt().text(),
        "Çoklu çizgi: yayın merkezini gösterin [Düz (D) / Açı (A) / Merkez (M) / Yarıçap (R) / İkinci nokta (İ) / Doğrultu (T) / Genişlik (E): 0.000 m / Kapat (K) / Geri (G) / Bitir (Enter)]"
    );
    b.click(10.0, 10.0);
    b.confirm();
    let p = newest(&b);
    assert!(ends_near(&pts(&p), [20.0, 10.0]), "{:?}", pts(&p));
    assert!(near(p.bulges.as_ref().expect("bulges")[1], QUARTER));
}

/// Açı, then Yarıçap: the radius typed, then the chord laid toward the pointer.
#[test]
fn angle_then_radius() {
    let mut b = arcs();
    assert!(b.type_text("A"));
    assert!(b.type_text("180"));
    assert!(b.type_text("R"));
    assert!(b.type_text("5"));
    assert_eq!(
        b.session.prompt().options.first().map(|o| o.key),
        Some("D"),
        "the arc options stay while the chord's direction is asked"
    );
    b.click(10.0, 30.0);
    b.confirm();
    let p = newest(&b);
    assert!(
        ends_near(&pts(&p), [10.0, 10.0]),
        "a chord of 10 north: {:?}",
        pts(&p)
    );
    assert!(near(p.bulges.as_ref().expect("bulges")[1], 1.0));
}

/// Merkez, then Açı: typed, it ends the arc.
#[test]
fn centre_then_angle() {
    let mut b = arcs();
    assert!(b.type_text("M"));
    b.click(10.0, 10.0);
    assert!(b.type_text("A"));
    assert!(b.type_text("-90"));
    b.confirm();
    let p = newest(&b);
    assert!(ends_near(&pts(&p), [0.0, 10.0]), "{:?}", pts(&p));
    assert!(near(p.bulges.as_ref().expect("bulges")[1], -QUARTER));
}

/// Merkez, then Uzunluk: a chord of 10 on a radius of 10 is a 60° arc.
#[test]
fn centre_then_chord() {
    let mut b = arcs();
    assert!(b.type_text("M"));
    b.click(10.0, 10.0);
    assert!(b.options().contains(&"U"));
    assert!(b.type_text("U"));
    assert!(b.type_text("10"));
    b.confirm();
    let p = newest(&b);
    let end = pts(&p)[2];
    assert!(
        near(end[0], 10.0 + 10.0 * (3f64).sqrt() / 2.0) && near(end[1], 5.0),
        "{end:?}"
    );
    // A chord longer than the diameter is said; the arc waits.
    let mut b = arcs();
    assert!(b.type_text("M"));
    b.click(10.0, 10.0);
    assert!(b.type_text("U"));
    assert!(b.type_text("30"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.points(), 2);
}

/// Yarıçap, then Açı: the chord's direction ends the arc; a click while a value is waited for is said.
#[test]
fn radius_then_angle() {
    let mut b = arcs();
    assert!(b.type_text("R"));
    assert!(b.type_text("5"));
    assert!(b.type_text("A"));
    b.click(30.0, 30.0);
    assert_eq!(
        b.last_level(),
        Some(Level::Warn),
        "the angle is typed, not clicked"
    );
    assert!(b.type_text("180"));
    b.click(20.0, 0.0);
    b.confirm();
    let p = newest(&b);
    assert!(ends_near(&pts(&p), [20.0, 0.0]), "{:?}", pts(&p));
    assert!(near(p.bulges.as_ref().expect("bulges")[1], 1.0));
}

/// Kapat in arc mode: the closing edge is an arc tangent to the last.
#[test]
fn closing_in_arc_mode_is_a_tangent_arc() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.click(10.0, 10.0);
    assert!(b.type_text("Y"));
    assert!(b.options().contains(&"K"));
    assert!(b.type_text("K"));
    let p = newest(&b);
    assert!(p.closed);
    // Leaving (10, 10) northward and ending at (0, 0): the tangent and the chord 135° apart, a 270° arc to the left,
    // bulge tan(67.5°) = 1 + √2.
    let bulges = p.bulges.expect("the closing arc");
    assert!(near(bulges[2], 1.0 + 2f64.sqrt()), "{bulges:?}");
}

/// Genişlik: start then end; the next edge tapers, the ones after keep the end width; Enter gives the start's.
#[test]
fn tapered_widths_go_with_the_edges() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.type_text("E"));
    assert!(b.type_text("2"));
    assert_eq!(
        b.session.prompt().text(),
        "Çoklu çizgi: bitiş genişliğini yazın (Enter: 2.000 m) [Geri (G)]"
    );
    assert!(b.type_text("0.5"));
    b.click(10.0, 0.0);
    b.click(20.0, 0.0);
    assert!(b.type_text("E"));
    assert!(b.type_text("1"));
    b.confirm();
    b.click(30.0, 0.0);
    b.confirm();
    let p = newest(&b);
    assert_eq!(
        p.widths,
        Some(vec![[2.0, 0.5], [0.5, 0.5], [1.0, 1.0], [0.0, 0.0]])
    );
    // Geri takes the edge and its widths back.
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.type_text("E"));
    assert!(b.type_text("1"));
    b.confirm();
    b.click(10.0, 0.0);
    assert!(b.type_text("G"));
    assert!(b.type_text("E"));
    assert!(b.type_text("0"));
    b.confirm();
    b.click(5.0, 5.0);
    b.confirm();
    assert_eq!(
        newest(&b).widths,
        None,
        "the edge drawn again without width"
    );
}

fn edge_tool(b: &mut Bench, ask: Ask, action: VertexAction) {
    let slot = b.add_path("cizim", &[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]], false);
    let uid = b.doc.uid(slot).expect("its id").to_string();
    let input = EntitiesVertex {
        uid,
        action,
        place: SubPlace::Mid {
            part: 0,
            ring: 0,
            index: 0,
        },
        through: None,
        radius: None,
        widths: None,
        topology: None,
        expected_revision: None,
    };
    let (a, z) = (
        Vec2::new(common::E, common::N),
        Vec2::new(common::E + 10.0, common::N),
    );
    b.session.run(Box::new(SegmentEdit::new(input, ask, a, z)));
}

/// Kenar düzenle's Yay…: the arc through the point clicked, through `cad.entities.vertex`; the tool ends.
#[test]
fn the_edge_becomes_the_arc_through_a_point() {
    let mut b = Bench::new("line");
    edge_tool(&mut b, Ask::Arc, VertexAction::Arc);
    b.move_to(5.0, -5.0);
    b.click(5.0, -5.0);
    let p = newest(&b);
    assert_eq!(p.bulges.as_deref(), Some(&[1.0, 0.0, 0.0][..]));
    assert_eq!(b.last_text(), Some("Yaya dönüştür: tamam."));
    assert_ne!(b.session.tool_id(), "segmentEdit");
    assert_eq!(b.doc.undo().as_deref(), Some("Yaya dönüştür"));
}

/// A typed radius bows to the pointer's side; one below half the chord is refused and the tool waits.
#[test]
fn a_typed_radius_bows_to_the_pointer() {
    let mut b = Bench::new("line");
    edge_tool(&mut b, Ask::Arc, VertexAction::Arc);
    b.move_to(5.0, 3.0);
    assert!(b.type_text("4"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.session.tool_id(), "segmentEdit");
    assert!(b.type_text("6.25"));
    let p = newest(&b);
    assert_eq!(
        p.bulges.as_deref(),
        Some(&[-0.5, 0.0, 0.0][..]),
        "left of travel"
    );
}

/// Kenar genişliği…: start, then end (Enter: the same).
#[test]
fn the_edge_takes_its_widths() {
    let mut b = Bench::new("line");
    edge_tool(&mut b, Ask::Width, VertexAction::Width);
    assert!(b.type_text("1"));
    assert!(b.type_text("0"));
    assert_eq!(
        newest(&b).widths,
        Some(vec![[1.0, 0.0], [0.0, 0.0], [0.0, 0.0]])
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Kenar genişliği"));
    let mut b = Bench::new("line");
    edge_tool(&mut b, Ask::Width, VertexAction::Width);
    assert!(b.type_text("0.25"));
    b.confirm();
    assert_eq!(
        newest(&b).widths,
        Some(vec![[0.25, 0.25], [0.0, 0.0], [0.0, 0.0]])
    );
}
