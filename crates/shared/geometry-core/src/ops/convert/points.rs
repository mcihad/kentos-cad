//! Nesneden nokta and Köşeleri noktaya çevir (docs/adr/0218 §2, §3): an
//! object's centroid or interior point, and its vertices as points.

use super::{PathOf, Zs, paths, z_along, z_of};
use crate::api::json::{FromJson, Json};
use crate::entity::{Shape, area_parts, entity_area};
use crate::geom::centroid::shape_centroid;
use crate::geom::intersect::{Edge, point_at};
use crate::jsmath::{atan2, cos, js_hypot, js_max_all, js_min_all, sin};
use crate::ops::edges::{edge_length, entity_edges};
use crate::ops::path::{path_of, point_at_s};
use crate::vec2::Vec2;

// ── Ağırlık merkezi ───────────────────────────────────────────────────

/// The length-weighted centroid of edges, about the first edge's start (big
/// map coordinates would cancel the decimals away otherwise): a segment's
/// middle, an arc's centroid in closed form, on its bisector r·sin α / α from
/// the circle's centre (α half its sweep). `None` without length.
fn edges_centroid(edges: &[Edge]) -> Option<Vec2> {
    let o = match edges.first()? {
        Edge::Seg { a, .. } => *a,
        Edge::Arc { c, .. } => *c,
    };
    let (mut sx, mut sy, mut total) = (0.0, 0.0, 0.0);
    for e in edges {
        let len = edge_length(e);
        if !(len > 0.0) {
            continue;
        }
        let m = match *e {
            Edge::Seg { a, b } => Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0),
            Edge::Arc { c, r, a0, sweep } => {
                let half = sweep.abs() / 2.0;
                let k = r * sin(half) / half;
                let mid = a0 + sweep / 2.0;
                Vec2::new(c.x + k * cos(mid), c.y + k * sin(mid))
            }
        };
        sx += (m.x - o.x) * len;
        sy += (m.y - o.y) * len;
        total += len;
    }
    (total > 0.0).then(|| Vec2::new(o.x + sx / total, o.y + sy / total))
}

/// The first vertex an object has.
fn first_vertex(s: &Shape) -> Option<Vec2> {
    match s {
        Shape::Point { p, .. } => Some(*p),
        Shape::Line { a, .. } => Some(*a),
        Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } => pts.first().copied(),
        Shape::Circle { c, r } => Some(Vec2::new(c.x + r, c.y)),
        Shape::Arc { c, r, a0, .. } => Some(Vec2::new(c.x + r * cos(*a0), c.y + r * sin(*a0))),
        _ => None,
    }
}

/// The centroid (§2): an area's of its area (holes out, parts by their
/// areas; a circle's centre), of its boundary when it has no area; a path's
/// length-weighted; a point itself, a multi-point object its points' mean;
/// the first vertex when there is nothing to weigh.
pub fn centroid(s: &Shape) -> Option<Vec2> {
    match s {
        Shape::Point { .. } => shape_centroid(s),
        Shape::Polygon { .. } | Shape::Circle { .. } if entity_area(s).is_some_and(|a| a > 0.0) => {
            shape_centroid(s)
        }
        _ => edges_centroid(&entity_edges(s)).or_else(|| first_vertex(s)),
    }
}

// ── İç nokta ─────────────────────────────────────────────────────────

/// Whether the angle `t` lies on an arc from `a0` sweeping `sweep`.
fn on_sweep(t: f64, a0: f64, sweep: f64) -> bool {
    let tau = std::f64::consts::TAU;
    let d = (if sweep >= 0.0 { t - a0 } else { a0 - t }).rem_euclid(tau);
    d <= sweep.abs()
}

/// The heights the scan line is chosen between: an edge's start and an
/// arc's top and bottom when they are on it.
fn heights(e: &Edge, out: &mut Vec<f64>) {
    match *e {
        Edge::Seg { a, .. } => out.push(a.y),
        Edge::Arc { c, r, a0, sweep } => {
            out.push(c.y + r * sin(a0));
            let half_pi = std::f64::consts::FRAC_PI_2;
            for t in [half_pi, -half_pi] {
                if on_sweep(t, a0, sweep) {
                    out.push(c.y + r * sin(t));
                }
            }
        }
    }
}

/// Where an edge crosses the line y = `scan`, GEOS's rules
/// (InteriorPointArea): a straight edge crosses when its ends are not both
/// above or both below, a horizontal one never, a downward one not at its
/// start, an upward one not at its end, at x0 + (scan − y0) / m; an arc where
/// its circle meets the line within its sweep.
fn crossings(e: &Edge, scan: f64, out: &mut Vec<f64>) {
    match *e {
        Edge::Seg { a, b } => {
            let (y0, y1) = (a.y, b.y);
            if (y0 > scan && y1 > scan) || (y0 < scan && y1 < scan) {
                return;
            }
            if y0 == y1 || (y0 == scan && y1 < scan) || (y1 == scan && y0 < scan) {
                return;
            }
            out.push(if a.x == b.x {
                a.x
            } else {
                let m = (y1 - y0) / (b.x - a.x);
                a.x + (scan - y0) / m
            });
        }
        Edge::Arc { c, r, a0, sweep } => {
            let dy = scan - c.y;
            if !(dy.abs() < r) {
                return;
            }
            let dx = (r * r - dy * dy).sqrt();
            for x in [c.x - dx, c.x + dx] {
                if on_sweep(atan2(dy, x - c.x), a0, sweep) {
                    out.push(x);
                }
            }
        }
    }
}

/// A single-part area's (its rings') widest interval on its scan line: the
/// line halfway between the heights just below and just above the middle
/// of its height range; (width, middle), none when it has no width.
fn widest(rings: &[Vec<Edge>]) -> Option<(f64, Vec2)> {
    let mut ys = Vec::new();
    for e in rings.iter().flatten() {
        heights(e, &mut ys);
    }
    let lo = js_min_all(ys.iter().copied());
    let hi = js_max_all(ys.iter().copied());
    if !(lo < hi) {
        return None;
    }
    let centre = (lo + hi) / 2.0;
    let (mut below, mut above) = (lo, hi);
    for &y in &ys {
        if y <= centre {
            if y > below {
                below = y;
            }
        } else if y < above {
            above = y;
        }
    }
    let scan = (below + above) / 2.0;
    let mut xs = Vec::new();
    for e in rings.iter().flatten() {
        crossings(e, scan, &mut xs);
    }
    xs.sort_by(f64::total_cmp);
    let mut best: Option<(f64, Vec2)> = None;
    for pair in xs.chunks_exact(2) {
        let width = pair[1] - pair[0];
        if width > best.map_or(0.0, |(w, _)| w) {
            best = Some((width, Vec2::new((pair[0] + pair[1]) / 2.0, scan)));
        }
    }
    best
}

/// A path's edges, zero-length ones left out.
fn ring_edges(p: &PathOf) -> Vec<Edge> {
    (0..p.edge_count())
        .filter(|&i| p.pts[i] != p.pts[(i + 1) % p.pts.len()])
        .map(|i| p.edge(i))
        .collect()
}

/// The interior point (§2), on the object always: an area's the middle of
/// the widest interval inside it on its scan line (GEOS's rule, with arcs;
/// the parts' widest, the first of equal ones), its boundary's halfway
/// point when it has no width; a path's halfway point (its first part's);
/// a point itself, a multi-point object's point nearest its points' mean
/// (the first of equally near ones).
pub fn interior_point(s: &Shape) -> Option<Vec2> {
    match s {
        Shape::Point { .. } => {
            let mean = shape_centroid(s)?;
            let mut best: Option<(f64, Vec2)> = None;
            for part in area_parts(s).iter() {
                if let Shape::Point { p, .. } = part {
                    let d = js_hypot(p.x - mean.x, p.y - mean.y);
                    if best.is_none_or(|(b, _)| d < b) {
                        best = Some((d, *p));
                    }
                }
            }
            best.map(|(_, p)| p)
        }
        Shape::Circle { c, .. } => Some(*c),
        Shape::Polygon { .. } => {
            let mut best: Option<(f64, Vec2)> = None;
            for part in area_parts(s).iter() {
                let rings: Vec<Vec<Edge>> = paths(part).iter().map(ring_edges).collect();
                if let Some((w, p)) = widest(&rings)
                    && best.is_none_or(|(b, _)| w > b)
                {
                    best = Some((w, p));
                }
            }
            best.map(|(_, p)| p).or_else(|| halfway(s))
        }
        _ => halfway(s),
    }
}

/// The point halfway along an object's path: its first part's, an area's
/// outer ring's.
fn halfway(s: &Shape) -> Option<Vec2> {
    let first = area_parts(s).first().cloned()?;
    let path = path_of(&first)?;
    Some(point_at_s(&path, path.length / 2.0))
}

// ── Köşeler ───────────────────────────────────────────────────────────

/// Which vertices Köşeleri noktaya çevir takes (§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Which {
    All,
    Start,
    End,
    Ends,
    Middle,
}

impl FromJson for Which {
    fn from_json(v: &Json) -> Result<Which, String> {
        match String::from_json(v)?.as_str() {
            "all" => Ok(Which::All),
            "start" => Ok(Which::Start),
            "end" => Ok(Which::End),
            "ends" => Ok(Which::Ends),
            "middle" => Ok(Which::Middle),
            other => Err(format!("bilinmeyen köşe seçimi: {other}")),
        }
    }
}

/// A point Köşeleri noktaya çevir writes: its place and elevation, its
/// number in the object (from 1; none for the middle, which is no vertex),
/// its part and ring, its distance along its ring or path from its start.
#[derive(Clone, Debug, PartialEq)]
pub struct VertexPoint {
    pub p: Vec2,
    pub z: Option<f64>,
    pub index: Option<u32>,
    pub part: u32,
    pub ring: u32,
    pub distance: f64,
}

crate::json_struct!(out VertexPoint {
    p,
    z,
    index,
    part,
    ring,
    distance
});

/// Köşeleri noktaya çevir (§3): the vertices `which` takes, their
/// elevations from `zs` (in `paths`' order). Every vertex: the outer ring,
/// the holes, then each other part's, a closed ring's first once; the
/// start, the end and the middle are the first path's (an area's outer
/// ring's, whose start and end are its first vertex).
pub fn vertex_points(s: &Shape, zs: Option<&Zs>, which: Which) -> Vec<VertexPoint> {
    let mut out = Vec::new();
    let mut index = 0;
    for (k, path) in paths(s).iter().enumerate() {
        let n = path.pts.len();
        if n == 0 || (k > 0 && which != Which::All) {
            continue;
        }
        let edges: Vec<(Edge, f64)> = (0..path.edge_count())
            .map(|i| {
                let e = path.edge(i);
                let len = edge_length(&e);
                (e, len)
            })
            .collect();
        let mut at = Vec::with_capacity(n);
        let mut along = 0.0;
        for i in 0..n {
            at.push(along);
            if let Some((_, len)) = edges.get(i) {
                along += len;
            }
        }
        let vertex = |i: usize, number: u32| VertexPoint {
            p: path.pts[i],
            z: z_of(zs, k, i),
            index: Some(number),
            part: path.part,
            ring: path.ring,
            distance: at[i],
        };
        match which {
            Which::All => {
                for i in 0..n {
                    index += 1;
                    out.push(vertex(i, index));
                }
            }
            Which::Start => out.push(vertex(0, 1)),
            Which::End if path.closed => out.push(vertex(0, 1)),
            Which::End => out.push(vertex(n - 1, n as u32)),
            Which::Ends => {
                out.push(vertex(0, 1));
                if !path.closed {
                    out.push(vertex(n - 1, n as u32));
                }
            }
            Which::Middle => {
                let half = along / 2.0;
                let mut s0 = 0.0;
                for (i, (e, len)) in edges.iter().enumerate() {
                    if half <= s0 + len || i + 1 == edges.len() {
                        let t = if *len > 0.0 {
                            ((half - s0) / len).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        out.push(VertexPoint {
                            p: point_at(e, t),
                            z: z_along(zs, k, i, (i + 1) % n, t),
                            index: None,
                            part: path.part,
                            ring: path.ring,
                            distance: half,
                        });
                        break;
                    }
                    s0 += len;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::bulge::bulge_of_sweep;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn polygon(pts: Vec<Vec2>, bulges: Option<Vec<f64>>) -> Shape {
        Shape::Polygon {
            pts,
            bulges,
            holes: None,
            parts: None,
        }
    }

    #[test]
    fn a_thin_l_has_its_centroid_outside_and_its_interior_point_inside() {
        let l = polygon(
            vec![
                v(0.0, 0.0),
                v(30.0, 0.0),
                v(30.0, 4.0),
                v(4.0, 4.0),
                v(4.0, 30.0),
                v(0.0, 30.0),
            ],
            None,
        );
        let c = centroid(&l).expect("a centroid");
        // Arms 30 × 4 and 4 × 26: (120·(15, 2) + 104·(2, 17)) / 224.
        assert!((c.x - (120.0 * 15.0 + 104.0 * 2.0) / 224.0).abs() < 1e-12);
        assert!((c.y - (120.0 * 2.0 + 104.0 * 17.0) / 224.0).abs() < 1e-12);
        // The scan line halfway between 4 (just below 15) and 30 (just above): y = 17, inside x ∈ [0, 4].
        assert_eq!(interior_point(&l), Some(v(2.0, 17.0)));
    }

    #[test]
    fn a_half_disc_arcs_centroid_is_on_its_bisector() {
        // A semicircular arc of radius 1 about the origin, from 0 to π: its centroid (0, 2/π).
        let arc = Shape::Arc {
            c: v(0.0, 0.0),
            r: 1.0,
            a0: 0.0,
            a1: std::f64::consts::PI,
        };
        let c = centroid(&arc).expect("a centroid");
        assert!(c.x.abs() < 1e-15 && (c.y - 2.0 / std::f64::consts::PI).abs() < 1e-15);
    }

    #[test]
    fn the_scan_line_reads_an_arc_top() {
        // A 2 × 1 rectangle whose top bulges up a half circle: heights 0, 1 and the top 2; the middle 1, the line
        // halfway between 1 and 2 crosses the arc.
        let s = polygon(
            vec![v(0.0, 0.0), v(2.0, 0.0), v(2.0, 1.0), v(0.0, 1.0)],
            Some(vec![0.0, 0.0, bulge_of_sweep(std::f64::consts::PI), 0.0]),
        );
        let p = interior_point(&s).expect("a point");
        assert!((p.x - 1.0).abs() < 1e-12 && (p.y - 1.5).abs() < 1e-12);
    }

    #[test]
    fn the_vertices_of_a_ring_and_its_middle() {
        let s = polygon(vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 3.0)], None);
        let zs = vec![vec![Some(10.0), Some(12.0), None]];
        let all = vertex_points(&s, Some(&zs), Which::All);
        assert_eq!(all.len(), 3);
        assert_eq!(all[2].distance, 7.0);
        let ends = vertex_points(&s, Some(&zs), Which::Ends);
        assert_eq!(ends.len(), 1);
        let mid = vertex_points(&s, Some(&zs), Which::Middle);
        // Perimeter 12, halfway 6 along: on the second edge, 2 of its 3; no elevation at its end.
        assert!((mid[0].p.x - 4.0).abs() < 1e-12 && (mid[0].p.y - 2.0).abs() < 1e-12);
        assert_eq!(mid[0].z, None);
        assert_eq!(mid[0].distance, 6.0);
    }
}
