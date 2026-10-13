//! An object's paths walked by length (docs/adr/0220 §4, §5): a line's, a
//! polyline's every part, an arc's and an area's rings (outer, holes, every
//! part) with their vertices' elevations; a circle's, an ellipse's and a
//! curve's (the last two as chords within 0.1 mm) without. A point s along
//! a path is on the edge it falls in, at a vertex the next one's start.

use crate::entity::Shape;
use crate::geom::intersect::{Edge, point_at};
use crate::jsmath::{cos, js_hypot, js_max, js_min, js_sign, sin};
use crate::ops::convert::{Zs, paths};
use crate::ops::edges::edge_length;
use crate::ops::path::path_of;
use crate::vec2::Vec2;

pub struct Walk {
    pub edges: Vec<Edge>,
    /// The length along at each edge's start.
    pub cum: Vec<f64>,
    pub length: f64,
    pub closed: bool,
    /// Each vertex's elevation (a closed path's vertices are its edges' starts); empty for none.
    pub zs: Vec<Option<f64>>,
}

/// A point along a path: where, its elevation and the unit direction the path runs there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub p: Vec2,
    pub z: Option<f64>,
    pub dir: Vec2,
}

impl Walk {
    pub fn new(edges: Vec<Edge>, closed: bool, zs: Vec<Option<f64>>) -> Walk {
        let mut cum = Vec::with_capacity(edges.len());
        let mut s = 0.0;
        for e in &edges {
            cum.push(s);
            s += edge_length(e);
        }
        Walk {
            edges,
            cum,
            length: s,
            closed,
            zs,
        }
    }

    fn z(&self, i: usize) -> Option<f64> {
        let n = self.zs.len();
        if n == 0 {
            return None;
        }
        self.zs
            .get(if self.closed { i % n } else { i })
            .copied()
            .flatten()
    }

    /// The point `s` along it (0 … length).
    pub fn at(&self, s: f64) -> Spot {
        let i = self.cum.partition_point(|&c| c <= s).saturating_sub(1);
        let e = &self.edges[i];
        let len = edge_length(e);
        let t = if len > 0.0 {
            js_min(1.0, js_max(0.0, (s - self.cum[i]) / len))
        } else {
            0.0
        };
        let z = if t == 0.0 {
            self.z(i)
        } else if t == 1.0 {
            self.z(i + 1)
        } else {
            match (self.z(i), self.z(i + 1)) {
                (Some(za), Some(zb)) => Some(za + (zb - za) * t),
                _ => None,
            }
        };
        let dir = match *e {
            Edge::Seg { a, b } => {
                let l = js_hypot(b.x - a.x, b.y - a.y);
                if l > 0.0 {
                    Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)
                } else {
                    Vec2::new(0.0, 0.0)
                }
            }
            Edge::Arc { a0, sweep, .. } => {
                let th = a0 + sweep * t;
                let w = js_sign(sweep);
                Vec2::new(-sin(th) * w, cos(th) * w)
            }
        };
        Spot {
            p: point_at(e, t),
            z,
            dir,
        }
    }
}

/// Whether the object's paths are chords of its curve (an ellipse, a curve).
pub fn chorded(s: &Shape) -> bool {
    matches!(
        s,
        Shape::Ellipse { .. } | Shape::Spline { .. } | Shape::Nurbs { .. }
    )
}

/// An object's paths in order, with their elevations (`zs` in the
/// conversion tools' order, docs/adr/0218).
pub fn walks_of(s: &Shape, zs: Option<&Zs>) -> Vec<Walk> {
    match s {
        Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. } | Shape::Polygon { .. } => {
            paths(s)
                .iter()
                .enumerate()
                .filter(|(_, p)| p.edge_count() > 0)
                .map(|(k, p)| {
                    let edges = (0..p.edge_count()).map(|i| p.edge(i)).collect();
                    let z = zs
                        .and_then(|z| z.get(k))
                        .map(|z| {
                            (0..p.pts.len())
                                .map(|i| z.get(i).copied().flatten())
                                .collect()
                        })
                        .unwrap_or_default();
                    Walk::new(edges, p.closed, z)
                })
                .collect()
        }
        Shape::Circle { .. }
        | Shape::Ellipse { .. }
        | Shape::Spline { .. }
        | Shape::Nurbs { .. } => path_of(s)
            .map(|p| vec![Walk::new(p.edges, p.closed, Vec::new())])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
