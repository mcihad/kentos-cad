//! The points an object gives Sınırlayıcı geometri and İçbükey zarf
//! (docs/adr/0219 §4, §5): its vertices and its points as written, its
//! arcs' points within 0.1 mm and the arcs' extremes along the axes (so a box
//! round arcs is exact), an ellipse's and a curve's outline within 0.1 mm
//! (`ops::geoprocess::class_of`).

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::entity::{Shape, area_parts};
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_arc, bulge_at, is_arc_bulge};
use crate::geom::intersect::Edge;
use crate::ops::geoprocess::{Class, class_of};
use crate::tessellate::{arc_points, bulge_path};
use crate::vec2::Vec2;

/// How far a curve's points may lie from it: 0.1 mm (docs/adr/0149 §5.3).
pub const CURVE_TOL: f64 = 1e-4;

/// Whether angle `t` lies on the arc from `a0` over the signed `sweep` (its ends included, no slack).
fn on_sweep(t: f64, a0: f64, sweep: f64) -> bool {
    if sweep.abs() >= TAU {
        return true;
    }
    if sweep >= 0.0 {
        norm_angle(t - a0) <= sweep
    } else {
        norm_angle(a0 - t) <= -sweep
    }
}

/// An arc's extremes along the axes that lie on it: the circle's east, north, west and south points.
fn extremes(c: Vec2, r: f64, a0: f64, sweep: f64, out: &mut Vec<Vec2>) {
    let at = [
        (0.0, Vec2::new(c.x + r, c.y)),
        (FRAC_PI_2, Vec2::new(c.x, c.y + r)),
        (2.0 * FRAC_PI_2, Vec2::new(c.x - r, c.y)),
        (3.0 * FRAC_PI_2, Vec2::new(c.x, c.y - r)),
    ];
    for (t, p) in at {
        if on_sweep(t, a0, sweep) {
            out.push(p);
        }
    }
}

/// A ring's or a path's vertices as written, its arcs' inner points and extremes.
fn bulged(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, out: &mut Vec<Vec2>) {
    out.extend(bulge_path(pts, bulges, closed, CURVE_TOL));
    let n = pts.len();
    if n < 2 {
        return;
    }
    let edges = if closed { n } else { n - 1 };
    for i in 0..edges {
        if let Some(a) = bulge_arc(pts[i], pts[(i + 1) % n], bulge_at(bulges, i)) {
            extremes(a.c, a.r, a.a0, a.sweep, out);
        }
    }
}

fn edge_points(e: &Edge, out: &mut Vec<Vec2>) {
    match *e {
        Edge::Seg { a, b } => {
            out.push(a);
            out.push(b);
        }
        Edge::Arc { c, r, a0, sweep } => {
            out.extend(arc_points(c, r, a0, sweep, CURVE_TOL));
            extremes(c, r, a0, sweep, out);
        }
    }
}

/// An object's points; none for one the tools do not take (a text, a dimension …). A point may come more than once.
pub fn outline_points(s: &Shape) -> Vec<Vec2> {
    let mut out = Vec::new();
    match s {
        Shape::Polygon { .. } => {
            for part in area_parts(s).iter() {
                if let Shape::Polygon {
                    pts, bulges, holes, ..
                } = part
                {
                    bulged(pts, bulges.as_deref(), true, &mut out);
                    for h in holes.iter().flatten() {
                        bulged(&h.pts, h.bulges.as_deref(), true, &mut out);
                    }
                }
            }
        }
        Shape::Polyline { .. } => {
            for part in area_parts(s).iter() {
                if let Shape::Polyline { pts, bulges, .. } = part {
                    bulged(pts, bulges.as_deref(), false, &mut out);
                }
            }
        }
        Shape::Circle { c, r } => {
            out.extend(arc_points(*c, *r, 0.0, TAU, CURVE_TOL));
            extremes(*c, *r, 0.0, TAU, &mut out);
        }
        _ => match class_of(s) {
            Class::Areas(areas) => {
                for a in &areas {
                    for e in crate::geom::region::ring_edges(&a.outer) {
                        edge_points(&e, &mut out);
                    }
                }
            }
            Class::Paths(paths) => {
                for e in paths.iter().flatten() {
                    edge_points(e, &mut out);
                }
            }
            Class::Points(points) => out.extend(points),
            Class::None => {}
        },
    }
    out
}

/// Whether an object enters with curves taken as points within 0.1 mm: an arc in its rings or paths, a circle, an
/// arc, an ellipse, a curve.
pub fn is_curved(s: &Shape) -> bool {
    let bent = |b: &Option<Vec<f64>>| b.iter().flatten().any(|v| is_arc_bulge(*v));
    match s {
        Shape::Circle { .. } | Shape::Arc { .. } | Shape::Ellipse { .. } | Shape::Spline { .. } => {
            true
        }
        Shape::Polygon { .. } | Shape::Polyline { .. } => area_parts(s).iter().any(|p| match p {
            Shape::Polygon { bulges, holes, .. } | Shape::Polyline { bulges, holes, .. } => {
                bent(bulges) || holes.iter().flatten().any(|h| bent(&h.bulges))
            }
            _ => false,
        }),
        _ => false,
    }
}
