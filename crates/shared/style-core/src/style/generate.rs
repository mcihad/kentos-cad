//! Geometri üretici (docs/adr/0226 §1): an object's geometry through a
//! generator layer's steps: centres, vertices, the boundary, parallels and
//! extensions, buffers, the envelope and the hull, circles, slices and
//! regular polygons, moves, turns and scales; their numbers per object.
//! Rings come out as the compiler takes them: outer counter-clockwise,
//! holes clockwise, the first point not repeated. Arcs are cut into chords
//! that stay within 0.05 paper mm of them. The independent reference is
//! scripts/fixtures/geometry_generator_cases.py.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::offset::offset_path;
use kentos_geometry_core::jsmath::{
    PI, TAU, acos, atan, atan2, cos, js_hypot, js_max, js_min, js_round, sin,
};
use kentos_geometry_core::ops::generate::bounding::convex_hull;
use kentos_geometry_core::ops::geoprocess::Class;
use kentos_geometry_core::ops::geoprocess::buffer::{Side, buffer};

use super::compile::{Env, Geom, Values, dd_number, to_world};
use super::model::{Dd, Step, Unit};
use super::place::{interior_point, line_middle};

const DEG: f64 = PI / 180.0;

/// The largest distance a chord keeps from its arc: 0.05 paper mm.
const CHORD_MM: f64 = 0.05;

/// A geometry between steps: points, paths (and whether each is closed), or
/// areas (each part's rings).
#[derive(Clone, Debug, PartialEq)]
pub enum Gen {
    Points(Vec<Vec2>),
    Lines(Vec<(Vec<Vec2>, bool)>),
    Areas(Vec<Vec<Vec<Vec2>>>),
}

impl Gen {
    /// The compiler's geometry as a generator takes it.
    pub fn of(g: &Geom) -> Gen {
        match g {
            Geom::Marker(p) => Gen::Points(vec![*p]),
            Geom::Line(paths) => Gen::Lines(paths.clone()),
            Geom::Fill(rings) => Gen::Areas(vec![rings.clone()]),
            Geom::Fills(parts) => Gen::Areas(parts.clone()),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Gen::Points(p) => p.is_empty(),
            Gen::Lines(p) => p.is_empty(),
            Gen::Areas(p) => p.is_empty(),
        }
    }

    /// Every coordinate, outer rings and holes too.
    fn coords(&self) -> Vec<Vec2> {
        match self {
            Gen::Points(p) => p.clone(),
            Gen::Lines(paths) => paths.iter().flat_map(|(p, _)| p.iter().copied()).collect(),
            Gen::Areas(parts) => parts.iter().flatten().flatten().copied().collect(),
        }
    }

    fn map(self, f: impl Fn(Vec2) -> Vec2) -> Gen {
        match self {
            Gen::Points(p) => Gen::Points(p.into_iter().map(&f).collect()),
            Gen::Lines(paths) => Gen::Lines(
                paths
                    .into_iter()
                    .map(|(p, c)| (p.into_iter().map(&f).collect(), c))
                    .collect(),
            ),
            Gen::Areas(parts) => Gen::Areas(
                parts
                    .into_iter()
                    .map(|rings| {
                        rings
                            .into_iter()
                            .map(|r| r.into_iter().map(&f).collect())
                            .collect()
                    })
                    .collect(),
            ),
        }
    }
}

/// The geometry after `steps`; empty when a step leaves nothing.
pub fn run(steps: &[Step], g: Gen, t: &dyn Values, env: &Env, unit: Unit) -> Gen {
    let mut g = g;
    for s in steps {
        g = step(s, g, t, env, unit);
        if g.is_empty() {
            break;
        }
    }
    g
}

/// A data-defined number, None when it is empty or not finite.
fn value(v: &Option<Dd<f64>>, t: &dyn Values) -> Option<f64> {
    let x = dd_number(v, t, f64::NAN);
    x.is_finite().then_some(x)
}

/// A step's length in metres; None when it is empty or not finite.
fn length(v: &Option<Dd<f64>>, t: &dyn Values, env: &Env, unit: Unit) -> Option<f64> {
    value(v, t)
        .map(|x| to_world(x, unit, env))
        .filter(|x| x.is_finite())
}

fn step(s: &Step, g: Gen, t: &dyn Values, env: &Env, unit: Unit) -> Gen {
    let tol = CHORD_MM * env.plot_scale / 1000.0;
    let len = |v: &Option<Dd<f64>>| length(v, t, env, unit);
    match s {
        Step::Centroid => Gen::Points(centroid(&g).into_iter().collect()),
        Step::PointOnSurface => match &g {
            Gen::Points(_) => g,
            Gen::Lines(paths) => Gen::Points(line_middle(paths).into_iter().collect()),
            Gen::Areas(parts) => {
                let largest = parts
                    .iter()
                    .max_by(|a, b| part_area(a).total_cmp(&part_area(b)));
                Gen::Points(
                    largest
                        .and_then(|r| interior_point(r))
                        .into_iter()
                        .collect(),
                )
            }
        },
        Step::Vertices(which) => vertices(&g, which),
        Step::Boundary => match g {
            Gen::Areas(parts) => {
                Gen::Lines(parts.into_iter().flatten().map(|r| (r, true)).collect())
            }
            _ => Gen::Lines(Vec::new()),
        },
        Step::Offset { distance } => {
            let Some(d) = len(distance) else {
                return Gen::Lines(Vec::new());
            };
            let paths: Vec<(Vec<Vec2>, bool)> = match g {
                Gen::Lines(paths) => paths,
                Gen::Areas(parts) => parts.into_iter().flatten().map(|r| (r, true)).collect(),
                Gen::Points(_) => Vec::new(),
            };
            Gen::Lines(
                paths
                    .into_iter()
                    .map(|(p, closed)| {
                        if d == 0.0 {
                            (p, closed)
                        } else {
                            (offset_path(&p, d, closed), closed)
                        }
                    })
                    .filter(|(p, _)| p.len() >= 2)
                    .collect(),
            )
        }
        Step::Extend { start, end } => match g {
            Gen::Lines(paths) => {
                let a = js_max(len(start).unwrap_or(0.0), 0.0);
                let b = js_max(len(end).unwrap_or(0.0), 0.0);
                Gen::Lines(
                    paths
                        .into_iter()
                        .map(|(p, closed)| {
                            if closed {
                                (p, true)
                            } else {
                                (extend(p, a, b), false)
                            }
                        })
                        .collect(),
                )
            }
            _ => Gen::Lines(Vec::new()),
        },
        Step::Buffer { distance, side } => {
            let Some(d) = len(distance) else {
                return Gen::Areas(Vec::new());
            };
            buffered(&g, d, side, tol)
        }
        Step::Envelope => {
            let c = g.coords();
            let (mut x0, mut y0, mut x1, mut y1) = (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            );
            for p in &c {
                x0 = js_min(x0, p.x);
                y0 = js_min(y0, p.y);
                x1 = js_max(x1, p.x);
                y1 = js_max(y1, p.y);
            }
            if !(x1 > x0 && y1 > y0) {
                return Gen::Areas(Vec::new());
            }
            Gen::Areas(vec![vec![vec![
                Vec2::new(x0, y0),
                Vec2::new(x1, y0),
                Vec2::new(x1, y1),
                Vec2::new(x0, y1),
            ]]])
        }
        Step::Hull => {
            let h = convex_hull(&g.coords());
            if h.len() < 3 {
                return Gen::Areas(Vec::new());
            }
            Gen::Areas(vec![vec![h]])
        }
        Step::Circle { radius } => {
            let Some(r) = len(radius).filter(|r| *r > 0.0) else {
                return Gen::Areas(Vec::new());
            };
            Gen::Areas(
                centres(&g)
                    .into_iter()
                    .map(|c| vec![circle(c, r, tol)])
                    .collect(),
            )
        }
        Step::Wedge {
            direction,
            width,
            radius,
            inner,
        } => {
            let (Some(dir), Some(w), Some(r)) = (value(direction, t), value(width, t), len(radius))
            else {
                return Gen::Areas(Vec::new());
            };
            let ri = js_max(len(inner).unwrap_or(0.0), 0.0);
            if w <= 0.0 || r <= 0.0 || ri >= r {
                return Gen::Areas(Vec::new());
            }
            Gen::Areas(
                centres(&g)
                    .into_iter()
                    .map(|c| wedge(c, dir, w, r, ri, tol))
                    .collect(),
            )
        }
        Step::Polygon {
            sides,
            radius,
            rotation,
        } => {
            let (Some(n), Some(r)) = (value(sides, t).map(js_round), len(radius)) else {
                return Gen::Areas(Vec::new());
            };
            if n < 3.0 || r <= 0.0 {
                return Gen::Areas(Vec::new());
            }
            let turn = value(rotation, t).unwrap_or(0.0);
            let n = js_min(n, 720.0) as usize;
            Gen::Areas(
                centres(&g)
                    .into_iter()
                    .map(|c| {
                        vec![
                            (0..n)
                                .map(|k| {
                                    let a = (90.0 + turn) * DEG + TAU * k as f64 / n as f64;
                                    Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
                                })
                                .collect(),
                        ]
                    })
                    .collect(),
            )
        }
        Step::Translate { dx, dy } => {
            let (x, y) = (len(dx).unwrap_or(0.0), len(dy).unwrap_or(0.0));
            g.map(|p| Vec2::new(p.x + x, p.y + y))
        }
        Step::Rotate { angle } => {
            let (Some(a), Some(c)) = (value(angle, t), centroid(&g)) else {
                return g;
            };
            let (ca, sa) = (cos(a * DEG), sin(a * DEG));
            g.map(|p| {
                let (x, y) = (p.x - c.x, p.y - c.y);
                Vec2::new(c.x + x * ca - y * sa, c.y + x * sa + y * ca)
            })
        }
        Step::Scale { x, y } => {
            let (vx, vy) = (value(x, t), value(y, t));
            let fx = vx.or(vy).unwrap_or(1.0);
            let fy = vy.or(vx).unwrap_or(1.0);
            let Some(c) = centroid(&g) else {
                return g;
            };
            let out = g.map(|p| Vec2::new(c.x + (p.x - c.x) * fx, c.y + (p.y - c.y) * fy));
            // A mirroring scale turns the rings: they are turned back (outer counter-clockwise).
            match out {
                Gen::Areas(parts) if fx * fy < 0.0 => Gen::Areas(
                    parts
                        .into_iter()
                        .map(|rings| {
                            rings
                                .into_iter()
                                .map(|mut r| {
                                    r.reverse();
                                    r
                                })
                                .collect()
                        })
                        .collect(),
                ),
                other => other,
            }
        }
        Step::Unknown => Gen::Points(Vec::new()),
    }
}

/// Twice a ring's signed area, from its first point (no cancellation far from the origin).
fn ring_area2(r: &[Vec2]) -> f64 {
    let Some(&o) = r.first() else {
        return 0.0;
    };
    let mut s = 0.0;
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        s += (a.x - o.x) * (b.y - o.y) - (b.x - o.x) * (a.y - o.y);
    }
    s
}

fn part_area(rings: &[Vec<Vec2>]) -> f64 {
    rings.iter().map(|r| ring_area2(r)).sum::<f64>() / 2.0
}

/// The centroid (docs/adr/0226 §1): of areas their rings' (holes minus),
/// of paths their edges' by length, of points their mean; None when there
/// is nothing.
pub fn centroid(g: &Gen) -> Option<Vec2> {
    let coords = g.coords();
    let o = *coords.first()?;
    match g {
        Gen::Points(p) => {
            let n = p.len() as f64;
            let (sx, sy) = p
                .iter()
                .fold((0.0, 0.0), |(x, y), q| (x + (q.x - o.x), y + (q.y - o.y)));
            Some(Vec2::new(o.x + sx / n, o.y + sy / n))
        }
        Gen::Lines(paths) => {
            let (mut sx, mut sy, mut total) = (0.0, 0.0, 0.0);
            for (p, closed) in paths {
                let n = p.len();
                let count = if *closed { n } else { n.saturating_sub(1) };
                for i in 0..count {
                    let (a, b) = (p[i], p[(i + 1) % n]);
                    let l = js_hypot(b.x - a.x, b.y - a.y);
                    sx += l * ((a.x - o.x) + (b.x - o.x)) / 2.0;
                    sy += l * ((a.y - o.y) + (b.y - o.y)) / 2.0;
                    total += l;
                }
            }
            if total > 0.0 {
                Some(Vec2::new(o.x + sx / total, o.y + sy / total))
            } else {
                Some(o)
            }
        }
        Gen::Areas(parts) => {
            let (mut sx, mut sy, mut a2) = (0.0, 0.0, 0.0);
            for r in parts.iter().flatten() {
                for i in 0..r.len() {
                    let (p, q) = (r[i], r[(i + 1) % r.len()]);
                    let (px, py, qx, qy) = (p.x - o.x, p.y - o.y, q.x - o.x, q.y - o.y);
                    let cross = px * qy - qx * py;
                    sx += (px + qx) * cross;
                    sy += (py + qy) * cross;
                    a2 += cross;
                }
            }
            if a2.abs() > 0.0 {
                Some(Vec2::new(o.x + sx / (3.0 * a2), o.y + sy / (3.0 * a2)))
            } else {
                // No area: the mean of the outer rings' points.
                let outer: Vec<Vec2> = parts
                    .iter()
                    .filter_map(|r| r.first())
                    .flatten()
                    .copied()
                    .collect();
                centroid(&Gen::Points(outer))
            }
        }
    }
}

/// Where a circle, a slice or a regular polygon is built: each point of
/// points, else the geometry's centroid.
fn centres(g: &Gen) -> Vec<Vec2> {
    match g {
        Gen::Points(p) => p.clone(),
        _ => centroid(g).into_iter().collect(),
    }
}

/// A path's or ring's points without a last one that repeats the first.
fn open_points(p: &[Vec2], closed: bool) -> &[Vec2] {
    match (closed, p.first(), p.last()) {
        (true, Some(a), Some(b)) if p.len() > 1 && a == b => &p[..p.len() - 1],
        _ => p,
    }
}

fn vertices(g: &Gen, which: &str) -> Gen {
    let paths: Vec<(&[Vec2], bool)> = match g {
        Gen::Lines(paths) => paths
            .iter()
            .map(|(p, c)| (open_points(p, *c), *c))
            .collect(),
        Gen::Areas(parts) => parts
            .iter()
            .flatten()
            .map(|r| (open_points(r, true), true))
            .collect(),
        Gen::Points(_) => Vec::new(),
    };
    let mut out = Vec::new();
    for (p, closed) in paths {
        let n = p.len();
        if n == 0 {
            continue;
        }
        match which {
            "inner" if closed => out.extend_from_slice(p),
            "inner" if n > 2 => out.extend_from_slice(&p[1..n - 1]),
            "inner" => {}
            "first" => out.push(p[0]),
            "last" => out.push(if closed { p[0] } else { p[n - 1] }),
            "ends" if closed || n == 1 => out.push(p[0]),
            "ends" => out.extend([p[0], p[n - 1]]),
            _ => out.extend_from_slice(p),
        }
    }
    Gen::Points(out)
}

/// An open path longer by `a` before its start and `b` past its end, along
/// its first and last edges that have a length.
fn extend(mut p: Vec<Vec2>, a: f64, b: f64) -> Vec<Vec2> {
    let n = p.len();
    if n < 2 {
        return p;
    }
    if a > 0.0
        && let Some(k) = (0..n - 1).find(|&k| p[k] != p[k + 1])
    {
        let (q, r) = (p[k], p[k + 1]);
        let l = js_hypot(r.x - q.x, r.y - q.y);
        p[0] = Vec2::new(p[0].x - (r.x - q.x) / l * a, p[0].y - (r.y - q.y) / l * a);
    }
    if b > 0.0
        && let Some(k) = (1..n).rev().find(|&k| p[k] != p[k - 1])
    {
        let (q, r) = (p[k - 1], p[k]);
        let l = js_hypot(r.x - q.x, r.y - q.y);
        p[n - 1] = Vec2::new(
            p[n - 1].x + (r.x - q.x) / l * b,
            p[n - 1].y + (r.y - q.y) / l * b,
        );
    }
    p
}

/// Chords for an arc of `sweep` radians and radius `r` (docs/adr/0226 §1).
pub fn chords(sweep: f64, r: f64, tol: f64) -> usize {
    let sweep = sweep.abs();
    if r.is_nan() || r <= tol || !sweep.is_finite() || sweep == 0.0 {
        return 1;
    }
    let most = js_max((720.0 * sweep / TAU).ceil(), 1.0);
    let n = (sweep / (2.0 * acos(1.0 - tol / r))).ceil();
    n.clamp(1.0, most) as usize
}

/// A circle's ring, counter-clockwise from east, at least 8 chords.
fn circle(c: Vec2, r: f64, tol: f64) -> Vec<Vec2> {
    let n = chords(TAU, r, tol).max(8);
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
        })
        .collect()
}

/// A slice of `width` degrees about `dir` (degrees clockwise from north)
/// between radii `ri` and `r`: one area (an annulus when it goes round).
fn wedge(c: Vec2, dir: f64, width: f64, r: f64, ri: f64, tol: f64) -> Vec<Vec<Vec2>> {
    if width >= 360.0 {
        let mut rings = vec![circle(c, r, tol)];
        if ri > 0.0 {
            let mut hole = circle(c, ri, tol);
            hole.reverse();
            rings.push(hole);
        }
        return rings;
    }
    let sweep = width * DEG;
    let a0 = (90.0 - dir) * DEG - sweep / 2.0;
    let n = chords(sweep, r, tol);
    let mut ring: Vec<Vec2> = (0..=n)
        .map(|k| {
            let a = a0 + sweep * k as f64 / n as f64;
            Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
        })
        .collect();
    if ri > 0.0 {
        let m = chords(sweep, ri, tol);
        ring.extend((0..=m).map(|k| {
            let a = a0 + sweep - sweep * k as f64 / m as f64;
            Vec2::new(c.x + ri * cos(a), c.y + ri * sin(a))
        }));
    } else {
        ring.push(c);
    }
    vec![ring]
}

/// A ring with arc edges cut into chords (docs/adr/0226 §1).
fn chorded(ring: &Ring, tol: f64) -> Vec<Vec2> {
    let n = ring.pts.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (a, b) = (ring.pts[i], ring.pts[(i + 1) % n]);
        out.push(a);
        let bulge = ring
            .bulges
            .as_ref()
            .and_then(|bs| bs.get(i))
            .copied()
            .unwrap_or(0.0);
        if bulge == 0.0 || !bulge.is_finite() {
            continue;
        }
        let sweep = 4.0 * atan(bulge);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let chord = js_hypot(dx, dy);
        if chord == 0.0 {
            continue;
        }
        let r = chord / (2.0 * sin(sweep.abs() / 2.0));
        // The centre: the chord's middle and, across it, (1 − b²)/(4b) of the chord.
        let k = (1.0 - bulge * bulge) / (4.0 * bulge);
        let c = Vec2::new(a.x + dx / 2.0 - dy * k, a.y + dy / 2.0 + dx * k);
        let a0 = atan2(a.y - c.y, a.x - c.x);
        let m = chords(sweep, r, tol);
        for j in 1..m {
            let t = a0 + sweep * j as f64 / m as f64;
            out.push(Vec2::new(c.x + r * cos(t), c.y + r * sin(t)));
        }
    }
    out
}

/// The buffer (docs/adr/0226 §1): the core's exact buffer, its arcs chorded.
fn buffered(g: &Gen, d: f64, side: &str, tol: f64) -> Gen {
    let side = match (g, side) {
        (Gen::Lines(_), "left") => Side::Left,
        (Gen::Lines(_), "right") => Side::Right,
        _ => Side::Both,
    };
    let class = match g {
        Gen::Points(p) => Class::Points(p.clone()),
        Gen::Lines(paths) => Class::Paths(
            paths
                .iter()
                .map(|(p, closed)| {
                    let n = p.len();
                    let count = if *closed { n } else { n.saturating_sub(1) };
                    (0..count)
                        .map(|i| Edge::Seg {
                            a: p[i],
                            b: p[(i + 1) % n],
                        })
                        .filter(|e| matches!(e, Edge::Seg { a, b } if a != b))
                        .collect::<Vec<Edge>>()
                })
                .filter(|edges| !edges.is_empty())
                .collect(),
        ),
        Gen::Areas(parts) => Class::Areas(
            parts
                .iter()
                .filter(|rings| rings.first().is_some_and(|r| r.len() >= 3))
                .map(|rings| Area {
                    outer: Ring {
                        pts: rings[0].clone(),
                        bulges: None,
                    },
                    holes: rings[1..]
                        .iter()
                        .filter(|r| r.len() >= 3)
                        .map(|r| Ring {
                            pts: r.clone(),
                            bulges: None,
                        })
                        .collect(),
                })
                .collect(),
        ),
    };
    Gen::Areas(
        buffer(&class, d, side)
            .iter()
            .map(|a| {
                std::iter::once(chorded(&a.outer, tol))
                    .chain(a.holes.iter().map(|h| chorded(h, tol)))
                    .filter(|r| r.len() >= 3)
                    .collect::<Vec<_>>()
            })
            .filter(|rings: &Vec<Vec<Vec2>>| !rings.is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_keep_within_the_tolerance() {
        // A 10 m circle at 1:1000 (0.05 m): n = ceil(2π / (2·acos(1 − 0.005))).
        let n = chords(TAU, 10.0, 0.05);
        assert_eq!(n, (TAU / (2.0 * acos(1.0 - 0.005))).ceil() as usize);
        assert_eq!(chords(TAU, 0.01, 0.05), 1);
        assert_eq!(chords(TAU, 1e9, 1e-9), 720);
    }

    #[test]
    fn a_square_s_centroid_and_vertices() {
        let sq = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(4.0, 0.0),
            Vec2::new(4.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        let g = Gen::Areas(vec![vec![sq.clone()]]);
        assert_eq!(centroid(&g), Some(Vec2::new(2.0, 1.0)));
        assert_eq!(
            vertices(&g, "first"),
            Gen::Points(vec![Vec2::new(0.0, 0.0)])
        );
        assert_eq!(vertices(&g, "all"), Gen::Points(sq));
    }
}
