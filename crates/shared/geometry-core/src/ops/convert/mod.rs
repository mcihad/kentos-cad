//! Geometri dönüştürme (docs/adr/0218): the points, paths and areas the
//! Dönüştürme tools write, one rule for both platforms: the desktop's tools
//! call the functions with shapes, the web's (`model/ops/convert.ts`) call
//! the operations in `calls` with whole lists. The independent reference is
//! scripts/fixtures/convert_cases.py; for straight-edged areas the interior
//! point is GEOS's own (shapely's `representative_point`).
//!
//! - `points`: centroids, interior points and vertices as points (§2, §3).
//! - `paths`: a path's piece, an area's rings as lines, a line's area,
//!   edges divided (§8–§11).
//! - `table`: lines from a bearing and a length, ellipses (§5–§7).

pub mod calls;
pub mod paths;
pub mod points;
pub mod table;

use crate::entity::Shape;
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::geom::intersect::Edge;
use crate::jsmath::{cos, sin};
use crate::vec2::Vec2;

pub(crate) use calls::OPS;

/// An object's elevations, path by path in `paths`' order, a vertex's
/// `None` when it has none (the web's `elevatedPaths`: a line's two ends; a
/// polyline, then its other parts; a polygon's outer ring, its holes, then
/// each other part's ring and holes).
pub type Zs = Vec<Vec<Option<f64>>>;

/// A path of an object: its points and bulges, whether it closes, its part
/// (from 1) and ring (0 the outer ring or the path, 1 … holes).
#[derive(Clone, Debug, PartialEq)]
pub struct PathOf {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub closed: bool,
    pub part: u32,
    pub ring: u32,
}

/// An object's paths in the order of its elevations (`Zs`). An arc is its
/// two ends with its bulge; the other kinds have none.
pub fn paths(s: &Shape) -> Vec<PathOf> {
    let path =
        |pts: &[Vec2], bulges: &Option<Vec<f64>>, closed: bool, part: u32, ring: u32| PathOf {
            pts: pts.to_vec(),
            bulges: bulges.clone(),
            closed,
            part,
            ring,
        };
    match s {
        Shape::Line { a, b } => vec![path(&[*a, *b], &None, false, 1, 0)],
        Shape::Arc { c, r, a0, a1 } => {
            let at = |t: f64| Vec2::new(c.x + r * cos(t), c.y + r * sin(t));
            let bulge = Some(vec![bulge_of_sweep(arc_sweep(*a0, *a1)), 0.0]);
            vec![path(&[at(*a0), at(*a1)], &bulge, false, 1, 0)]
        }
        Shape::Polyline {
            pts, bulges, parts, ..
        } => {
            let mut out = vec![path(pts, bulges, false, 1, 0)];
            for (p, part) in parts.iter().flatten().zip(2..) {
                out.push(path(&p.pts, &p.bulges, false, part, 0));
            }
            out
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let mut out = vec![path(pts, bulges, true, 1, 0)];
            for (h, ring) in holes.iter().flatten().zip(1..) {
                out.push(path(&h.pts, &h.bulges, true, 1, ring));
            }
            for (p, part) in parts.iter().flatten().zip(2..) {
                out.push(path(&p.pts, &p.bulges, true, part, 0));
                for (h, ring) in p.holes.iter().flatten().zip(1..) {
                    out.push(path(&h.pts, &h.bulges, true, part, ring));
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

/// An arc's counter-clockwise sweep from `a0` to `a1`, in (0, 2π].
pub(crate) fn arc_sweep(a0: f64, a1: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let s = (a1 - a0) % tau;
    if s <= 0.0 { s + tau } else { s }
}

impl PathOf {
    /// How many edges it has.
    pub fn edge_count(&self) -> usize {
        let n = self.pts.len();
        match (self.closed, n) {
            (_, 0 | 1) => 0,
            (true, _) => n,
            (false, _) => n - 1,
        }
    }

    /// Its edge from vertex `i`.
    pub fn edge(&self, i: usize) -> Edge {
        let n = self.pts.len();
        let (a, b) = (self.pts[i], self.pts[(i + 1) % n]);
        match bulge_arc(a, b, bulge_at(self.bulges.as_deref(), i)) {
            Some(arc) => Edge::Arc {
                c: arc.c,
                r: arc.r,
                a0: arc.a0,
                sweep: arc.sweep,
            },
            None => Edge::Seg { a, b },
        }
    }
}

/// A shape with its paths' elevations, none when no vertex has one.
#[derive(Clone, Debug, PartialEq)]
pub struct Elevations {
    pub shape: Shape,
    pub zs: Option<Zs>,
}

/// The elevations, kept when a vertex of them has one.
pub(crate) fn kept(zs: Zs) -> Option<Zs> {
    zs.iter()
        .any(|z| z.iter().any(Option::is_some))
        .then_some(zs)
}

/// Path `k`'s vertex `i`'s elevation.
pub(crate) fn z_of(zs: Option<&Zs>, k: usize, i: usize) -> Option<f64> {
    zs.and_then(|z| z.get(k))
        .and_then(|z| z.get(i))
        .copied()
        .flatten()
}

/// The elevation a fraction `t` along path `k`'s edge from vertex `i` to `j`
/// (docs/adr/0142): at its ends the vertices' own, inside it linearly (by
/// length on a straight edge, by angle on an arc); none unless both ends have one.
pub(crate) fn z_along(zs: Option<&Zs>, k: usize, i: usize, j: usize, t: f64) -> Option<f64> {
    if t == 0.0 {
        return z_of(zs, k, i);
    }
    if t == 1.0 {
        return z_of(zs, k, j);
    }
    let (za, zb) = (z_of(zs, k, i)?, z_of(zs, k, j)?);
    Some(za + (zb - za) * t)
}
