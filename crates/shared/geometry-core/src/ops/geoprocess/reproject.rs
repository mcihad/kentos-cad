//! Koordinat sistemine dönüştür (docs/adr/0201 §8): an object's vertices
//! from the system its coordinates are in to the project's, the project's
//! datum choices taken (`crs::transform_in`); arcs and circles first become
//! chords within 1 mm of the curve (a projection does not keep a circle a
//! circle), ellipses and curves enter as their 0.1 mm chords.

use super::{Class, shape_of};
use crate::crs::{Choice, System, Unreached, transform_in};
use crate::entity::Shape;
use crate::geom::arrangement::{Area, Ring};
use crate::geom::intersect::{Edge, point_at};
use crate::jsmath::{acos, js_max};
use crate::vec2::Vec2;

/// How far a chord may stand off its arc (m).
pub const SAGITTA: f64 = 0.001;

/// What converting an object gave: its shape, or why it could not be
/// (the first vertex that could not be), and how many arcs became chords.
#[derive(Clone, Debug, PartialEq)]
pub struct Projected {
    pub shape: Option<Shape>,
    pub error: Option<Unreached>,
    pub chorded: usize,
}

/// An edge's points after its start: a segment's end, an arc's chords' ends within `SAGITTA`.
fn edge_points(e: &Edge, chorded: &mut usize, out: &mut Vec<Vec2>) {
    match *e {
        Edge::Seg { b, .. } => out.push(b),
        Edge::Arc { r, sweep, .. } => {
            *chorded += 1;
            let step = if r > SAGITTA {
                2.0 * acos(1.0 - SAGITTA / r)
            } else {
                sweep.abs()
            };
            let n = js_max((sweep.abs() / step).ceil(), 1.0) as usize;
            for k in 1..=n {
                out.push(point_at(e, k as f64 / n as f64));
            }
        }
    }
}

fn ring_points(r: &Ring, chorded: &mut usize) -> Vec<Vec2> {
    let edges = crate::geom::region::ring_edges(r);
    let mut pts = Vec::with_capacity(edges.len() + 1);
    if let Some(first) = edges.first() {
        pts.push(point_at(first, 0.0));
    }
    for e in &edges {
        edge_points(e, chorded, &mut pts);
    }
    // A closed ring does not repeat its first point.
    pts.pop();
    pts
}

/// The object in the system `to`, its coordinates read in `from`.
pub fn reproject(s: &Shape, from: &System, to: &System, choices: &[Choice]) -> Projected {
    let mut chorded = 0;
    let class = super::class_of(s);
    let mut moved = |p: Vec2| transform_in(from, to, p, choices).map(|t| t.point);
    let result: Result<Class, Unreached> = (|| {
        Ok(match &class {
            Class::Areas(areas) => {
                let mut out = Vec::with_capacity(areas.len());
                for a in areas {
                    let ring = |r: &Ring,
                                chorded: &mut usize,
                                moved: &mut dyn FnMut(Vec2) -> Result<Vec2, Unreached>|
                     -> Result<Ring, Unreached> {
                        let pts = ring_points(r, chorded)
                            .into_iter()
                            .map(&mut *moved)
                            .collect::<Result<Vec<Vec2>, Unreached>>()?;
                        Ok(Ring { pts, bulges: None })
                    };
                    let outer = ring(&a.outer, &mut chorded, &mut moved)?;
                    let mut holes = Vec::with_capacity(a.holes.len());
                    for h in &a.holes {
                        holes.push(ring(h, &mut chorded, &mut moved)?);
                    }
                    out.push(Area { outer, holes });
                }
                Class::Areas(out)
            }
            Class::Paths(paths) => {
                let mut out = Vec::with_capacity(paths.len());
                for path in paths {
                    let mut pts = Vec::with_capacity(path.len() + 1);
                    if let Some(first) = path.first() {
                        pts.push(point_at(first, 0.0));
                    }
                    for e in path {
                        edge_points(e, &mut chorded, &mut pts);
                    }
                    let pts = pts
                        .into_iter()
                        .map(&mut moved)
                        .collect::<Result<Vec<Vec2>, Unreached>>()?;
                    out.push(
                        pts.windows(2)
                            .map(|w| Edge::Seg { a: w[0], b: w[1] })
                            .collect(),
                    );
                }
                Class::Paths(out)
            }
            Class::Points(points) => Class::Points(
                points
                    .iter()
                    .copied()
                    .map(&mut moved)
                    .collect::<Result<Vec<Vec2>, Unreached>>()?,
            ),
            Class::None => Class::None,
        })
    })();
    match result {
        Ok(c) => Projected {
            shape: keep_heights(s, shape_of(&c)),
            error: None,
            chorded,
        },
        Err(why) => Projected {
            shape: None,
            error: Some(why),
            chorded,
        },
    }
}

/// A point's height stays with it (a path's and an area's vertices are written anew).
fn keep_heights(s: &Shape, out: Option<Shape>) -> Option<Shape> {
    match (s, out) {
        (
            Shape::Point { z, parts, .. },
            Some(Shape::Point {
                p, parts: moved, ..
            }),
        ) => Some(Shape::Point {
            p,
            z: *z,
            parts: moved.map(|m| {
                m.into_iter()
                    .zip(parts.iter().flatten())
                    .map(|(q, old)| crate::entity::PointPart { p: q.p, z: old.z })
                    .collect()
            }),
        }),
        (_, out) => out,
    }
}
