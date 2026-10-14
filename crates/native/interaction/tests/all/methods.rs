//! The drawing tools' methods added after the comparison with AutoCAD (docs/adr/0287): Yay's eight
//! constructions given before the first point, Daire's Merkez, çap, Düzgün çokgen's circles set (not
//! turned), Dikdörtgen's Alan and its ribbon methods, Yardımcı çizgi's Paralel (and Noktadan) and
//! Revizyon bulutu's Nesne. Every value is worked out by hand from the geometry.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::Level;

const PI: f64 = std::f64::consts::PI;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn arc(b: &Bench) -> ([f64; 2], f64, f64, f64) {
    let Entity::Arc(a) = b.newest() else {
        panic!("an arc, not {:?}", b.newest());
    };
    (rel(a.c), a.r, a.a0, a.a1)
}

fn ring(b: &Bench) -> (Vec<[f64; 2]>, Vec<f64>) {
    let Entity::Polygon(p) = b.newest() else {
        panic!("a closed area, not {:?}", b.newest());
    };
    (
        p.pts.iter().map(|q| rel(*q)).collect(),
        p.bulges.clone().unwrap_or_default(),
    )
}

// ── Yay ─────────────────────────────────────────────────────────────────────

/// Başlangıç, merkez, açı: the start, the centre, the included angle typed; the method stays.
#[test]
fn arc_start_centre_angle_stays_for_the_next_arcs() {
    let mut b = Bench::new("arc");
    assert!(b.type_text("BMA"));
    b.click(10.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Yay: yayın merkezini belirtin");
    b.click(0.0, 0.0);
    assert!(b.type_text("90"));
    let (c, r, a0, a1) = arc(&b);
    assert_eq!(c, [0.0, 0.0]);
    assert!(
        near(r, 10.0) && near(a0, 0.0) && near(a1, PI / 2.0),
        "{a0} {a1}"
    );
    // The next arc goes the same way.
    b.click(5.0, 0.0);
    b.click(0.0, 0.0);
    assert!(b.type_text("180"));
    let (_, r, a0, a1) = arc(&b);
    assert!(near(r, 5.0) && near(a0, 0.0) && near(a1, PI), "{a0} {a1}");
}

/// Merkez, başlangıç, uzunluk: a chord of 10 on a radius of 10 is a 60° arc.
#[test]
fn arc_centre_start_chord() {
    let mut b = Bench::new("arc");
    assert!(b.type_text("MBU"));
    assert_eq!(b.session.prompt().text(), "Yay: yayın merkezini belirtin");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.type_text("10"));
    let (c, r, a0, a1) = arc(&b);
    assert_eq!(c, [0.0, 0.0]);
    assert!(
        near(r, 10.0) && near(a0, 0.0) && near(a1, PI / 3.0),
        "{a0} {a1}"
    );
}

/// Başlangıç, bitiş, yarıçap: a chord of 10 with a radius of 5 is a half circle on its middle.
#[test]
fn arc_start_end_radius() {
    let mut b = Bench::new("arc");
    assert!(b.type_text("BSR"));
    b.click(0.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Yay: bitiş noktasını belirtin");
    b.click(10.0, 0.0);
    assert!(b.type_text("5"));
    let (c, r, _, _) = arc(&b);
    assert!(
        near(c[0], 5.0) && near(c[1], 0.0) && near(r, 5.0),
        "{c:?} {r}"
    );
}

/// Başlangıç, bitiş, açı and başlangıç, merkez, bitiş: the third input as the method says.
#[test]
fn arc_start_end_angle_and_start_centre_end() {
    let mut b = Bench::new("arc");
    assert!(b.type_text("BSA"));
    b.click(10.0, 0.0);
    b.click(0.0, 10.0);
    assert!(b.type_text("90"));
    let (c, r, _, _) = arc(&b);
    assert!(
        near(c[0], 0.0) && near(c[1], 0.0) && near(r, 10.0),
        "{c:?} {r}"
    );
    b.start("arc");
    assert!(b.type_text("BMS"));
    b.click(10.0, 0.0);
    b.click(0.0, 0.0);
    b.click(-3.0, 0.0);
    let (c, r, a0, a1) = arc(&b);
    assert_eq!(c, [0.0, 0.0]);
    assert!(near(r, 10.0) && near(a0, 0.0) && near(a1, PI), "{a0} {a1}");
}

// ── Daire ───────────────────────────────────────────────────────────────────

/// Merkez, çap: the centre, then the diameter typed.
#[test]
fn circle_centre_diameter() {
    let mut b = Bench::new("circle");
    assert!(b.type_text("MÇ"));
    b.click(0.0, 0.0);
    assert!(b.type_text("10"));
    let Entity::Circle(c) = b.newest() else {
        panic!("a circle");
    };
    assert!(near(c.r, 5.0));
}

// ── Düzgün çokgen ───────────────────────────────────────────────────────────

/// Çember içinde and dışında set the circle, whatever it was; Çember turns it.
#[test]
fn regular_polygon_circles_are_set() {
    let mut b = Bench::new("regularPolygon");
    assert!(b.type_text("DIŞ"));
    assert!(!b.memory.polygon_inscribed);
    assert!(b.type_text("DIŞ"));
    assert!(!b.memory.polygon_inscribed, "set, not turned");
    assert!(b.type_text("İÇ"));
    assert!(b.memory.polygon_inscribed);
    b.click(0.0, 0.0);
    assert!(b.type_text("10"));
    let (pts, _) = ring(&b);
    assert_eq!(pts.len(), 6);
    assert!(
        pts.iter().all(|p| near(p[0].hypot(p[1]), 10.0)),
        "corners on the circle: {pts:?}"
    );
}

// ── Dikdörtgen ──────────────────────────────────────────────────────────────

/// Alan: 500 m² with a side of 20 is 20 × 25, opening toward the click.
#[test]
fn rectangle_by_area() {
    let mut b = Bench::new("rectangle");
    b.click(0.0, 0.0);
    assert!(b.options().contains(&"A"), "{:?}", b.options());
    assert!(b.type_text("A"));
    assert!(b.type_text("500"));
    assert!(b.type_text("20"));
    b.click(-1.0, -1.0);
    let (pts, _) = ring(&b);
    assert_eq!(
        pts,
        [[0.0, 0.0], [-20.0, 0.0], [-20.0, -25.0], [0.0, -25.0]]
    );
}

/// The ribbon's Alanla: every first corner goes on to the area.
#[test]
fn rectangle_area_method_stays() {
    let mut b = Bench::new("rectangle");
    assert!(b.type_text("ALAN"));
    for at in [0.0, 50.0] {
        b.click(at, 0.0);
        assert_eq!(b.session.prompt().text(), "Dikdörtgen: alanı yazın (m²)");
        assert!(b.type_text("100"));
        assert!(b.type_text("4"));
        b.click(at + 1.0, 1.0);
        let (pts, _) = ring(&b);
        assert_eq!(pts[2], [at + 4.0, 25.0]);
    }
    // A side of zero is said, the area kept.
    b.click(0.0, 40.0);
    assert!(b.type_text("100"));
    let before = b.log.len();
    assert!(!b.type_text("0") || b.said(before).iter().any(|(l, _)| *l == Level::Warn));
}

/// The ribbon's Boyutlarla: every first corner goes on to the size.
#[test]
fn rectangle_size_method() {
    let mut b = Bench::new("rectangle");
    assert!(b.type_text("BOYUT"));
    b.click(0.0, 0.0);
    assert!(b.type_text("20,10"));
    b.click(1.0, 1.0);
    let (pts, _) = ring(&b);
    assert_eq!(pts[2], [20.0, 10.0]);
}

// ── Yardımcı çizgi ──────────────────────────────────────────────────────────

fn xline(b: &Bench) -> ([f64; 2], [f64; 2]) {
    let Entity::Xline(x) = b.newest() else {
        panic!("a construction line, not {:?}", b.newest());
    };
    (rel(x.p), [x.dir.x, x.dir.y])
}

/// Paralel: 2 m from the line on the side clicked, one line a pick; Noktadan through the point.
#[test]
fn xline_parallel_by_distance_and_through_a_point() {
    let mut b = Bench::new("xline");
    b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    assert!(b.options().contains(&"P"));
    assert!(b.type_text("P"));
    assert_eq!(
        b.session.prompt().text(),
        "Yardımcı çizgi: paralel uzaklığı yazın [Noktadan (N)]"
    );
    assert!(b.type_text("2"));
    b.click(5.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yardımcı çizgi: hangi yana konacağını gösterin"
    );
    b.click(5.0, 3.0);
    assert_eq!(xline(&b), ([0.0, 2.0], [1.0, 0.0]));
    b.click(5.0, 0.0);
    b.click(5.0, -3.0);
    assert_eq!(xline(&b), ([0.0, -2.0], [1.0, 0.0]));
    // Nothing straight under the press: said, nothing picked.
    let n = b.doc.len();
    b.click(30.0, 30.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.len(), n);
    // Enter starts over; Noktadan.
    b.confirm();
    assert!(b.type_text("P"));
    assert!(b.type_text("N"));
    b.click(5.0, 0.0);
    b.click(3.0, 7.0);
    assert_eq!(xline(&b), ([3.0, 7.0], [1.0, 0.0]));
}

// ── Revizyon bulutu ─────────────────────────────────────────────────────────

/// Nesne: a closed area's outline gets a cloud, the area stays; an open line gets none.
#[test]
fn revision_cloud_round_an_object() {
    let mut b = Bench::new("revcloud");
    b.add_path(
        "cizim",
        &[[0.0, 0.0], [60.0, 0.0], [60.0, 60.0], [0.0, 60.0]],
        true,
    );
    b.add_line("cizim", [80.0, -20.0], [90.0, -20.0]);
    assert!(b.options().contains(&"N"));
    assert!(b.type_text("N"));
    b.click(60.0, 30.0);
    assert_eq!(b.doc.len(), 3, "the cloud beside the area");
    let (pts, bulges) = ring(&b);
    assert!(
        pts.len() >= 8 && bulges.iter().all(|k| *k != 0.0),
        "{pts:?} {bulges:?}"
    );
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("Revizyon bulutu eklendi"))
    );
    b.click(85.0, -20.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.len(), 3);
}
