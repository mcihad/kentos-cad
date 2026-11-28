//! A region's least width (docs/adr/8110): the narrowest place one passes through it, a corridor's width (BYKHY 49/2).
//! It is the shortest chord across the region that leaves, on each side, room for a circle as wide as the chord is long:
//! a door's reveal or a niche shallower than its mouth is no neck, a branch as deep as it is wide is one. A region with
//! no such chord is as wide as its largest circle ([`pole`]). Its largest circle alone would make an L-shaped corridor
//! up to 17 % wider than it is, a T 25 %, a cross 41 %: the circle grows where the arms meet.
//!
//! - **Chords tried:** square into the region from points along every edge ([`STEP`] apart, each piece's middle) and,
//!   from every reflex corner, square to its two edges, to the nearest point of every edge and to every reflex corner
//!   nearer than the region is wide; each runs to the first ring it meets. Where, along one wall, a chord that is no neck
//!   lies next to a longer one that is (a sloping wall), the place between is found by halving.
//! - **A neck** has both ends on one ring (a chord from the outline to a hole does not cut the region in two) and leaves
//!   each side a circle at least as wide as itself, to within `precision`: the passages beside a free-standing column
//!   (a hole) are not measured.
//! - **The rings** are moved to the outline's first point first, so a room far out in a projected plane keeps its
//!   millimetres.
//!
//! What it does not see: the narrow end of a corridor tapering to a dead end is no neck (beyond every chord there the
//! corridor is narrower than the chord, as in a corner), nor is the gap beside a free-standing column.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::f64::consts::{PI, SQRT_2};

use crate::Vec2;
use crate::geometry::{point_in_polygon, signed_area};
use crate::jsmath::{js_max, js_min};
use crate::labels::geom::{inside_rings, pole, segment_distance2, signed_distance};

/// How far apart the chords square to an edge are tried, metres.
pub const STEP: f64 = 0.01;
/// How many times the place between a neck and a chord that is none along one wall is halved.
const REFINE: usize = 32;
/// Chords whose lengths differ by less than this are of one length, metres: of those, the one nearest the largest
/// circle is tried first.
const SAME: f64 = 1e-6;
/// A ray meets nothing nearer its start than this, metres (its own edge, its own corner).
const NEAR: f64 = 1e-9;
/// The region's largest circle is found to within this first, metres: it only orders the chords and bounds their
/// length (a long corridor's circle to the final precision is a long search along its middle).
const COARSE: f64 = 1e-2;

/// A region's least width and where it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeastWidth {
    /// Its least width, metres.
    pub width: f64,
    /// The neck's two ends; the largest circle's centre twice for a region with no neck.
    pub from: Vec2,
    pub to: Vec2,
    /// Whether a neck sets it (else its largest circle).
    pub neck: bool,
}

/// The least width of the region `rings` (the first its outline, the rest its holes; closed implicitly, either
/// orientation), its circles found to within `precision`; none for an outline of fewer than three points or no area.
pub fn least_width(rings: &[Vec<Vec2>], precision: f64) -> Option<LeastWidth> {
    let first = rings.first()?;
    let o = *first.first()?;
    let rings: Vec<Vec<Vec2>> = rings
        .iter()
        .map(|r| tidy(r.iter().map(|p| Vec2::new(p.x - o.x, p.y - o.y))))
        .enumerate()
        .filter(|(i, r)| *i == 0 || r.len() >= 3)
        .map(|(_, r)| r)
        .collect();
    if rings[0].len() < 3 {
        return None;
    }
    let (c, d) = pole(&rings, js_max(precision, COARSE))?;
    let back = |p: Vec2| Vec2::new(p.x + o.x, p.y + o.y);
    let region = Region::new(&rings, c, d, precision);
    if let Some(ch) = region.least() {
        return Some(LeastWidth {
            width: ch.len,
            from: back(ch.p),
            to: back(ch.q),
            neck: true,
        });
    }
    // No neck: a room about as long as it is wide, its circle soon found.
    let (c, d) = pole(&rings, precision)?;
    Some(LeastWidth {
        width: 2.0 * d,
        from: back(c),
        to: back(c),
        neck: false,
    })
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn length(v: Vec2) -> f64 {
    (v.x * v.x + v.y * v.y).sqrt()
}

fn along(a: Vec2, b: Vec2, t: f64) -> Vec2 {
    Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn same(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() <= 1e-12 && (a.y - b.y).abs() <= 1e-12
}

/// A ring without repeated points, nor its first point again at its end.
fn tidy(points: impl Iterator<Item = Vec2>) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::new();
    for p in points {
        if out.last().is_none_or(|l| !same(*l, p)) {
            out.push(p);
        }
    }
    while out.len() > 1 && same(out[0], out[out.len() - 1]) {
        out.pop();
    }
    out
}

/// A point on a ring: on its edge `edge` (from its point `edge` to the next), `t` along it.
#[derive(Clone, Copy, Debug)]
struct At {
    ring: usize,
    edge: usize,
    t: f64,
}

/// A chord across the region from `p` (at `a`) to `q` (at `b`).
#[derive(Clone, Copy, Debug)]
struct Chord {
    p: Vec2,
    q: Vec2,
    len: f64,
    a: At,
    b: At,
}

/// A chord tried, and the edge point it was drawn square from: its ring, edge and place among the edge's points.
struct Tried {
    chord: Chord,
    from: Option<(usize, usize, usize)>,
}

struct Region<'a> {
    rings: &'a [Vec<Vec2>],
    /// Whether the region lies on each ring's left.
    left: Vec<bool>,
    /// Its largest circle: its centre and radius.
    c: Vec2,
    d: f64,
    precision: f64,
}

impl<'a> Region<'a> {
    fn new(rings: &'a [Vec<Vec2>], c: Vec2, d: f64, precision: f64) -> Self {
        let left = rings
            .iter()
            .enumerate()
            .map(|(i, r)| (signed_area(r) > 0.0) == (i == 0))
            .collect();
        Region {
            rings,
            left,
            c,
            d,
            precision,
        }
    }

    fn edge(&self, r: usize, i: usize) -> (Vec2, Vec2) {
        let ring = &self.rings[r];
        (ring[i], ring[(i + 1) % ring.len()])
    }

    /// The unit normal of an edge into the region; none for an edge of no length.
    fn inward(&self, r: usize, i: usize) -> Option<Vec2> {
        let (a, b) = self.edge(r, i);
        let e = sub(b, a);
        let l = length(e);
        if !(l > 0.0) {
            return None;
        }
        Some(if self.left[r] {
            Vec2::new(-e.y / l, e.x / l)
        } else {
            Vec2::new(e.y / l, -e.x / l)
        })
    }

    /// Where the ray from `o` along `u` (a unit vector) first meets a ring, past [`NEAR`], the edges `skip` names left
    /// out: how far, and where.
    fn hit(&self, o: Vec2, u: Vec2, skip: &[(usize, usize)]) -> Option<(f64, At)> {
        let mut best: Option<(f64, At)> = None;
        for (r, ring) in self.rings.iter().enumerate() {
            let n = ring.len();
            for i in 0..n {
                if skip.contains(&(r, i)) {
                    continue;
                }
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                let e = sub(b, a);
                let den = cross(u, e);
                if den == 0.0 {
                    continue;
                }
                let w = sub(a, o);
                let s = cross(w, e) / den;
                let t = cross(w, u) / den;
                if s > NEAR
                    && (-1e-12..=1.0 + 1e-12).contains(&t)
                    && best.is_none_or(|(bs, _)| s < bs)
                {
                    best = Some((
                        s,
                        At {
                            ring: r,
                            edge: i,
                            t: t.clamp(0.0, 1.0),
                        },
                    ));
                }
            }
        }
        best
    }

    /// The chord from `from` along `u` to the first ring it meets.
    fn ray(&self, from: At, u: Vec2, skip: &[(usize, usize)]) -> Option<Chord> {
        let (a, b) = self.edge(from.ring, from.edge);
        let p = along(a, b, from.t);
        let (_, at) = self.hit(p, u, skip)?;
        let (c, d) = self.edge(at.ring, at.edge);
        let q = along(c, d, at.t);
        Some(Chord {
            p,
            q,
            len: length(sub(q, p)),
            a: from,
            b: at,
        })
    }

    /// How many points along an edge chords are drawn square from.
    fn points_on(&self, r: usize, i: usize) -> usize {
        let (a, b) = self.edge(r, i);
        let k = (length(sub(b, a)) / STEP).ceil();
        if k >= 1.0 { k as usize } else { 1 }
    }

    /// The chord square from the point `t` along an edge.
    fn square(&self, r: usize, i: usize, t: f64) -> Option<Chord> {
        let u = self.inward(r, i)?;
        self.ray(
            At {
                ring: r,
                edge: i,
                t,
            },
            u,
            &[(r, i)],
        )
    }

    /// Whether a ring's point is a reflex corner of the region.
    fn reflex(&self, r: usize, k: usize) -> bool {
        let ring = &self.rings[r];
        let n = ring.len();
        let turn = cross(
            sub(ring[k], ring[(k + n - 1) % n]),
            sub(ring[(k + 1) % n], ring[k]),
        );
        if self.left[r] { turn < 0.0 } else { turn > 0.0 }
    }

    /// Every chord tried.
    fn chords(&self) -> Vec<Tried> {
        // No neck is longer than the region's largest circle is wide.
        let reach = 2.0 * (self.d + js_max(self.precision, COARSE)) + 2.0 * self.precision;
        let mut out = Vec::new();
        for (r, ring) in self.rings.iter().enumerate() {
            for i in 0..ring.len() {
                let k = self.points_on(r, i);
                for j in 0..k {
                    let t = (j as f64 + 0.5) / k as f64;
                    if let Some(chord) = self.square(r, i, t) {
                        out.push(Tried {
                            chord,
                            from: Some((r, i, j)),
                        });
                    }
                }
            }
        }
        let corners: Vec<(usize, usize)> = self
            .rings
            .iter()
            .enumerate()
            .flat_map(|(r, ring)| (0..ring.len()).map(move |k| (r, k)))
            .filter(|&(r, k)| self.reflex(r, k))
            .collect();
        for &(r, k) in &corners {
            let n = self.rings[r].len();
            let v = self.rings[r][k];
            let own = [(r, (k + n - 1) % n), (r, k)];
            let mut toward: Vec<Vec2> = own
                .iter()
                .filter_map(|&(rr, i)| self.inward(rr, i))
                .collect();
            for (rr, ring) in self.rings.iter().enumerate() {
                for i in 0..ring.len() {
                    if own.contains(&(rr, i)) {
                        continue;
                    }
                    let (a, b) = self.edge(rr, i);
                    let e = sub(b, a);
                    let ee = e.x * e.x + e.y * e.y;
                    let t = if ee > 0.0 {
                        (((v.x - a.x) * e.x + (v.y - a.y) * e.y) / ee).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let f = sub(along(a, b, t), v);
                    let l = length(f);
                    if l > NEAR && l <= reach {
                        toward.push(Vec2::new(f.x / l, f.y / l));
                    }
                }
            }
            for &(rr, kk) in &corners {
                let f = sub(self.rings[rr][kk], v);
                let l = length(f);
                if (rr, kk) != (r, k) && l > NEAR && l <= reach {
                    toward.push(Vec2::new(f.x / l, f.y / l));
                }
            }
            for u in toward {
                let step = Vec2::new(v.x + u.x * 1e-6, v.y + u.y * 1e-6);
                if !inside_rings(step, self.rings) {
                    continue;
                }
                if let Some(chord) = self.ray(
                    At {
                        ring: r,
                        edge: k,
                        t: 0.0,
                    },
                    u,
                    &own,
                ) {
                    out.push(Tried { chord, from: None });
                }
            }
        }
        out.retain(|x| {
            x.chord.a.ring == x.chord.b.ring && x.chord.len > 0.0 && x.chord.len <= reach
        });
        out
    }

    /// The shortest neck, if any.
    fn least(&self) -> Option<Chord> {
        let tried = self.chords();
        let key = |x: &Tried| {
            let m = Vec2::new(
                (x.chord.p.x + x.chord.q.x) / 2.0,
                (x.chord.p.y + x.chord.q.y) / 2.0,
            );
            (
                (x.chord.len / SAME).floor(),
                (m.x - self.c.x) * (m.x - self.c.x) + (m.y - self.c.y) * (m.y - self.c.y),
            )
        };
        let mut order: Vec<usize> = (0..tried.len()).collect();
        order.sort_by(|&a, &b| {
            let (ka, kb) = (key(&tried[a]), key(&tried[b]));
            ka.0.total_cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        });
        let mut status: HashMap<usize, bool> = HashMap::new();
        let mut first = None;
        for &x in &order {
            let ok = self.neck(&tried[x].chord);
            status.insert(x, ok);
            if ok {
                first = Some(tried[x].chord);
                break;
            }
        }
        let mut best = first?;
        // Along a sloping wall: between a chord that is no neck and the longer one next to it that is.
        let at: HashMap<(usize, usize, usize), usize> = tried
            .iter()
            .enumerate()
            .filter_map(|(x, t)| t.from.map(|f| (f, x)))
            .collect();
        let mut failed: Vec<usize> = status
            .iter()
            .filter(|(_, ok)| !**ok)
            .map(|(x, _)| *x)
            .collect();
        failed.sort_unstable();
        for x in failed {
            let Some((r, i, j)) = tried[x].from else {
                continue;
            };
            let k = self.points_on(r, i);
            for nj in [j.checked_sub(1), Some(j + 1)].into_iter().flatten() {
                let Some(&nx) = at.get(&(r, i, nj)) else {
                    continue;
                };
                if tried[nx].chord.len <= tried[x].chord.len + SAME {
                    continue;
                }
                let ok = match status.get(&nx) {
                    Some(ok) => *ok,
                    None => {
                        let ok = self.neck(&tried[nx].chord);
                        status.insert(nx, ok);
                        ok
                    }
                };
                if !ok {
                    continue;
                }
                let fail = (j as f64 + 0.5) / k as f64;
                let good = (nj as f64 + 0.5) / k as f64;
                let found = self.refine(r, i, fail, good, tried[nx].chord);
                if found.len < best.len {
                    best = found;
                }
            }
        }
        Some(best)
    }

    /// Between the chord square from `fail` along an edge (no neck) and `good`'s (a neck): the neck nearest `fail`.
    fn refine(&self, r: usize, i: usize, mut fail: f64, mut good: f64, mut chord: Chord) -> Chord {
        for _ in 0..REFINE {
            let t = (fail + good) / 2.0;
            match self.square(r, i, t) {
                Some(c) if c.a.ring == c.b.ring && self.neck(&c) => {
                    good = t;
                    chord = c;
                }
                _ => fail = t,
            }
        }
        chord
    }

    /// Whether a chord is a neck: it cuts the region in two, each side holding a circle as wide as it is long.
    fn neck(&self, ch: &Chord) -> bool {
        let Some(sides) = self.split(ch) else {
            return false;
        };
        sides.iter().all(|side| self.holds(side, ch))
    }

    /// Whether one side of a chord holds a circle as wide as the chord is long, to within the precision.
    fn holds(&self, side: &[Vec<Vec2>], ch: &Chord) -> bool {
        let half = ch.len / 2.0;
        let need = js_max(half - self.precision, 0.0);
        let outer = &side[0];
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
        if x1 - x0 < 2.0 * need || y1 - y0 < 2.0 * need {
            return false;
        }
        let area =
            signed_area(outer).abs() - side[1..].iter().map(|h| signed_area(h).abs()).sum::<f64>();
        if area < PI * need * need {
            return false;
        }
        // The region's largest circle, on this side and clear of the chord, is circle enough.
        if self.d >= need
            && segment_distance2(self.c, ch.p, ch.q) >= need * need
            && inside_rings(self.c, side)
        {
            return true;
        }
        fits(side, half, self.precision)
    }

    /// The region's two sides of a chord with both ends on one ring; none for a chord from one ring to another.
    fn split(&self, ch: &Chord) -> Option<[Vec<Vec<Vec2>>; 2]> {
        let (a, b) = (ch.a, ch.b);
        if a.ring != b.ring {
            return None;
        }
        let ring = &self.rings[a.ring];
        let one = walk(ring, a, ch.p, b, ch.q);
        let two = walk(ring, b, ch.q, a, ch.p);
        if one.len() < 3 || two.len() < 3 {
            return None;
        }
        let holes = || (1..self.rings.len()).filter(move |&h| h != a.ring);
        if a.ring == 0 {
            let (mut s1, mut s2) = (vec![one], vec![two]);
            for h in holes() {
                let hole = &self.rings[h];
                if point_in_polygon(hole[0], &s1[0]) {
                    s1.push(hole.clone());
                } else {
                    s2.push(hole.clone());
                }
            }
            return Some([s1, s2]);
        }
        // A chord from a hole round to itself cuts off the pocket between it and the hole; the rest keeps the hole
        // with the pocket as one.
        let (pocket, rest) = if signed_area(&one).abs() <= signed_area(&two).abs() {
            (one, two)
        } else {
            (two, one)
        };
        let (mut s1, mut s2) = (vec![pocket], vec![self.rings[0].clone()]);
        for h in holes() {
            let hole = &self.rings[h];
            if point_in_polygon(hole[0], &s1[0]) {
                s1.push(hole.clone());
            } else {
                s2.push(hole.clone());
            }
        }
        s2.push(rest);
        Some([s1, s2])
    }
}

/// A cell of [`fits`]'s search: its middle, half its side, its middle's distance from the edges.
struct Cell {
    c: Vec2,
    h: f64,
    d: f64,
    seq: u64,
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
    /// The middle farther from the edges first, then the earlier made.
    fn cmp(&self, o: &Self) -> Ordering {
        self.d.total_cmp(&o.d).then_with(|| o.seq.cmp(&self.seq))
    }
}

/// Whether the rings hold a circle of radius `r`: a point inside, at least `r` from every edge. Yes when they do; when
/// they do not, yes only for a point at least `r - margin` from the edges. A search over square cells, the middle
/// farthest from the edges first, a cell dropped once none of its points can be `r` from the edges (its middle's
/// distance and half its diagonal): it stops at the first middle near enough, so a long side is soon answered where
/// finding its largest circle would walk the whole of its middle.
fn fits(rings: &[Vec<Vec2>], r: f64, margin: f64) -> bool {
    let Some(outer) = rings.first().filter(|o| o.len() >= 3) else {
        return false;
    };
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
    let size = js_min(x1 - x0, y1 - y0);
    if !(size > 0.0) {
        return false;
    }
    let mut seq = 0u64;
    let mut cell = |c: Vec2, h: f64| {
        seq += 1;
        Cell {
            c,
            h,
            d: signed_distance(c, rings),
            seq,
        }
    };
    let mut queue = BinaryHeap::new();
    let half = size / 2.0;
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            queue.push(cell(Vec2::new(x + half, y + half), half));
            y += size;
        }
        x += size;
    }
    while let Some(c) = queue.pop() {
        if c.d >= r - margin {
            return true;
        }
        let h = c.h / 2.0;
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let child = cell(Vec2::new(c.c.x + sx * h, c.c.y + sy * h), h);
            if child.d + child.h * SQRT_2 >= r {
                queue.push(child);
            }
        }
    }
    false
}

/// The ring from `p` (at `from`) forward to `q` (at `to`), closed by the chord back to `p`.
fn walk(ring: &[Vec2], from: At, p: Vec2, to: At, q: Vec2) -> Vec<Vec2> {
    let n = ring.len();
    let mut out = vec![p];
    let push = |out: &mut Vec<Vec2>, x: Vec2| {
        if out.last().is_none_or(|l| !same(*l, x)) {
            out.push(x);
        }
    };
    if !(to.edge == from.edge && to.t >= from.t) {
        let mut i = (from.edge + 1) % n;
        loop {
            push(&mut out, ring[i]);
            if i == to.edge {
                break;
            }
            i = (i + 1) % n;
        }
    }
    push(&mut out, q);
    if out.len() > 1 && same(out[0], out[out.len() - 1]) {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: f64 = 1e-4;

    fn ring(points: &[(f64, f64)]) -> Vec<Vec2> {
        points.iter().map(|&(x, y)| Vec2::new(x, y)).collect()
    }

    fn width(rings: &[Vec<Vec2>]) -> LeastWidth {
        least_width(rings, P).expect("a region")
    }

    fn near(got: f64, want: f64, tol: f64) {
        assert!((got - want).abs() <= tol, "got {got}, want {want}");
    }

    #[test]
    fn a_long_corridor_is_as_wide_as_its_walls_are_apart() {
        let w = width(&[ring(&[(0.0, 0.0), (20.0, 0.0), (20.0, 2.0), (0.0, 2.0)])]);
        near(w.width, 2.0, 1e-9);
        assert!(w.neck);
        near((w.from.x - w.to.x).abs(), 0.0, 1e-9);
    }

    #[test]
    fn a_short_room_has_no_neck_and_is_as_wide_as_its_circle() {
        let w = width(&[ring(&[(0.0, 0.0), (3.0, 0.0), (3.0, 2.0), (0.0, 2.0)])]);
        assert!(!w.neck);
        near(w.width, 2.0, 2.0 * P);
    }

    #[test]
    fn an_l_a_t_and_a_cross_are_as_wide_as_their_arms() {
        let l = ring(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 1.8),
            (1.8, 1.8),
            (1.8, 10.0),
            (0.0, 10.0),
        ]);
        near(width(&[l]).width, 1.8, 1e-9);
        let t = ring(&[
            (0.0, 0.0),
            (20.0, 0.0),
            (20.0, 1.6),
            (10.8, 1.6),
            (10.8, 10.0),
            (9.2, 10.0),
            (9.2, 1.6),
            (0.0, 1.6),
        ]);
        near(width(&[t]).width, 1.6, 1e-9);
        let x = ring(&[
            (-0.75, -10.0),
            (0.75, -10.0),
            (0.75, -0.75),
            (10.0, -0.75),
            (10.0, 0.75),
            (0.75, 0.75),
            (0.75, 10.0),
            (-0.75, 10.0),
            (-0.75, 0.75),
            (-10.0, 0.75),
            (-10.0, -0.75),
            (-0.75, -0.75),
        ]);
        near(width(&[x]).width, 1.5, 1e-9);
    }

    #[test]
    fn a_door_reveal_is_no_neck_but_a_deep_niche_is() {
        // A 2.00 m corridor with a 1.00 m reveal 0.15 m deep in one wall.
        let reveal = ring(&[
            (0.0, 0.0),
            (9.5, 0.0),
            (9.5, -0.15),
            (10.5, -0.15),
            (10.5, 0.0),
            (20.0, 0.0),
            (20.0, 2.0),
            (0.0, 2.0),
        ]);
        near(width(&[reveal]).width, 2.0, 1e-9);
        // The same with a niche 1.00 m wide and 1.20 m deep: a branch narrower than the corridor.
        let niche = ring(&[
            (0.0, 0.0),
            (9.5, 0.0),
            (9.5, -1.2),
            (10.5, -1.2),
            (10.5, 0.0),
            (20.0, 0.0),
            (20.0, 2.0),
            (0.0, 2.0),
        ]);
        near(width(&[niche]).width, 1.0, 1e-9);
    }

    #[test]
    fn a_pier_narrows_the_corridor_to_the_gap_beside_it() {
        let pier = ring(&[
            (0.0, 0.0),
            (9.7, 0.0),
            (9.7, 0.4),
            (10.3, 0.4),
            (10.3, 0.0),
            (20.0, 0.0),
            (20.0, 2.4),
            (0.0, 2.4),
        ]);
        near(width(&[pier]).width, 2.0, 1e-9);
    }

    #[test]
    fn a_corridor_narrowing_between_two_wide_ends_is_its_waist_wide() {
        // 2.50 m at both ends, 1.50 m in the middle.
        let waist = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 2.5), (5.0, 1.5), (0.0, 2.5)]);
        let w = width(&[waist]);
        assert!(w.neck, "{w:?}");
        near(w.width, 1.5, 1e-9);
    }

    #[test]
    fn the_narrow_end_of_a_corridor_tapering_to_a_dead_end_is_no_neck() {
        // 1.50 m at its closed end, 2.50 m at the other: beyond every chord it is narrower than the chord, as a corner
        // is; it is as wide as its largest circle, which touches the wide end: 2.50 / (1 + (√1.01 - 1) / 2 + 0.05).
        let taper = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 2.5), (0.0, 1.5)]);
        let w = width(&[taper]);
        assert!(!w.neck, "{w:?}");
        near(
            w.width,
            2.5 / ((1.0 + 1.01f64.sqrt()) / 2.0 + 0.05),
            4.0 * P,
        );
    }

    #[test]
    fn far_out_coordinates_keep_their_millimetres() {
        let (x, y) = (4_400_000.0, 520_000.0);
        let l = ring(&[
            (x, y),
            (x + 10.0, y),
            (x + 10.0, y + 1.8),
            (x + 1.8, y + 1.8),
            (x + 1.8, y + 10.0),
            (x, y + 10.0),
        ]);
        let w = width(&[l]);
        near(w.width, 1.8, 1e-9);
        assert!(w.from.x > x && w.from.y >= y);
    }

    #[test]
    fn a_ring_drawn_either_way_round_gives_one_width() {
        let mut l = ring(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 1.8),
            (1.8, 1.8),
            (1.8, 10.0),
            (0.0, 10.0),
        ]);
        let a = width(std::slice::from_ref(&l)).width;
        l.reverse();
        near(width(&[l]).width, a, 1e-12);
    }
}
