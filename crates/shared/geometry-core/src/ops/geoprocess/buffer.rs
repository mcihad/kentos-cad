//! Tampon (docs/adr/0201 §2): the points within a distance of an object,
//! built of exact pieces joined by the core's overlay: a point's disc, a
//! segment's capsule (two sides and two half circles), an arc's band with a
//! disc at each end; an area with them all around its boundary, or less
//! them inward. A one-sided buffer of a path is its strips on that side,
//! rectangles and arc bands, with a disc's wedge where the path turns away
//! from that side; its ends are flat.

use super::{Class, subtract_all, union_all};
use crate::geom::arrangement::{Area, Ring, TOL};
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::{Edge, point_at};
use crate::jsmath::{atan2, cos, js_hypot, js_max, sin};
use crate::vec2::Vec2;

/// Which side of a path a buffer takes (paths only; areas and points take both).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Both,
    Left,
    Right,
}

impl Side {
    pub fn from_key(key: &str) -> Option<Side> {
        match key {
            "both" => Some(Side::Both),
            "left" => Some(Side::Left),
            "right" => Some(Side::Right),
            _ => None,
        }
    }
}

fn piece(pts: Vec<Vec2>, bulges: Vec<f64>) -> Area {
    let curved = bulges.iter().any(|b| *b != 0.0);
    Area {
        outer: Ring {
            pts,
            bulges: curved.then_some(bulges),
        },
        holes: Vec::new(),
    }
}

fn at(c: Vec2, r: f64, angle: f64) -> Vec2 {
    Vec2::new(c.x + r * cos(angle), c.y + r * sin(angle))
}

/// The disc of radius `r` about `c`: two half circles.
pub fn disc(c: Vec2, r: f64) -> Area {
    piece(
        vec![Vec2::new(c.x + r, c.y), Vec2::new(c.x - r, c.y)],
        vec![1.0, 1.0],
    )
}

/// The points within `d` of the segment `a`–`b`: its two sides and a half circle about each end.
pub fn capsule(a: Vec2, b: Vec2, d: f64) -> Area {
    let len = js_hypot(b.x - a.x, b.y - a.y);
    if len <= TOL {
        return disc(a, d);
    }
    let (nx, ny) = (-(b.y - a.y) / len * d, (b.x - a.x) / len * d);
    piece(
        vec![
            Vec2::new(a.x - nx, a.y - ny),
            Vec2::new(b.x - nx, b.y - ny),
            Vec2::new(b.x + nx, b.y + ny),
            Vec2::new(a.x + nx, a.y + ny),
        ],
        vec![0.0, 1.0, 0.0, 1.0],
    )
}

/// The band between radii `lo` and `hi` about `c` over the arc's angles
/// (from `a0`, `sweep`); a pie when `lo` is at the centre.
fn band(c: Vec2, a0: f64, sweep: f64, lo: f64, hi: f64) -> Area {
    let a1 = a0 + sweep;
    let bulge = bulge_of_sweep(sweep);
    if lo <= TOL {
        return piece(vec![c, at(c, hi, a0), at(c, hi, a1)], vec![0.0, bulge, 0.0]);
    }
    piece(
        vec![at(c, hi, a0), at(c, hi, a1), at(c, lo, a1), at(c, lo, a0)],
        vec![bulge, 0.0, -bulge, 0.0],
    )
}

/// The pieces within `d` of an edge: a capsule, or an arc's band (a pie
/// when `d` reaches its centre) and a disc at each end.
pub fn edge_pieces(e: &Edge, d: f64, out: &mut Vec<Area>) {
    match *e {
        Edge::Seg { a, b } => out.push(capsule(a, b, d)),
        Edge::Arc { c, r, a0, sweep } => {
            out.push(disc(point_at(e, 0.0), d));
            out.push(disc(point_at(e, 1.0), d));
            out.push(band(c, a0, sweep, r - d, r + d));
        }
    }
}

/// The edge's unit direction at its start (`t` 0) or end (`t` 1).
fn tangent(e: &Edge, t: f64) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => {
            let len = js_max(js_hypot(b.x - a.x, b.y - a.y), f64::MIN_POSITIVE);
            Vec2::new((b.x - a.x) / len, (b.y - a.y) / len)
        }
        Edge::Arc { a0, sweep, .. } => {
            let a = a0 + sweep * t;
            let s = if sweep < 0.0 { -1.0 } else { 1.0 };
            Vec2::new(-sin(a) * s, cos(a) * s)
        }
    }
}

/// The strip on one side of an edge out to `d`: a rectangle, or for an arc
/// the band between it and its parallel (towards its centre when the side
/// is the inside of its turn).
fn strip(e: &Edge, d: f64, left: bool) -> Area {
    match *e {
        Edge::Seg { a, b } => {
            let len = js_max(js_hypot(b.x - a.x, b.y - a.y), f64::MIN_POSITIVE);
            let s = if left { d } else { -d };
            let (nx, ny) = (-(b.y - a.y) / len * s, (b.x - a.x) / len * s);
            piece(
                vec![
                    a,
                    b,
                    Vec2::new(b.x + nx, b.y + ny),
                    Vec2::new(a.x + nx, a.y + ny),
                ],
                vec![0.0; 4],
            )
        }
        Edge::Arc { c, r, a0, sweep } => {
            // Running counter-clockwise, the left is the centre's side.
            let inward = (sweep > 0.0) == left;
            if inward {
                band(c, a0, sweep, r - d, r)
            } else {
                band(c, a0, sweep, r, r + d)
            }
        }
    }
}

/// Where a path turns away from the side (its outside there), the disc's
/// wedge between the two edges' normals at the vertex.
fn wedge(v: Vec2, t1: Vec2, t2: Vec2, d: f64, left: bool) -> Option<Area> {
    let cross = t1.x * t2.y - t1.y * t2.x;
    let dot = t1.x * t2.x + t1.y * t2.y;
    // Left: a right turn opens the left side; right: a left turn the right.
    let opens = if left { cross < 0.0 } else { cross > 0.0 };
    if !opens {
        return None;
    }
    let turn = atan2(cross, dot);
    let s = if left { 1.0 } else { -1.0 };
    let n1 = Vec2::new(-t1.y * s * d, t1.x * s * d);
    let n2 = Vec2::new(-t2.y * s * d, t2.x * s * d);
    Some(piece(
        vec![
            v,
            Vec2::new(v.x + n1.x, v.y + n1.y),
            Vec2::new(v.x + n2.x, v.y + n2.y),
        ],
        vec![0.0, bulge_of_sweep(turn), 0.0],
    ))
}

/// A path's one-sided pieces: its edges' strips and the wedges where it turns away from the side.
fn side_pieces(path: &[Edge], d: f64, left: bool, out: &mut Vec<Area>) {
    for (i, e) in path.iter().enumerate() {
        out.push(strip(e, d, left));
        let next = match path.get(i + 1) {
            Some(n) => Some(n),
            // A path that closes turns at its first vertex too.
            None if path.len() > 2 => {
                let (s, f) = (point_at(&path[0], 0.0), point_at(e, 1.0));
                (js_hypot(s.x - f.x, s.y - f.y) <= TOL).then(|| &path[0])
            }
            None => None,
        };
        if let Some(n) = next
            && let Some(w) = wedge(point_at(e, 1.0), tangent(e, 1.0), tangent(n, 0.0), d, left)
        {
            out.push(w);
        }
    }
}

/// The buffer of an object at distance `d` (m): for areas a minus distance
/// goes inward; `side` takes one side of a path. Empty when nothing is left
/// (an area buffered inward past its middle, a path or a point at a minus
/// distance).
pub fn buffer(c: &Class, d: f64, side: Side) -> Vec<Area> {
    if d == 0.0 || !d.is_finite() {
        return match c {
            Class::Areas(a) => union_all(a),
            _ => Vec::new(),
        };
    }
    if d < 0.0 && !matches!(c, Class::Areas(_)) {
        return Vec::new();
    }
    let mut pieces = Vec::new();
    match c {
        Class::Points(points) => {
            for &p in points {
                pieces.push(disc(p, d.abs()));
            }
            union_all(&pieces)
        }
        Class::Paths(paths) => {
            for path in paths {
                match side {
                    Side::Both => path
                        .iter()
                        .for_each(|e| edge_pieces(e, d.abs(), &mut pieces)),
                    Side::Left | Side::Right => {
                        side_pieces(path, d.abs(), side == Side::Left, &mut pieces)
                    }
                }
            }
            union_all(&pieces)
        }
        Class::Areas(areas) => {
            for e in super::boundary_edges(areas) {
                edge_pieces(&e, d.abs(), &mut pieces);
            }
            if d > 0.0 {
                pieces.extend(areas.iter().cloned());
                union_all(&pieces)
            } else {
                subtract_all(areas, &pieces)
            }
        }
        Class::None => Vec::new(),
    }
}
