//! Ötele's notes and the shapes it now takes (docs/adr/0293): a fit-point and a NURBS curve offset by their chords
//! (each copy's vertex the distance from the curve, within the chords' tolerance and the mitres'), an area's hole
//! going the other way, a ring turning over refused as a collapse, a copy crossing itself counted.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::curve_outline::{CURVE_TOL, spline_outline};
use kentos_geometry_core::jsmath::{js_max, js_min};
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::offset::{collapses, offset_noted};
use kentos_geometry_core::vec2::Vec2;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn seg_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy);
    let t = js_min(1.0, js_max(0.0, t));
    let (ex, ey) = (p.x - a.x - t * dx, p.y - a.y - t * dy);
    (ex * ex + ey * ey).sqrt()
}

/// The distance from `p` to a curve, by a dense sampling of it (much finer than the chords).
fn distance_to(p: Vec2, curve: &[Vec2]) -> f64 {
    curve
        .windows(2)
        .map(|w| seg_distance(p, w[0], w[1]))
        .fold(f64::INFINITY, js_min)
}

#[test]
fn a_fit_point_curve_is_offset_by_its_chords_within_the_tolerance() {
    let pts = vec![v(0.0, 0.0), v(10.0, 4.0), v(20.0, 0.0), v(30.0, 5.0)];
    let curve = Shape::Spline { pts: pts.clone(), closed: false };
    let (g, note) = offset_noted(&curve, 2.0, v(10.0, 10.0));
    let Geometry::Ok(copy) = g else { panic!("a copy") };
    let Shape::Polyline { pts: out, closed, .. } = copy.shape else { panic!("a polyline") };
    assert!(!closed);
    assert_eq!(note.approximate, Some(CURVE_TOL));
    // A fine sampling: the chords of a tolerance a hundred times smaller would do; the outline itself is within it.
    let dense = spline_outline(&pts, false);
    for p in &out[1..out.len() - 1] {
        let d = distance_to(*p, &dense);
        assert!((d - 2.0).abs() < 0.01, "{p:?} is {d} from the curve");
    }
}

#[test]
fn an_area_s_hole_goes_the_other_way() {
    let area = Shape::Polygon {
        pts: vec![v(0.0, 0.0), v(20.0, 0.0), v(20.0, 20.0), v(0.0, 20.0)],
        bulges: None,
        holes: Some(vec![kentos_geometry_core::geom::arrangement::Ring {
            pts: vec![v(6.0, 6.0), v(6.0, 14.0), v(14.0, 14.0), v(14.0, 6.0)],
            bulges: None,
        }]),
        parts: None,
    };
    let (g, note) = offset_noted(&area, 2.0, v(2.0, 2.0));
    let Geometry::Ok(copy) = g else { panic!("a copy") };
    let Shape::Polygon { pts, holes: Some(holes), .. } = copy.shape else { panic!("an area with its hole") };
    assert_eq!(pts, vec![v(2.0, 2.0), v(18.0, 2.0), v(18.0, 18.0), v(2.0, 18.0)]);
    assert_eq!(holes[0].pts, vec![v(4.0, 4.0), v(4.0, 16.0), v(16.0, 16.0), v(16.0, 4.0)]);
    assert_eq!(note.approximate, None);
    // Inward past what the hole leaves: refused as the area closing up.
    match offset_noted(&area, 5.0, v(2.0, 2.0)).0 {
        Geometry::Error(e) => assert!(collapses(&e), "{e}"),
        Geometry::Ok(_) => panic!("refused"),
    }
}

#[test]
fn a_square_offset_inward_past_its_middle_collapses() {
    let square = Shape::Polygon {
        pts: vec![v(0.0, 0.0), v(8.0, 0.0), v(8.0, 8.0), v(0.0, 8.0)],
        bulges: None,
        holes: None,
        parts: None,
    };
    match offset_noted(&square, 5.0, v(4.0, 4.5)).0 {
        Geometry::Error(e) => assert!(collapses(&e), "{e}"),
        Geometry::Ok(c) => panic!("refused, not {:?}", c.shape),
    }
}

#[test]
fn a_copy_crossing_itself_is_counted() {
    let path = Shape::Polyline {
        pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 4.0), v(6.0, 4.0), v(6.0, 1.0), v(3.0, 1.0), v(3.0, 6.0)],
        bulges: None,
        holes: None,
        parts: None,
        closed: false,
        widths: None,
    };
    let (_, note) = offset_noted(&path, 1.0, v(1.0, 0.5));
    assert_eq!(note.crossings, 2);
}
