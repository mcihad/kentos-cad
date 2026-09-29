//! The tag of a grip that stands on a vertex with an elevation (docs/adr/0142),
//! at rest and while it moves, and the elevation the vertex keeps as it moves.
//! The objects are added to the traces' empty drawing; expected values are
//! worked out by hand.

mod common;

use common::{Bench, E, N};
use kentos_contracts::{Entity, EntityBase, LineEntity, PathEntity, RingGeometry, Vec2};
use kentos_domain::Slot;
use kentos_interaction::Format;

const EMPTY: &str = include_str!("../../../../fixtures/interaction/v1/empty.kcad");

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

// ── The grip's tag ──────────────────────────────────────────────────────────

fn selected(b: &mut Bench, slot: Slot) {
    b.draft.snap = false;
    b.selection.set([slot]);
}

#[test]
fn resting_on_the_grip_of_a_vertex_with_an_elevation_shows_its_tag() {
    let mut b = Bench::on(EMPTY);
    let slot = polyline(&mut b);
    selected(&mut b, slot);
    b.move_to(0.1, 22.1);
    assert_eq!(preview_lines(&b), Some(vec!["Kot 1.500 m".to_owned()]));
    // The pointer is on the vertex, not on the object: nothing is hovered.
    assert_eq!(b.selection.hover(), None);
    b.move_to(12.0, 22.0);
    assert_eq!(preview_lines(&b), Some(vec!["Kot 3.250 m".to_owned()]));
    // The vertex without one: nothing changes.
    b.move_to(6.0, 22.0);
    assert_eq!(preview_lines(&b), None);
    assert_eq!(
        b.selection.hover(),
        Some(slot),
        "the object is hovered as before"
    );
    // Nor the grip of an edge's middle, nor open ground.
    b.move_to(3.0, 22.0);
    assert_eq!(preview_lines(&b), None);
    b.move_to(40.0, 5.0);
    assert_eq!(preview_lines(&b), None);
}

#[test]
fn a_hole_s_grip_and_a_line_s_end_show_theirs() {
    let mut b = Bench::on(EMPTY);
    let a = area(&mut b);
    let l = line(&mut b);
    b.draft.snap = false;
    b.selection.set([a, l]);
    b.move_to(3.0, 33.0);
    assert_eq!(preview_lines(&b), Some(vec!["Kot 7.000 m".to_owned()]));
    b.move_to(20.0, 22.0);
    assert_eq!(preview_lines(&b), Some(vec!["Kot 100.000 m".to_owned()]));
    b.move_to(30.0, 22.0);
    assert_eq!(preview_lines(&b), None, "the end without one");
}

#[test]
fn an_object_that_is_not_selected_has_no_grips_and_so_no_tag() {
    let mut b = Bench::on(EMPTY);
    polyline(&mut b);
    b.draft.snap = false;
    b.move_to(0.0, 22.0);
    assert_eq!(preview_lines(&b), None);
}

#[test]
fn the_tag_goes_when_the_grip_is_taken_and_comes_back_beside_the_distance() {
    let mut b = Bench::on(EMPTY);
    let slot = polyline(&mut b);
    selected(&mut b, slot);
    b.move_to(0.0, 22.0);
    assert_eq!(preview_lines(&b), Some(vec!["Kot 1.500 m".to_owned()]));
    // A click takes the grip: it is hot, and its tag has the distance and the elevation.
    b.click(0.0, 22.0);
    assert!(b.session.grip_active());
    b.move_to(0.0, 26.0);
    assert_eq!(
        preview_lines(&b),
        Some(vec!["4.000 m".to_owned(), "Kot 1.500 m".to_owned()])
    );
    // A vertex without an elevation has the distance alone.
    b.run(|s, cx| s.cancel(cx));
    b.click(6.0, 22.0);
    b.move_to(6.0, 25.0);
    assert_eq!(preview_lines(&b), Some(vec!["3.000 m".to_owned()]));
}

/// The vertex keeps its elevation as it moves: the grip's edit carries it by place.
#[test]
fn a_moved_vertex_keeps_its_elevation() {
    let mut b = Bench::on(EMPTY);
    let slot = polyline(&mut b);
    selected(&mut b, slot);
    b.drag([0.0, 22.0], [0.0, 26.0]);
    let Some(Entity::Polyline(p)) = b.doc.get(slot) else {
        panic!("a polyline");
    };
    assert_eq!(p.pts[0], pt(0.0, 26.0));
    assert_eq!(p.zs, zs(&[Some(1.5), None, Some(3.25)]));
    assert_eq!(b.doc.undo().as_deref(), Some("Tutamaçla düzenle"));
}
