//! Hızlı ölçü (docs/adr/0147 §7): the dimensions a selection of lines,
//! polylines and areas gets, by the rules `scripts/fixtures/quick_dimension_cases.py`
//! writes its independent reference from (`fixtures/dimension/v1/quick.json`):
//!
//! - a line is a path of one edge, a polyline an open path; an area is rings:
//!   its outer ring, its holes, then each further part's outer ring and holes;
//!   anything else is skipped and counted. An edge whose two ends are the same
//!   point is left out;
//! - an edge is measured once: one with the same two ends as an edge before
//!   it (either way round), for an arc the same arc (the same bulge the same
//!   way round, the opposite bulge the other way), is left out;
//! - the distance is the typed value's size, or else the cursor's distance to
//!   the nearest edge of the whole selection;
//! - the side: out of an area's outer ring, into a hole, and for an open path
//!   the cursor's side of its nearest edge (the first of the nearest);
//! - a straight edge gets an aligned dimension along it, an arc an arc length
//!   dimension about its centre, its ends counter-clockwise.

use super::DimensionGeom;
use crate::entity::Shape;
use crate::geom::bulge::{BulgeArc, bulge_arc, bulge_at, bulge_ring_area};
use crate::geom::intersect::{Edge, closest_on_edge};
use crate::jsmath::{js_hypot, js_min};
use crate::vec2::Vec2;

/// What Hızlı ölçü writes: the dimensions, and how many objects were
/// neither a line, a polyline nor an area.
#[derive(Clone, Debug, PartialEq)]
pub struct QuickDimensions {
    pub dimensions: Vec<DimensionGeom>,
    pub skipped: usize,
}

crate::json_struct!(out QuickDimensions { dimensions, skipped });

/// An edge of a path or a ring: its ends in the path's order and its bulge.
#[derive(Clone, Copy, Debug)]
struct Run {
    a: Vec2,
    b: Vec2,
    bulge: f64,
}

impl Run {
    /// Its arc, when it is one.
    fn arc(&self) -> Option<BulgeArc> {
        if self.bulge == 0.0 {
            return None;
        }
        bulge_arc(self.a, self.b, self.bulge)
    }

    fn edge(&self) -> Edge {
        match self.arc() {
            Some(arc) => Edge::Arc {
                c: arc.c,
                r: arc.r,
                a0: arc.a0,
                sweep: arc.sweep,
            },
            None => Edge::Seg {
                a: self.a,
                b: self.b,
            },
        }
    }

    fn distance(&self, p: Vec2) -> f64 {
        closest_on_edge(&self.edge(), p).d
    }

    /// 1 when `p` is left of the way the edge runs, −1 right: a segment's by
    /// the cross product's sign (0 counted left); an arc's, left being towards
    /// its centre when it runs counter-clockwise and away when clockwise (on
    /// its circle counted inside).
    fn side_of(&self, p: Vec2) -> f64 {
        match self.arc() {
            Some(arc) => {
                let inside = js_hypot(p.x - arc.c.x, p.y - arc.c.y) <= arc.r;
                if (arc.sweep > 0.0) == inside {
                    1.0
                } else {
                    -1.0
                }
            }
            None => {
                let cross = (self.b.x - self.a.x) * (p.y - self.a.y)
                    - (self.b.y - self.a.y) * (p.x - self.a.x);
                if cross < 0.0 { -1.0 } else { 1.0 }
            }
        }
    }

    /// The same edge as `other`: the same ends and bulge, or the other way
    /// round with the opposite bulge.
    fn same_as(&self, other: &Run) -> bool {
        (self.a == other.a && self.b == other.b && self.bulge == other.bulge)
            || (self.a == other.b && self.b == other.a && self.bulge == -other.bulge)
    }
}

/// Where a path's dimensions go: the cursor's side, or out of an outer ring
/// or into a hole (by its signed area, arcs included).
#[derive(Clone, Copy)]
enum Rule {
    Open,
    Outer(f64),
    Hole(f64),
}

fn runs(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Vec<Run> {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    (0..count)
        .map(|i| Run {
            a: pts[i],
            b: pts[(i + 1) % n],
            bulge: bulge_at(bulges, i),
        })
        .filter(|r| r.a != r.b)
        .collect()
}

/// A shape's paths and rings, with their rules; none for a kind that is not measured.
fn paths(shape: &Shape) -> Option<Vec<(Vec<Run>, Rule)>> {
    let ring = |pts: &[Vec2], bulges: Option<&[f64]>, outer: bool| {
        let area = bulge_ring_area(pts, bulges);
        let rule = if outer {
            Rule::Outer(area)
        } else {
            Rule::Hole(area)
        };
        (runs(pts, bulges, true), rule)
    };
    match shape {
        Shape::Line { a, b } => Some(vec![(runs(&[*a, *b], None, false), Rule::Open)]),
        Shape::Polyline { pts, bulges, .. } => {
            Some(vec![(runs(pts, bulges.as_deref(), false), Rule::Open)])
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let mut out = Vec::new();
            let mut area =
                |pts: &[Vec2], bulges: Option<&[f64]>, holes: &[crate::geom::arrangement::Ring]| {
                    out.push(ring(pts, bulges, true));
                    for h in holes {
                        out.push(ring(&h.pts, h.bulges.as_deref(), false));
                    }
                };
            area(pts, bulges.as_deref(), holes.as_deref().unwrap_or_default());
            for part in parts.as_deref().unwrap_or_default() {
                area(
                    &part.pts,
                    part.bulges.as_deref(),
                    part.holes.as_deref().unwrap_or_default(),
                );
            }
            Some(out)
        }
        _ => None,
    }
}

/// The dimensions Hızlı ölçü writes for `shapes` (in the order given) with
/// the cursor at `at` and `typed` the distance typed, if any; `height` is
/// each dimension's value height.
pub fn quick_dimensions(
    shapes: &[Shape],
    at: Vec2,
    typed: Option<f64>,
    height: f64,
) -> QuickDimensions {
    let mut all = Vec::new();
    let mut skipped = 0;
    for shape in shapes {
        match paths(shape) {
            Some(p) => all.extend(p),
            None => skipped += 1,
        }
    }
    let d = match typed {
        Some(t) => t.abs(),
        None => all
            .iter()
            .flat_map(|(runs, _)| runs.iter())
            .map(|r| r.distance(at))
            .reduce(js_min)
            .unwrap_or(0.0),
    };
    let mut seen: Vec<Run> = Vec::new();
    let mut dimensions = Vec::new();
    for (path, rule) in &all {
        let Some(first) = path.first() else {
            continue;
        };
        let s = match *rule {
            Rule::Outer(area) => {
                if area > 0.0 {
                    -1.0
                } else {
                    1.0
                }
            }
            Rule::Hole(area) => {
                if area > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Rule::Open => {
                // The first of the nearest edges.
                let mut nearest = first;
                let mut best = first.distance(at);
                for r in &path[1..] {
                    let dist = r.distance(at);
                    if dist < best {
                        nearest = r;
                        best = dist;
                    }
                }
                nearest.side_of(at)
            }
        };
        for r in path {
            if seen.iter().any(|q| q.same_as(r)) {
                continue;
            }
            seen.push(*r);
            let geom = |a, b, offset, style: Option<&str>, c| DimensionGeom {
                a,
                b,
                offset,
                height,
                style: style.map(str::to_owned),
                angle: None,
                c,
                za: None,
                zb: None,
            };
            dimensions.push(match r.arc() {
                // An arc length's offset is outwards from its centre; left of a
                // counter-clockwise arc is inwards.
                Some(arc) if arc.sweep > 0.0 => {
                    geom(r.a, r.b, -s * d, Some("arcLength"), Some(arc.c))
                }
                Some(arc) => geom(r.b, r.a, s * d, Some("arcLength"), Some(arc.c)),
                None => geom(r.a, r.b, s * d, None, None),
            });
        }
    }
    QuickDimensions {
        dimensions,
        skipped,
    }
}
