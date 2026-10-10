//! The label engine's geometry on the page (docs/adr/0212 §3): pixels, the
//! window's lower left the origin, y up. A label's box turned by its angle
//! (`Obb`) and the tests it takes (another box, a segment, a circle, a
//! point), walking a path by its length, a path or an area cut to a
//! rectangle, the pole of inaccessibility (Mapbox's polylabel) and the
//! smallest rectangle round a shape. The independent reference is
//! `scripts/fixtures/label_engine_cases.py`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::jsmath::{atan2, cos, js_max, js_min, sin};
use crate::vec2::Vec2;

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// A vector's length, as the reference computes it (`math.sqrt` of the sum).
pub fn norm(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// A rectangle turned by an angle: its middle, the unit direction of its
/// length (`u`), half its length (`a`) and half its height (`b`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obb {
    pub c: Vec2,
    pub u: Vec2,
    pub a: f64,
    pub b: f64,
}

impl Obb {
    /// Turned `angle` radians counter-clockwise.
    pub fn new(c: Vec2, angle: f64, a: f64, b: f64) -> Obb {
        Obb {
            c,
            u: Vec2::new(cos(angle), sin(angle)),
            a,
            b,
        }
    }

    /// Along the unit direction `u` (`cos`, `sin` of its angle, as `new` makes it).
    pub fn along(c: Vec2, u: Vec2, a: f64, b: f64) -> Obb {
        Obb { c, u, a, b }
    }

    /// Level.
    pub fn level(c: Vec2, a: f64, b: f64) -> Obb {
        Obb {
            c,
            u: Vec2::new(1.0, 0.0),
            a,
            b,
        }
    }

    /// The unit direction of its height (`u` turned a quarter left).
    pub fn v(&self) -> Vec2 {
        Vec2::new(-self.u.y, self.u.x)
    }

    /// The same box `by` larger on every side.
    pub fn grown(&self, by: f64) -> Obb {
        Obb {
            a: self.a + by,
            b: self.b + by,
            ..*self
        }
    }

    /// Its corners: back bottom, front bottom, front top, back top.
    pub fn corners(&self) -> [Vec2; 4] {
        let (u, v) = (self.u, self.v());
        let at = |s: f64, t: f64| {
            Vec2::new(
                self.c.x + u.x * s * self.a + v.x * t * self.b,
                self.c.y + u.y * s * self.a + v.y * t * self.b,
            )
        };
        [at(-1.0, -1.0), at(1.0, -1.0), at(1.0, 1.0), at(-1.0, 1.0)]
    }

    /// Its level box: `[min_x, min_y, max_x, max_y]`.
    pub fn aabb(&self) -> [f64; 4] {
        let v = self.v();
        let hx = self.a * self.u.x.abs() + self.b * v.x.abs();
        let hy = self.a * self.u.y.abs() + self.b * v.y.abs();
        [self.c.x - hx, self.c.y - hy, self.c.x + hx, self.c.y + hy]
    }

    /// Half its extent along the unit direction `l`.
    fn radius(&self, l: Vec2) -> f64 {
        self.a * dot(self.u, l).abs() + self.b * dot(self.v(), l).abs()
    }

    /// Whether the two boxes share any area (the separating axis test on
    /// their four axes); boxes that only touch do not.
    pub fn overlaps(&self, o: &Obb) -> bool {
        let d = sub(o.c, self.c);
        for l in [self.u, self.v(), o.u, o.v()] {
            if dot(d, l).abs() >= self.radius(l) + o.radius(l) {
                return false;
            }
        }
        true
    }

    /// A point in its frame: along `u`, along `v`.
    fn local(&self, p: Vec2) -> Vec2 {
        let d = sub(p, self.c);
        Vec2::new(dot(d, self.u), dot(d, self.v()))
    }

    /// Whether `p` is inside (its edges not).
    pub fn contains(&self, p: Vec2) -> bool {
        let q = self.local(p);
        q.x.abs() < self.a && q.y.abs() < self.b
    }

    /// Whether the segment `p`–`q` passes through it (touching it does not).
    pub fn hits_segment(&self, p: Vec2, q: Vec2) -> bool {
        let (p, q) = (self.local(p), self.local(q));
        if js_max(p.x, q.x) <= -self.a || js_min(p.x, q.x) >= self.a {
            return false;
        }
        if js_max(p.y, q.y) <= -self.b || js_min(p.y, q.y) >= self.b {
            return false;
        }
        // The segment's normal: the box's corners all on one side of its line.
        let n = Vec2::new(p.y - q.y, q.x - p.x);
        let r = self.a * n.x.abs() + self.b * n.y.abs();
        dot(p, n).abs() < r
    }

    /// Whether a disc of radius `r` round `p` covers any of it.
    pub fn hits_circle(&self, p: Vec2, r: f64) -> bool {
        let q = self.local(p);
        let dx = js_max(q.x.abs() - self.a, 0.0);
        let dy = js_max(q.y.abs() - self.b, 0.0);
        dx * dx + dy * dy < r * r
    }
}

/// Whether `p` is inside the rings (even-odd: holes are rings too).
pub fn inside_rings(p: Vec2, rings: &[Vec<Vec2>]) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (ring[i], ring[j]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            j = i;
        }
    }
    inside
}

/// The distance from `p` to the segment `a`–`b`, squared.
pub fn segment_distance2(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut x, mut y) = (a.x, a.y);
    if dx != 0.0 || dy != 0.0 {
        let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy);
        if t > 1.0 {
            x = b.x;
            y = b.y;
        } else if t > 0.0 {
            x += dx * t;
            y += dy * t;
        }
    }
    let (ex, ey) = (p.x - x, p.y - y);
    ex * ex + ey * ey
}

/// The distance from `p` to the nearest edge of the rings, negative outside them: the distances and
/// the crossings (`inside_rings`'s) in one walk round the edges.
pub fn signed_distance(p: Vec2, rings: &[Vec<Vec2>]) -> f64 {
    let mut least = f64::INFINITY;
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        if n < 2 {
            continue;
        }
        let crossings = n >= 3;
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (ring[i], ring[j]);
            least = js_min(least, segment_distance2(p, a, b));
            if crossings
                && (a.y > p.y) != (b.y > p.y)
                && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x
            {
                inside = !inside;
            }
            j = i;
        }
    }
    let d = least.sqrt();
    if inside { d } else { -d }
}

/// Whether a box lies wholly inside the rings: its corners inside and no
/// edge of theirs through it.
pub fn box_inside(b: &Obb, rings: &[Vec<Vec2>]) -> bool {
    if !b.corners().iter().all(|&p| inside_rings(p, rings)) {
        return false;
    }
    for ring in rings {
        let n = ring.len();
        if n < 2 {
            continue;
        }
        let mut j = n - 1;
        for i in 0..n {
            if b.hits_segment(ring[j], ring[i]) {
                return false;
            }
            j = i;
        }
    }
    true
}

/// One side of the clipping rectangle: which points it keeps, where an edge crosses it, and where it is.
type ClipEdge = (fn(Vec2, f64) -> bool, fn(Vec2, Vec2, f64) -> Vec2, f64);

/// A ring cut to the rectangle `[x0, x1] × [y0, y1]` (Sutherland and
/// Hodgman's, edge by edge: left, right, bottom, top). An area whose rings
/// are cut so is the area cut so (even-odd); its edges along the rectangle
/// are edges of the cut ring.
pub fn clip_ring(ring: &[Vec2], r: [f64; 4]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = ring.to_vec();
    let edges: [ClipEdge; 4] = [
        (|p, x| p.x >= x, |a, b, x| cut_x(a, b, x), r[0]),
        (|p, x| p.x <= x, |a, b, x| cut_x(a, b, x), r[2]),
        (|p, y| p.y >= y, |a, b, y| cut_y(a, b, y), r[1]),
        (|p, y| p.y <= y, |a, b, y| cut_y(a, b, y), r[3]),
    ];
    for (keep, cut, at) in edges {
        let input = std::mem::take(&mut out);
        let n = input.len();
        if n == 0 {
            break;
        }
        let mut prev = input[n - 1];
        for &p in &input {
            let (inp, inq) = (keep(p, at), keep(prev, at));
            if inp {
                if !inq {
                    out.push(cut(prev, p, at));
                }
                out.push(p);
            } else if inq {
                out.push(cut(prev, p, at));
            }
            prev = p;
        }
    }
    out
}

fn cut_x(a: Vec2, b: Vec2, x: f64) -> Vec2 {
    let t = (x - a.x) / (b.x - a.x);
    Vec2::new(x, a.y + (b.y - a.y) * t)
}

fn cut_y(a: Vec2, b: Vec2, y: f64) -> Vec2 {
    let t = (y - a.y) / (b.y - a.y);
    Vec2::new(a.x + (b.x - a.x) * t, y)
}

/// The pieces of a path inside the rectangle `[x0, y0, x1, y1]` (each
/// segment cut by Liang and Barsky's rule), in the path's order.
pub fn clip_path(pts: &[Vec2], r: [f64; 4]) -> Vec<Vec<Vec2>> {
    let mut pieces: Vec<Vec<Vec2>> = Vec::new();
    let mut open = false;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut ok = true;
        for (p, q) in [
            (-dx, a.x - r[0]),
            (dx, r[2] - a.x),
            (-dy, a.y - r[1]),
            (dy, r[3] - a.y),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    ok = false;
                    break;
                }
            } else {
                let t = q / p;
                if p < 0.0 {
                    t0 = js_max(t0, t);
                } else {
                    t1 = js_min(t1, t);
                }
            }
        }
        if !ok || t0 > t1 {
            open = false;
            continue;
        }
        let s = Vec2::new(a.x + dx * t0, a.y + dy * t0);
        let e = Vec2::new(a.x + dx * t1, a.y + dy * t1);
        match pieces.last_mut() {
            Some(piece) if open && t0 <= 0.0 => piece.push(e),
            _ => pieces.push(vec![s, e]),
        }
        open = t1 >= 1.0;
    }
    pieces.retain(|p| p.len() >= 2);
    pieces
}

/// A path walked by its length.
#[derive(Clone, Debug)]
pub struct Walk {
    pub pts: Vec<Vec2>,
    /// The length up to each point.
    pub at: Vec<f64>,
}

impl Walk {
    /// The path's points with repeated ones dropped.
    pub fn new(pts: &[Vec2]) -> Walk {
        let mut kept: Vec<Vec2> = Vec::with_capacity(pts.len());
        let mut at = Vec::with_capacity(pts.len());
        let mut s = 0.0;
        for &p in pts {
            if let Some(&q) = kept.last() {
                let d = norm(p.x - q.x, p.y - q.y);
                if d == 0.0 {
                    continue;
                }
                s += d;
            }
            kept.push(p);
            at.push(s);
        }
        Walk { pts: kept, at }
    }

    pub fn length(&self) -> f64 {
        self.at.last().copied().unwrap_or(0.0)
    }

    /// The segment `s` falls on (the last one past the end).
    fn segment(&self, s: f64) -> usize {
        let n = self.pts.len();
        if n < 2 {
            return 0;
        }
        // The first point beyond s, then its segment.
        let i = self.at.partition_point(|&x| x <= s);
        i.clamp(1, n - 1) - 1
    }

    /// The point `s` along (clamped to the path's ends).
    pub fn point(&self, s: f64) -> Vec2 {
        let n = self.pts.len();
        if n == 0 {
            return Vec2::default();
        }
        if n == 1 {
            return self.pts[0];
        }
        let i = self.segment(s);
        let (a, b) = (self.pts[i], self.pts[i + 1]);
        let len = self.at[i + 1] - self.at[i];
        let t = ((s - self.at[i]) / len).clamp(0.0, 1.0);
        Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }

    /// The path's direction `s` along, radians.
    pub fn angle(&self, s: f64) -> f64 {
        self.angle_at(s)
    }

    fn angle_at(&self, s: f64) -> f64 {
        if self.pts.len() < 2 {
            return 0.0;
        }
        let i = self.segment(s);
        let (a, b) = (self.pts[i], self.pts[i + 1]);
        atan2(b.y - a.y, b.x - a.x)
    }

    /// The angle at `s` and its cosine and sine, for one who needs all three (a curved label's letter).
    pub fn turn(&self, s: f64) -> (f64, f64, f64) {
        let a = self.angle_at(s);
        (a, cos(a), sin(a))
    }

    /// The points strictly between `s0` and `s1` along.
    pub fn between(&self, s0: f64, s1: f64) -> impl Iterator<Item = Vec2> + '_ {
        self.pts
            .iter()
            .zip(&self.at)
            .filter(move |(_, s)| **s > s0 && **s < s1)
            .map(|(p, _)| *p)
    }

    /// The same path walked from its other end.
    pub fn reversed(&self) -> Walk {
        let total = self.length();
        Walk {
            pts: self.pts.iter().rev().copied().collect(),
            at: self.at.iter().rev().map(|s| total - s).collect(),
        }
    }
}

/// A cell of the pole's search: its middle, half its side, the distance from
/// its middle to the area's edge (negative outside) and the most any point of
/// it can have.
#[derive(Clone, Copy, Debug)]
struct Cell {
    c: Vec2,
    h: f64,
    d: f64,
    max: f64,
    /// When it was made: equal cells are taken first in, first out.
    seq: u64,
}

impl Cell {
    fn new(c: Vec2, h: f64, rings: &[Vec<Vec2>], seq: u64) -> Cell {
        let d = signed_distance(c, rings);
        Cell {
            c,
            h,
            d,
            max: d + h * std::f64::consts::SQRT_2,
            seq,
        }
    }
}

impl PartialEq for Cell {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}

impl Eq for Cell {}

impl PartialOrd for Cell {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Cell {
    /// The greater potential first, then the earlier made.
    fn cmp(&self, o: &Self) -> Ordering {
        self.max
            .total_cmp(&o.max)
            .then_with(|| o.seq.cmp(&self.seq))
    }
}

/// The point inside the rings farthest from their edges (the pole of
/// inaccessibility, Mapbox's polylabel) to within `precision`, and its
/// distance; the first ring is the outline. None for an outline of fewer
/// than three points or no area.
pub fn pole(rings: &[Vec<Vec2>], precision: f64) -> Option<(Vec2, f64)> {
    let outer = rings.first().filter(|r| r.len() >= 3)?;
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in outer {
        x0 = js_min(x0, p.x);
        y0 = js_min(y0, p.y);
        x1 = js_max(x1, p.x);
        y1 = js_max(y1, p.y);
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let size = js_min(w, h);
    if !(size > 0.0) {
        return None;
    }
    let mut seq = 0u64;
    let mut make = |c: Vec2, h: f64| {
        seq += 1;
        Cell::new(c, h, rings, seq)
    };
    let mut queue = BinaryHeap::with_capacity(64);
    let half = size / 2.0;
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            queue.push(make(Vec2::new(x + half, y + half), half));
            y += size;
        }
        x += size;
    }
    // The outline's centroid first, then the box's middle if it is better.
    let mut best = make(centroid(outer), 0.0);
    let middle = make(Vec2::new(x0 + w / 2.0, y0 + h / 2.0), 0.0);
    if middle.d > best.d {
        best = middle;
    }
    while let Some(cell) = queue.pop() {
        if cell.d > best.d {
            best = cell;
        }
        if cell.max - best.d <= precision {
            continue;
        }
        let h = cell.h / 2.0;
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let child = make(Vec2::new(cell.c.x + sx * h, cell.c.y + sy * h), h);
            // One that cannot pass the best (its distance at most its potential, the best only grows)
            // would be taken out and dropped: it is dropped now, its turn kept by the others.
            if child.max > best.d {
                queue.push(child);
            }
        }
    }
    (best.d > 0.0).then_some((best.c, best.d))
}

/// A ring's centroid by its area (the first point when it has none).
pub fn centroid(ring: &[Vec2]) -> Vec2 {
    let (mut area, mut x, mut y) = (0.0, 0.0, 0.0);
    let n = ring.len();
    if n == 0 {
        return Vec2::default();
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        let f = a.x * b.y - b.x * a.y;
        x += (a.x + b.x) * f;
        y += (a.y + b.y) * f;
        area += f * 3.0;
        j = i;
    }
    if area == 0.0 {
        return ring[0];
    }
    Vec2::new(x / area, y / area)
}

/// The convex hull (Andrew's monotone chain), counter-clockwise, without
/// points on its edges.
pub fn hull(pts: &[Vec2]) -> Vec<Vec2> {
    let mut p: Vec<Vec2> = pts.to_vec();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: Vec2, a: Vec2, b: Vec2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower: Vec<Vec2> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0.0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<Vec2> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0.0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The direction of the long side of the smallest rectangle round the
/// points (one side on a hull edge; of equal areas the first edge),
/// radians; none for fewer than three points off one line.
pub fn long_axis(pts: &[Vec2]) -> Option<f64> {
    let h = hull(pts);
    if h.len() < 3 {
        return None;
    }
    let mut best: Option<(f64, f64)> = None;
    for i in 0..h.len() {
        let (a, b) = (h[i], h[(i + 1) % h.len()]);
        let len = norm(b.x - a.x, b.y - a.y);
        if len == 0.0 {
            continue;
        }
        let u = Vec2::new((b.x - a.x) / len, (b.y - a.y) / len);
        let v = Vec2::new(-u.y, u.x);
        let (mut s0, mut s1, mut t0, mut t1) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for p in &h {
            let d = sub(*p, a);
            let (s, t) = (dot(d, u), dot(d, v));
            s0 = js_min(s0, s);
            s1 = js_max(s1, s);
            t0 = js_min(t0, t);
            t1 = js_max(t1, t);
        }
        let area = (s1 - s0) * (t1 - t0);
        if best.is_none_or(|(least, _)| area < least) {
            let along = atan2(u.y, u.x);
            let angle = if s1 - s0 >= t1 - t0 {
                along
            } else {
                atan2(v.y, v.x)
            };
            best = Some((area, angle));
        }
    }
    best.map(|(_, a)| a)
}

/// An angle made to read: in `(−π/2, π/2]`.
pub fn upright(angle: f64) -> f64 {
    let half = std::f64::consts::FRAC_PI_2;
    let mut a = angle;
    while a > half {
        a -= std::f64::consts::PI;
    }
    while a <= -half {
        a += std::f64::consts::PI;
    }
    a
}

/// An angle in `(−π, π]`.
pub fn wrap(angle: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let mut a = angle;
    while a > pi {
        a -= 2.0 * pi;
    }
    while a <= -pi {
        a += 2.0 * pi;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn boxes_overlap_by_their_axes_and_touching_ones_do_not() {
        let a = Obb::level(p(0.0, 0.0), 10.0, 5.0);
        assert!(a.overlaps(&Obb::level(p(19.0, 0.0), 10.0, 5.0)));
        assert!(!a.overlaps(&Obb::level(p(20.0, 0.0), 10.0, 5.0)));
        // Turned 45°: its corner reaches 7.07 back along x.
        assert!(a.overlaps(&Obb::new(
            p(16.5, 0.0),
            std::f64::consts::FRAC_PI_4,
            5.0,
            5.0
        )));
        assert!(!a.overlaps(&Obb::new(
            p(17.2, 0.0),
            std::f64::consts::FRAC_PI_4,
            5.0,
            5.0
        )));
    }

    #[test]
    fn a_segment_hits_a_box_it_passes_through() {
        let b = Obb::level(p(0.0, 0.0), 10.0, 5.0);
        assert!(b.hits_segment(p(-20.0, 0.0), p(20.0, 1.0)));
        assert!(!b.hits_segment(p(-20.0, 6.0), p(20.0, 6.0)));
        // Along an edge: touching only.
        assert!(!b.hits_segment(p(-20.0, 5.0), p(20.0, 5.0)));
        // Diagonal passing the corner by.
        assert!(!b.hits_segment(p(11.0, 0.0), p(20.0, 9.0)));
        assert!(b.hits_segment(p(9.0, 0.0), p(20.0, 11.0)));
    }

    #[test]
    fn the_pole_of_an_l_is_in_its_wider_arm() {
        let l = vec![
            p(0.0, 0.0),
            p(100.0, 0.0),
            p(100.0, 20.0),
            p(20.0, 20.0),
            p(20.0, 100.0),
            p(0.0, 100.0),
        ];
        let (c, d) = pole(std::slice::from_ref(&l), 0.5).unwrap();
        assert!(inside_rings(c, &[l]));
        // The largest circle touches the two outer walls and the inner corner: c = 20√2 / (1 + √2).
        let best = 20.0 * std::f64::consts::SQRT_2 / (1.0 + std::f64::consts::SQRT_2);
        assert!(
            (d - best).abs() < 0.5 && (c.x - best).abs() < 1.0 && (c.y - best).abs() < 1.0,
            "{c:?} {d}"
        );
    }

    #[test]
    fn a_path_cut_to_a_rectangle_keeps_its_inside_pieces() {
        let path = [
            p(-10.0, 5.0),
            p(5.0, 5.0),
            p(5.0, 20.0),
            p(8.0, 20.0),
            p(8.0, 5.0),
            p(20.0, 5.0),
        ];
        let pieces = clip_path(&path, [0.0, 0.0, 10.0, 10.0]);
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0], vec![p(0.0, 5.0), p(5.0, 5.0), p(5.0, 10.0)]);
        assert_eq!(pieces[1], vec![p(8.0, 10.0), p(8.0, 5.0), p(10.0, 5.0)]);
    }

    #[test]
    fn the_long_axis_of_a_turned_rectangle_is_its_length() {
        let a = 0.5f64;
        let (c, s) = (cos(a), sin(a));
        let r = [
            p(0.0, 0.0),
            p(40.0 * c, 40.0 * s),
            p(40.0 * c - 10.0 * s, 40.0 * s + 10.0 * c),
            p(-10.0 * s, 10.0 * c),
        ];
        assert!((long_axis(&r).unwrap() - a).abs() < 1e-12);
    }
}
