//! Sınırlayıcı geometri (docs/adr/0219 §4): a set of points' convex hull,
//! its box along the axes, its rotated box of least area and its least
//! enclosing circle.

use std::f64::consts::PI;

use super::voronoi::{centre, normalised};
use crate::jsmath::{atan2, js_hypot, js_max, js_min};
use crate::predicates::orient2d;
use crate::vec2::Vec2;

/// Which bounding geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hull,
    Box,
    Rotated,
    Circle,
}

impl Kind {
    pub fn from_key(key: &str) -> Option<Kind> {
        match key {
            "hull" => Some(Kind::Hull),
            "box" => Some(Kind::Box),
            "rotated" => Some(Kind::Rotated),
            "circle" => Some(Kind::Circle),
            _ => None,
        }
    }
}

/// A bounding geometry and its measures.
#[derive(Clone, Debug, PartialEq)]
pub enum Bound {
    /// The hull's ring and area.
    Hull { ring: Vec<Vec2>, area: f64 },
    /// The box's ring, its width (east) and height (north).
    Box {
        ring: Vec<Vec2>,
        width: f64,
        height: f64,
    },
    /// The rotated box's ring, its longer and shorter sides and its longer side's bearing (radians clockwise from
    /// north, below a half turn).
    Rotated {
        ring: Vec<Vec2>,
        length: f64,
        width: f64,
        bearing: f64,
    },
    /// The circle.
    Circle { c: Vec2, r: f64 },
}

/// The convex hull, counter-clockwise from its lowest point (the westmost of
/// equal ones), without points on its edges (Andrew's monotone chain over
/// (y, x), the turns exact). Fewer than three points off one line: the
/// distinct points along it (none, one, or the two ends).
pub fn convex_hull(points: &[Vec2]) -> Vec<Vec2> {
    let mut p: Vec<Vec2> = points
        .iter()
        .filter(|q| q.x.is_finite() && q.y.is_finite())
        .map(|q| Vec2::new(q.x + 0.0, q.y + 0.0))
        .collect();
    p.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let chain = |seq: &mut dyn Iterator<Item = Vec2>| {
        let mut out: Vec<Vec2> = Vec::new();
        for q in seq {
            while out.len() >= 2 && orient2d(out[out.len() - 2], out[out.len() - 1], q) <= 0.0 {
                out.pop();
            }
            out.push(q);
        }
        out
    };
    let mut lower = chain(&mut p.iter().copied());
    let mut upper = chain(&mut p.iter().rev().copied());
    lower.pop();
    upper.pop();
    lower.extend(upper);
    if lower.len() < 3 {
        return vec![p[0], p[p.len() - 1]];
    }
    lower
}

/// Twice a ring's signed area, about its first corner.
fn doubled_area(ring: &[Vec2]) -> f64 {
    let Some(&o) = ring.first() else {
        return 0.0;
    };
    let mut s = 0.0;
    for i in 1..ring.len().saturating_sub(1) {
        let (a, b) = (ring[i], ring[i + 1]);
        s += (a.x - o.x) * (b.y - o.y) - (b.x - o.x) * (a.y - o.y);
    }
    s
}

/// The box along the axes: none when it has no width or no height.
pub fn axis_box(points: &[Vec2]) -> Option<Bound> {
    let mut it = points.iter().filter(|q| q.x.is_finite() && q.y.is_finite());
    let first = it.next()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
    for p in it {
        x0 = js_min(x0, p.x);
        y0 = js_min(y0, p.y);
        x1 = js_max(x1, p.x);
        y1 = js_max(y1, p.y);
    }
    (x1 > x0 && y1 > y0).then(|| Bound::Box {
        ring: vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ],
        width: x1 - x0,
        height: y1 - y0,
    })
}

/// A bearing (radians clockwise from north) folded below a half turn.
fn half_bearing(u: Vec2) -> f64 {
    let mut t = atan2(u.x, u.y);
    if t < 0.0 {
        t += PI;
    }
    if t >= PI {
        t -= PI;
    }
    t
}

/// The rotated box of least area round a convex hull (counter-clockwise, at
/// least three points): one side on a hull edge (Freeman and Shapira), the
/// extremes followed round the hull by rotating calipers; of equal areas the
/// first edge's.
pub fn min_area_box(hull: &[Vec2]) -> Option<Bound> {
    let n = hull.len();
    if n < 3 {
        return None;
    }
    // About the first corner: small numbers.
    let o = hull[0];
    let p: Vec<Vec2> = hull
        .iter()
        .map(|q| Vec2::new(q.x - o.x, q.y - o.y))
        .collect();
    let along = |u: Vec2, q: Vec2| q.x * u.x + q.y * u.y;
    let across = |u: Vec2, q: Vec2| q.y * u.x - q.x * u.y;
    let unit = |i: usize| {
        let (a, b) = (p[i], p[(i + 1) % n]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l = js_hypot(dx, dy);
        Vec2::new(dx / l, dy / l)
    };
    // The extremes for the first edge by a scan, then each moved forward with the edge.
    let u = unit(0);
    let (mut hi, mut lo, mut top) = (0usize, 0usize, 0usize);
    for k in 1..n {
        if along(u, p[k]) > along(u, p[hi]) {
            hi = k;
        }
        if along(u, p[k]) < along(u, p[lo]) {
            lo = k;
        }
        if across(u, p[k]) > across(u, p[top]) {
            top = k;
        }
    }
    let mut best: Option<(f64, Bound)> = None;
    for i in 0..n {
        let u = unit(i);
        if !(u.x.is_finite() && u.y.is_finite()) {
            continue;
        }
        for _ in 0..n {
            let k = (hi + 1) % n;
            if along(u, p[k]) >= along(u, p[hi]) && k != i {
                hi = k;
            } else {
                break;
            }
        }
        for _ in 0..n {
            let k = (lo + 1) % n;
            if along(u, p[k]) <= along(u, p[lo]) && k != (i + 1) % n {
                lo = k;
            } else {
                break;
            }
        }
        for _ in 0..n {
            let k = (top + 1) % n;
            if across(u, p[k]) >= across(u, p[top]) {
                top = k;
            } else {
                break;
            }
        }
        let base = p[i];
        let (s0, s1) = (
            along(u, p[lo]) - along(u, base),
            along(u, p[hi]) - along(u, base),
        );
        let t1 = across(u, p[top]) - across(u, base);
        let area = (s1 - s0) * t1;
        if best.as_ref().is_some_and(|(b, _)| area >= *b) {
            continue;
        }
        let w = Vec2::new(-u.y, u.x);
        let at = |s: f64, t: f64| {
            Vec2::new(
                o.x + base.x + s * u.x + t * w.x,
                o.y + base.y + s * u.y + t * w.y,
            )
        };
        let ring = normalised(vec![at(s0, 0.0), at(s1, 0.0), at(s1, t1), at(s0, t1)]);
        let (length, width, long) = if s1 - s0 >= t1 {
            (s1 - s0, t1, u)
        } else {
            (t1, s1 - s0, w)
        };
        best = Some((
            area,
            Bound::Rotated {
                ring,
                length,
                width,
                bearing: half_bearing(long),
            },
        ));
    }
    best.map(|(_, b)| b)
}

/// A circle: its centre and radius.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Disc {
    c: Vec2,
    r: f64,
}

fn holds(d: &Disc, p: Vec2) -> bool {
    // A hair of slack for the rounding of the circle's own centre.
    js_hypot(p.x - d.c.x, p.y - d.c.y) <= d.r * (1.0 + 1e-12)
}

fn two(a: Vec2, b: Vec2) -> Disc {
    let c = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    Disc {
        c,
        r: js_max(
            js_hypot(a.x - c.x, a.y - c.y),
            js_hypot(b.x - c.x, b.y - c.y),
        ),
    }
}

fn three(a: Vec2, b: Vec2, c: Vec2) -> Option<Disc> {
    let turn = orient2d(a, b, c);
    if turn == 0.0 {
        return None;
    }
    let o = if turn > 0.0 {
        centre(a, b, c)
    } else {
        centre(a, c, b)
    };
    let r = js_max(
        js_hypot(a.x - o.x, a.y - o.y),
        js_max(
            js_hypot(b.x - o.x, b.y - o.y),
            js_hypot(c.x - o.x, c.y - o.y),
        ),
    );
    (o.x.is_finite() && o.y.is_finite()).then_some(Disc { c: o, r })
}

/// The least circle round the points (Welzl's algorithm without recursion,
/// the points in an order shuffled by a fixed seed, so every target finds the
/// same circle; worked out about the points' first one). None without points.
pub fn min_circle(points: &[Vec2]) -> Option<(Vec2, f64)> {
    let o = *points.first()?;
    let mut p: Vec<Vec2> = points
        .iter()
        .map(|q| Vec2::new(q.x - o.x, q.y - o.y))
        .collect();
    // Fisher–Yates with a fixed linear congruential sequence.
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    for i in (1..p.len()).rev() {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = ((seed >> 33) % (i as u64 + 1)) as usize;
        p.swap(i, j);
    }
    let mut d = Disc { c: p[0], r: 0.0 };
    for i in 1..p.len() {
        if holds(&d, p[i]) {
            continue;
        }
        d = Disc { c: p[i], r: 0.0 };
        for j in 0..i {
            if holds(&d, p[j]) {
                continue;
            }
            d = two(p[i], p[j]);
            for k in 0..j {
                if holds(&d, p[k]) {
                    continue;
                }
                if let Some(t) = three(p[i], p[j], p[k]) {
                    d = t;
                }
            }
        }
    }
    Some((Vec2::new(o.x + d.c.x, o.y + d.c.y), d.r))
}

/// A set of points' bounding geometry of the kind; none when it has no area
/// (one point, points on one line, a circle of no radius).
pub fn bound(points: &[Vec2], kind: Kind) -> Option<Bound> {
    match kind {
        Kind::Hull => {
            let ring = convex_hull(points);
            let area = doubled_area(&ring) / 2.0;
            (ring.len() >= 3 && area > 0.0).then_some(Bound::Hull { ring, area })
        }
        Kind::Box => axis_box(points),
        Kind::Rotated => min_area_box(&convex_hull(points)),
        Kind::Circle => min_circle(points)
            .filter(|(_, r)| *r > 0.0)
            .map(|(c, r)| Bound::Circle { c, r }),
    }
}
