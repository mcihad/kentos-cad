//! Mekânsal sorgu (docs/adr/0200 §1): an object as a query reads it (its
//! areas with their holes, its edges, its points, its centre) and the six
//! relations Konuma göre seç and Bilgi al decide by: Kesişen, İçeren, İçinde
//! kalan, Ayrık, Uzaklıkta and Merkezi içinde. Areas are the area tools'
//! (`areas_of_entity`: arcs exact, an ellipse and a closed curve within
//! 1 mm), edges the core's (`entity_edges`); “on the boundary” and “meets”
//! are within `TOLERANCE`. The centre is the expressions' `$merkez_y`,
//! `$merkez_x` (`shape_centroid`). The store pairs objects by these
//! (`Store::relate_pairs`); the independent reference is
//! scripts/fixtures/spatial_query_cases.py (fixtures/spatial-query/v1).

use crate::entity::{Shape, area_parts, entity_anchor};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::centroid::shape_centroid;
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, on_edge_arc, point_at};
use crate::geom::region::{inside_area, ring_edges};
use crate::geometry::{Bounds, dist, empty_bounds, extend_bounds, is_empty_bounds};
use crate::jsmath::{atan2, cos, js_max, js_min, sin};
use crate::ops::areas::areas_of_entity;
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// How near counts as on a boundary or meeting, metres (docs/adr/0200 §1).
pub const TOLERANCE: f64 = 1e-3;

/// A relation between an object and a reference object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// They have a point in common.
    Intersects,
    /// The object's areas hold the reference wholly.
    Contains,
    /// The reference's areas hold the object wholly.
    Within,
    /// No point in common.
    Disjoint,
    /// Not farther apart than the given distance.
    Near,
    /// The object's centre is in or on the reference's areas.
    CenterIn,
}

impl Relation {
    pub const ALL: [Relation; 6] = [
        Relation::Intersects,
        Relation::Contains,
        Relation::Within,
        Relation::Disjoint,
        Relation::Near,
        Relation::CenterIn,
    ];

    /// Its key in the tools' values and the shared cases.
    pub fn key(self) -> &'static str {
        match self {
            Relation::Intersects => "intersects",
            Relation::Contains => "contains",
            Relation::Within => "within",
            Relation::Disjoint => "disjoint",
            Relation::Near => "near",
            Relation::CenterIn => "centerIn",
        }
    }

    pub fn from_key(key: &str) -> Option<Relation> {
        Relation::ALL.into_iter().find(|r| r.key() == key)
    }

    /// The code the web's store binding passes (its place in `ALL`).
    pub fn from_code(code: u32) -> Option<Relation> {
        Relation::ALL.get(code as usize).copied()
    }
}

/// An object as a query reads it.
#[derive(Clone, Debug)]
pub struct Geometry {
    /// The areas it encloses, with their holes.
    pub areas: Vec<Area>,
    /// The areas' boundaries.
    pub area_edges: Vec<Edge>,
    /// Every edge: an open path's and the areas' boundaries.
    pub edges: Vec<Edge>,
    /// A point object's points, or the place of a text, an insert, a table,
    /// an image or a dimension.
    pub points: Vec<Vec2>,
    /// A point of each ring and path, to find one inside another's area.
    pub starts: Vec<Vec2>,
    /// `$merkez_y`, `$merkez_x`.
    pub center: Option<Vec2>,
    pub bounds: Bounds,
}

impl Geometry {
    fn is_empty(&self) -> bool {
        self.edges.is_empty() && self.points.is_empty()
    }
}

fn area_of_hatch(ring: &[Vec2], holes: Option<&Vec<Vec<Vec2>>>) -> Area {
    Area {
        outer: Ring {
            pts: ring.to_vec(),
            bulges: None,
        },
        holes: holes
            .into_iter()
            .flatten()
            .map(|h| Ring {
                pts: h.clone(),
                bulges: None,
            })
            .collect(),
    }
}

fn extend_by_edge(b: &mut Bounds, e: &Edge) {
    match *e {
        Edge::Seg { a, b: q } => {
            extend_bounds(b, a, 0.0);
            extend_bounds(b, q, 0.0);
        }
        // The whole circle's box: a bound, enough to skip pairs far apart.
        Edge::Arc { c, r, .. } => extend_bounds(b, c, r),
    }
}

/// The object as a query reads it; none for a construction line or ray.
pub fn geometry_of(s: &Shape) -> Option<Geometry> {
    let mut g = Geometry {
        areas: Vec::new(),
        area_edges: Vec::new(),
        edges: Vec::new(),
        points: Vec::new(),
        starts: Vec::new(),
        center: None,
        bounds: empty_bounds(),
    };
    match s {
        Shape::Xline { .. } | Shape::Ray { .. } => return None,
        Shape::Point { .. } => {
            for part in area_parts(s).iter() {
                if let Shape::Point { p, .. } = part {
                    g.points.push(*p);
                }
            }
        }
        Shape::Text { .. }
        | Shape::Insert { .. }
        | Shape::Table { .. }
        | Shape::Image { .. }
        | Shape::Raster { .. }
        | Shape::PointCloud { .. }
        | Shape::Dimension { .. } => g.points.extend(entity_anchor(s)),
        Shape::Hatch { ring, holes, .. } => g.areas.push(area_of_hatch(ring, holes.as_ref())),
        _ => {
            g.areas = areas_of_entity(s);
            if g.areas.is_empty() {
                for part in area_parts(s).iter() {
                    let edges = entity_edges(part);
                    if let Some(e) = edges.first() {
                        g.starts.push(point_at(e, 0.0));
                    }
                    g.edges.extend(edges);
                }
            }
        }
    }
    for a in &g.areas {
        if let Some(p) = a.outer.pts.first() {
            g.starts.push(*p);
        }
        g.area_edges.extend(ring_edges(&a.outer));
        for h in &a.holes {
            g.area_edges.extend(ring_edges(h));
        }
    }
    g.edges.extend(g.area_edges.iter().copied());
    let mut b = empty_bounds();
    for e in &g.edges {
        extend_by_edge(&mut b, e);
    }
    for p in &g.points {
        extend_bounds(&mut b, *p, 0.0);
    }
    if is_empty_bounds(&b) {
        return None;
    }
    g.bounds = b;
    g.center = shape_centroid(s);
    Some(g)
}

fn boxes_meet(a: &Bounds, b: &Bounds, pad: f64) -> bool {
    a.min_x - pad <= b.max_x
        && b.min_x - pad <= a.max_x
        && a.min_y - pad <= b.max_y
        && b.min_y - pad <= a.max_y
}

fn inside(areas: &[Area], p: Vec2) -> bool {
    areas.iter().any(|a| inside_area(a, p))
}

fn on_edges(edges: &[Edge], p: Vec2) -> bool {
    edges.iter().any(|e| closest_on_edge(e, p).d <= TOLERANCE)
}

/// In the object's areas or on their boundaries.
fn in_or_on(g: &Geometry, p: Vec2) -> bool {
    inside(&g.areas, p) || on_edges(&g.area_edges, p)
}

/// Inside the object's areas and away from their boundaries.
fn strictly_inside(g: &Geometry, p: Vec2) -> bool {
    inside(&g.areas, p) && !on_edges(&g.area_edges, p)
}

/// A point of `g` at `p`: on its edges or points, or in its areas.
fn meets_point(g: &Geometry, p: Vec2) -> bool {
    inside(&g.areas, p)
        || on_edges(&g.edges, p)
        || g.points.iter().any(|q| dist(p, *q) <= TOLERANCE)
}

fn ends(e: &Edge) -> [Vec2; 2] {
    [point_at(e, 0.0), point_at(e, 1.0)]
}

/// The point of an arc toward `q` from its centre, when that direction is on the arc.
fn arc_toward(e: &Edge, q: Vec2) -> Option<Vec2> {
    let Edge::Arc { c, r, a0, sweep } = *e else {
        return None;
    };
    let theta = atan2(q.y - c.y, q.x - c.x);
    on_edge_arc(a0, sweep, theta).then(|| Vec2::new(c.x + r * cos(theta), c.y + r * sin(theta)))
}

/// The gap between two edges that do not cross: the least of their ends'
/// distances to the other and, for an arc, of its point toward the other
/// edge's point nearest its centre.
fn edge_gap(e1: &Edge, e2: &Edge) -> f64 {
    let mut best = f64::INFINITY;
    for p in ends(e1) {
        best = js_min(best, closest_on_edge(e2, p).d);
    }
    for p in ends(e2) {
        best = js_min(best, closest_on_edge(e1, p).d);
    }
    for (arc, other) in [(e1, e2), (e2, e1)] {
        if let Edge::Arc { c, .. } = *arc {
            let q = closest_on_edge(other, c).p;
            if let Some(p) = arc_toward(arc, q) {
                best = js_min(best, closest_on_edge(other, p).d);
            }
        }
    }
    best
}

fn edges_meet(e1: &Edge, e2: &Edge) -> bool {
    !intersect_edges(e1, e2).is_empty() || edge_gap(e1, e2) <= TOLERANCE
}

/// Kesişen: they have a point in common.
pub fn intersects(a: &Geometry, b: &Geometry) -> bool {
    if !boxes_meet(&a.bounds, &b.bounds, TOLERANCE) {
        return false;
    }
    for e1 in &a.edges {
        for e2 in &b.edges {
            if edges_meet(e1, e2) {
                return true;
            }
        }
    }
    if a.points.iter().any(|&p| meets_point(b, p)) || b.points.iter().any(|&p| meets_point(a, p)) {
        return true;
    }
    // One wholly inside the other's area: no edges met, so any of its rings or paths shows it.
    a.starts.iter().any(|&p| inside(&b.areas, p)) || b.starts.iter().any(|&p| inside(&a.areas, p))
}

/// İçeren: `a`'s areas hold `b` wholly (in or on them); `b`'s area takes in none of `a`'s holes.
pub fn contains(a: &Geometry, b: &Geometry) -> bool {
    if a.areas.is_empty() || b.is_empty() {
        return false;
    }
    let (ab, bb) = (&a.bounds, &b.bounds);
    if bb.min_x < ab.min_x - TOLERANCE
        || bb.min_y < ab.min_y - TOLERANCE
        || bb.max_x > ab.max_x + TOLERANCE
        || bb.max_y > ab.max_y + TOLERANCE
    {
        // An arc's box is its whole circle's: only a box test for the points and segments.
        if b.edges.iter().all(|e| matches!(e, Edge::Seg { .. })) {
            return false;
        }
    }
    if !b.points.iter().all(|&p| in_or_on(a, p)) {
        return false;
    }
    for e in &b.edges {
        // Cut where it meets a's boundary: every piece's middle in or on a.
        let mut ts = vec![0.0, 1.0];
        for f in &a.area_edges {
            for h in intersect_edges(e, f) {
                ts.push(h.t.clamp(0.0, 1.0));
            }
        }
        ts.sort_by(f64::total_cmp);
        if ends(e).iter().any(|&p| !in_or_on(a, p)) {
            return false;
        }
        for w in ts.windows(2) {
            if w[1] - w[0] > 1e-12 && !in_or_on(a, point_at(e, (w[0] + w[1]) / 2.0)) {
                return false;
            }
        }
    }
    // A hole of a inside b's area: b is not within a.
    !a.areas
        .iter()
        .flat_map(|area| &area.holes)
        .filter_map(|h| h.pts.first())
        .any(|&p| strictly_inside(b, p))
}

/// The least distance between them; 0 when they meet.
pub fn distance(a: &Geometry, b: &Geometry) -> f64 {
    if intersects(a, b) {
        return 0.0;
    }
    let mut best = f64::INFINITY;
    for &p in &a.points {
        for &q in &b.points {
            best = js_min(best, dist(p, q));
        }
        for e in &b.edges {
            best = js_min(best, closest_on_edge(e, p).d);
        }
    }
    for &q in &b.points {
        for e in &a.edges {
            best = js_min(best, closest_on_edge(e, q).d);
        }
    }
    for e1 in &a.edges {
        for e2 in &b.edges {
            best = js_min(best, edge_gap(e1, e2));
        }
    }
    best
}

/// Merkezi içinde: `a`'s centre in or on `b`'s areas.
pub fn center_in(a: &Geometry, b: &Geometry) -> bool {
    a.center.is_some_and(|c| in_or_on(b, c))
}

/// Whether `a` stands in `relation` to `b`; `within` the distance of Uzaklıkta.
pub fn relate(a: &Geometry, b: &Geometry, relation: Relation, within: f64) -> bool {
    match relation {
        Relation::Intersects => intersects(a, b),
        Relation::Contains => contains(a, b),
        Relation::Within => contains(b, a),
        Relation::Disjoint => !intersects(a, b),
        // Not farther than the distance: a hair over by rounding still counts.
        Relation::Near => {
            boxes_meet(&a.bounds, &b.bounds, within + TOLERANCE)
                && distance(a, b) <= within + 1e-9 * js_max(within.abs(), 1.0)
        }
        Relation::CenterIn => center_in(a, b),
    }
}

/// How far around `a`'s box a reference can be and still stand in the relation.
pub fn reach(relation: Relation, within: f64) -> f64 {
    match relation {
        Relation::Near => js_max(within, 0.0) + TOLERANCE,
        _ => TOLERANCE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Shape {
        Shape::Polygon {
            pts: vec![
                Vec2::new(x0, y0),
                Vec2::new(x1, y0),
                Vec2::new(x1, y1),
                Vec2::new(x0, y1),
            ],
            bulges: None,
            holes: None,
            parts: None,
        }
    }

    fn g(s: &Shape) -> Geometry {
        geometry_of(s).expect("a geometry")
    }

    #[test]
    fn a_square_inside_another_is_within_it_and_meets_it() {
        let big = g(&square(0.0, 0.0, 10.0, 10.0));
        let small = g(&square(2.0, 2.0, 4.0, 4.0));
        assert!(intersects(&big, &small) && contains(&big, &small) && !contains(&small, &big));
        let apart = g(&square(20.0, 0.0, 25.0, 5.0));
        assert!(!intersects(&big, &apart));
        assert_eq!(distance(&big, &apart), 10.0);
        // Sharing an edge: they meet, neither holds the other.
        let beside = g(&square(10.0, 0.0, 15.0, 5.0));
        assert!(intersects(&big, &beside) && !contains(&big, &beside));
    }
}
