//! Koordinat oku over a snapped vertex with an elevation (docs/adr/0142): a
//! point, an end of a line, a vertex of a polyline or an area, a hole's. The
//! objects are added to the traces' empty drawing; expected values are
//! worked out by hand.

use crate::common;

use common::{Bench, E, N};
use kentos_contracts::{Entity, EntityBase, LineEntity, PathEntity, RingGeometry, Vec2};
use kentos_domain::Slot;
use kentos_interaction::Format;

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn pt(x: f64, y: f64) -> Vec2 {
    Vec2 { x: E + x, y: N + y }
}

fn base() -> EntityBase {
    common::base("cizim")
}

fn zs(v: &[Option<f64>]) -> Option<Vec<Option<f64>>> {
    Some(v.to_vec())
}

/// An open polyline along y = 22 with an elevation on two of its three vertices.
fn polyline(b: &mut Bench) -> Slot {
    b.doc
        .add(Entity::Polyline(PathEntity {
            base: base(),
            pts: vec![pt(0.0, 22.0), pt(6.0, 22.0), pt(12.0, 22.0)],
            bulges: None,
            holes: None,
            zs: zs(&[Some(1.5), None, Some(3.25)]),
            parts: None,
        }))
        .expect("a slot")
}

/// A 10 m square from (0, 30) with an elevation on every corner and a hole
/// with its own.
fn area(b: &mut Bench) -> Slot {
    b.doc
        .add(Entity::Polygon(PathEntity {
            base: base(),
            pts: vec![pt(0.0, 30.0), pt(10.0, 30.0), pt(10.0, 40.0), pt(0.0, 40.0)],
            bulges: None,
            holes: Some(vec![RingGeometry {
                pts: vec![pt(3.0, 33.0), pt(6.0, 33.0), pt(6.0, 36.0)],
                bulges: None,
                zs: zs(&[Some(7.0), Some(8.0), Some(9.0)]),
            }]),
            zs: zs(&[Some(1.0), Some(2.0), Some(3.0), Some(4.0)]),
            parts: None,
        }))
        .expect("a slot")
}

fn line(b: &mut Bench) -> Slot {
    b.doc
        .add(Entity::Line(LineEntity {
            base: base(),
            a: pt(20.0, 22.0),
            b: pt(30.0, 22.0),
            za: Some(100.0),
            zb: None,
        }))
        .expect("a line")
}

fn preview_lines(b: &Bench) -> Option<Vec<String>> {
    b.session
        .preview(&Format::default())
        .and_then(|p| p.tag)
        .map(|t| t.lines)
}

// ── Koordinat oku ───────────────────────────────────────────────────────────

#[test]
fn a_snapped_vertex_with_an_elevation_says_it_and_one_without_reads_as_before() {
    let mut b = Bench::on(EMPTY);
    polyline(&mut b);
    b.start("crsQuery");
    // Near the first vertex: the snap is on it, its elevation follows.
    b.click(0.02, 22.02);
    assert_eq!(b.last_text(), Some("Y=487000.000, X=4420022.000, Z=1.500"));
    b.click(12.02, 21.98);
    assert_eq!(b.last_text(), Some("Y=487012.000, X=4420022.000, Z=3.250"));
    // The middle vertex has none.
    b.click(6.02, 22.02);
    assert_eq!(b.last_text(), Some("Y=487006.000, X=4420022.000"));
}

#[test]
fn an_edge_s_middle_is_no_vertex_and_reads_without_one() {
    let mut b = Bench::on(EMPTY);
    b.doc
        .add(Entity::Polyline(PathEntity {
            base: base(),
            pts: vec![pt(0.0, 22.0), pt(6.0, 22.0)],
            bulges: None,
            holes: None,
            zs: zs(&[Some(10.0), Some(20.0)]),
            parts: None,
        }))
        .expect("a slot");
    b.start("crsQuery");
    // The midpoint snap: (3, 22) is on the edge, not a vertex.
    b.click(3.02, 22.02);
    assert_eq!(b.last_text(), Some("Y=487003.000, X=4420022.000"));
}

#[test]
fn a_line_s_ends_and_a_hole_s_vertices_are_vertices_too() {
    let mut b = Bench::on(EMPTY);
    line(&mut b);
    area(&mut b);
    b.start("crsQuery");
    b.click(20.02, 22.02);
    assert_eq!(
        b.last_text(),
        Some("Y=487020.000, X=4420022.000, Z=100.000")
    );
    // The line's other end has none.
    b.click(30.02, 22.02);
    assert_eq!(b.last_text(), Some("Y=487030.000, X=4420022.000"));
    // A corner of the hole.
    b.click(6.02, 36.02);
    assert_eq!(b.last_text(), Some("Y=487006.000, X=4420036.000, Z=9.000"));
    // A corner of the ring.
    b.click(10.02, 40.02);
    assert_eq!(b.last_text(), Some("Y=487010.000, X=4420040.000, Z=3.000"));
}

#[test]
fn the_cursor_carries_the_elevation_too() {
    let mut b = Bench::on(EMPTY);
    polyline(&mut b);
    b.start("crsQuery");
    b.move_to(0.02, 22.02);
    assert_eq!(
        preview_lines(&b),
        Some(vec![
            "Y 487000.000".to_owned(),
            "X 4420022.000".to_owned(),
            "Z 1.500".to_owned()
        ])
    );
    b.move_to(6.02, 22.02);
    assert_eq!(preview_lines(&b).map(|l| l.len()), Some(2));
}
