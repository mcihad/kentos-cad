//! Geometri üretimi (docs/adr/0219): areas and points made from objects —
//! the points' Voronoi cells and Delaunay triangles, a set's convex hull,
//! box, rotated box, least enclosing circle and concave hull, an area's pole
//! of inaccessibility. One rule for both platforms: the desktop's tools call
//! the functions, the web's (`model/ops/generate.ts`) the operations in
//! `calls` with whole lists. The independent reference is
//! scripts/fixtures/generate_cases.py (exact cells and circles by fractions,
//! GEOS through shapely, the concave hull and polylabel written from their
//! descriptions).
//!
//! - `voronoi`: cells and triangles (§2, §3).
//! - `bounding`: hull, boxes, circle (§4).
//! - `concave`: the concave hull (§5).
//! - `pole`: the pole of inaccessibility (§6).
//! - `points`: the points an object gives (§4).

pub mod bounding;
pub mod calls;
pub mod concave;
pub mod points;
pub mod pole;
#[cfg(test)]
mod tests;
pub mod voronoi;

pub(crate) use calls::OPS;

use crate::entity::Shape;
use crate::geom::arrangement::Ring;
use crate::vec2::Vec2;
use bounding::{Bound, Kind, bound};
use points::outline_points;

/// The objects' points gathered by group (`groups[i]` is object i's group, below `count`).
fn grouped(shapes: &[Shape], groups: &[usize], count: usize) -> Vec<Vec<Vec2>> {
    let mut out: Vec<Vec<Vec2>> = vec![Vec::new(); count];
    for (s, &g) in shapes.iter().zip(groups) {
        if let Some(list) = out.get_mut(g) {
            list.extend(outline_points(s));
        }
    }
    out
}

/// Whether every object is its own group, in order (Her nesne).
fn each_alone(groups: &[usize], count: usize) -> bool {
    count == groups.len() && groups.iter().enumerate().all(|(i, &g)| g == i)
}

/// Sınırlayıcı geometri (§4): each group's bounding geometry of the kind; none for a group without area.
pub fn bounding_run(
    shapes: &[Shape],
    groups: &[usize],
    count: usize,
    kind: Kind,
) -> Vec<Option<Bound>> {
    if each_alone(groups, count) && shapes.len() == count {
        return shapes
            .iter()
            .map(|s| bound(&outline_points(s), kind))
            .collect();
    }
    grouped(shapes, groups, count)
        .iter()
        .map(|pts| bound(pts, kind))
        .collect()
}

/// İçbükey zarf (§5): each group's concave hull as rings; none for a group without a triangle.
pub fn concave_run(
    shapes: &[Shape],
    groups: &[usize],
    count: usize,
    ratio: f64,
    holes: bool,
) -> Vec<Option<Vec<Vec<Vec2>>>> {
    if each_alone(groups, count) && shapes.len() == count {
        return shapes
            .iter()
            .map(|s| concave::concave_hull(&outline_points(s), ratio, holes))
            .collect();
    }
    grouped(shapes, groups, count)
        .iter()
        .map(|pts| concave::concave_hull(pts, ratio, holes))
        .collect()
}

/// A bounding geometry as the shape the tool writes: an area, or a circle.
pub fn bound_shape(b: &Bound) -> Shape {
    match b {
        Bound::Hull { ring, .. } | Bound::Box { ring, .. } | Bound::Rotated { ring, .. } => {
            area(ring.clone(), Vec::new())
        }
        Bound::Circle { c, r } => Shape::Circle { c: *c, r: *r },
    }
}

/// Rings (the outer first) as an area.
pub fn area(outer: Vec<Vec2>, holes: Vec<Vec<Vec2>>) -> Shape {
    Shape::Polygon {
        pts: outer,
        bulges: None,
        holes: (!holes.is_empty()).then(|| {
            holes
                .into_iter()
                .map(|pts| Ring { pts, bulges: None })
                .collect()
        }),
        parts: None,
    }
}

/// A ring's area (its corners, counter-clockwise positive).
pub fn ring_area(r: &[Vec2]) -> f64 {
    let Some(&o) = r.first() else {
        return 0.0;
    };
    let mut s = 0.0;
    for i in 1..r.len().saturating_sub(1) {
        s += (r[i].x - o.x) * (r[i + 1].y - o.y) - (r[i + 1].x - o.x) * (r[i].y - o.y);
    }
    s / 2.0
}

/// A concave hull's rings as an area and its net area.
pub fn hull_area(rings: &[Vec<Vec2>]) -> Option<(Shape, f64)> {
    let (outer, holes) = rings.split_first()?;
    let net = ring_area(outer) + holes.iter().map(|h| ring_area(h)).sum::<f64>();
    (net > 0.0).then(|| (area(outer.clone(), holes.to_vec()), net))
}
