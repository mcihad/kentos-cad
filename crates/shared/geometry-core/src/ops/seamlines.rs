//! Two overlapping footprints' seamline polygons (plan/map-cad.md MAP-12, docs/adr/4700), the geometry of
//! `cad.orthophoto.seamlines`.
//!
//! - **Footprints** are convex rings of straight edges (a raster's extent, or an area the user gives), taken
//!   counter-clockwise without their closing point, repeated corners or corners on a straight edge.
//! - **Overlap**: the first clipped by the second (Sutherland–Hodgman; a point on an edge's line is inside), from its
//!   southernmost corner (the westernmost of equals). Each of
//!   its edges lies on the second's boundary only (`FirstEnd`: where the second image ends inside the first, so the
//!   first takes over beyond it), on the first's only (`SecondEnd`), or on both (`Shared`, the union's outer boundary).
//!   Going round it the first's end and the second's end must each be one run: the boundary changes hands twice.
//! - **Distances** (Pan, Wang, Li, Li 2009, the area Voronoi diagram with overlap): a point's distance to an image's
//!   own part (its footprint less the other's) is, inside the overlap, its distance to that image's end run;
//!   `f(p)` = to the first's − to the second's. On the first's run `f` < 0, on the second's `f` > 0.
//! - **Ends**: going round counter-clockwise, after the first's run the boundary reaches the second's either at a
//!   corner (the footprints' boundaries cross: the end) or along a shared run (the end is its first point, from the
//!   first's side, where `f` reaches 0: its edge found by the corners, the point by bisection). The seam goes from the
//!   end after the first's run (`start`) to the one after the second's (`end`); the first's run lies on its left.
//! - **Voronoi seam**: `f` = 0 traced on lines across the chord from start to end, every `step` along it (at least
//!   one between), each line's root by bisection. At each root the nearest element of each run is an edge or a corner;
//!   while both are edges, or both corners, or a corner on the other's edge line, the seam is straight (a bisector
//!   line) and only its ends are kept: where it turns, the next piece is found by bisection on the chord and the turn
//!   is the two lines' crossing. A piece between a corner and an edge is a parabola: its roots are kept.
//! - **Polygons**: the first's ring is its boundary from the end back to the start (counter-clockwise, the side
//!   without the second's end), then the seam; the second's is its boundary from the start to the end, then the seam
//!   back. They share the seam's points exactly, do not overlap and cover both footprints.
//! - **Grid path** (radiometry; the same paper's refinement, Chon, Kim and Lin 2010): through cells of known
//!   difference, 8 neighbours, the path whose largest difference is the smallest (minimax), then among the cells under
//!   it the path of least cost, a move costing its length (1 or √2 cells) × (1 + the mean of its two cells'
//!   differences). Ties go to the smaller cost, then the smaller cell index; a cell's cost is set only when strictly
//!   lowered.
//!
//! Everything is computed about the first footprint's first corner (small numbers) and given back in place.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::jsmath::{js_hypot, js_max};
use crate::predicates::orientation;
use crate::vec2::Vec2;

/// A point (east, north).
pub type Pt = [f64; 2];

/// How near (metres) a point lies to an edge to be on it.
const ON: f64 = 1e-7;
/// Two points this near are one.
const SAME: f64 = 1e-9;
/// Halvings of a bisection: past a double's precision.
const HALVINGS: u32 = 80;

/// Why two footprints give no seam.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamFault {
    /// Footprint k (0 the first) has fewer than three corners or no area.
    Degenerate(usize),
    /// Footprint k is not convex.
    NotConvex(usize),
    /// They do not overlap, or only touch.
    Apart,
    /// Footprint k lies inside the other (or they are the same): the other covers it, no seam.
    Inside(usize),
    /// The overlap's boundary changes hands more than twice.
    Crossed,
}

/// An overlap edge's place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// On the second's boundary only: the second image ends here, inside the first.
    FirstEnd,
    /// On the first's boundary only.
    SecondEnd,
    /// On both: the union's outer boundary.
    Shared,
}

/// A run's nearest element to a point: a corner or an edge (its index in the run).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Near {
    Corner(usize),
    Edge(usize),
}

fn sub(a: Pt, b: Pt) -> Pt {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: Pt, b: Pt) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: Pt, b: Pt) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn len(a: Pt) -> f64 {
    js_hypot(a[0], a[1])
}
fn lerp(a: Pt, b: Pt, t: f64) -> Pt {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}
fn v2(p: Pt) -> Vec2 {
    Vec2::new(p[0], p[1])
}
fn turn(a: Pt, b: Pt, c: Pt) -> i8 {
    orientation(v2(a), v2(b), v2(c))
}

/// A ring's signed area (counter-clockwise positive).
pub fn ring_area(r: &[Pt]) -> f64 {
    let n = r.len();
    if n < 3 {
        return 0.0;
    }
    let o = r[0];
    let mut s = 0.0;
    for i in 1..n - 1 {
        s += cross(sub(r[i], o), sub(r[i + 1], o));
    }
    s / 2.0
}

/// A point's distance to the segment a–b.
pub fn to_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    if l2 == 0.0 {
        return len(sub(p, a));
    }
    let t = dot(sub(p, a), ab) / l2;
    if t <= 0.0 {
        len(sub(p, a))
    } else if t >= 1.0 {
        len(sub(p, b))
    } else {
        cross(ab, sub(p, a)).abs() / l2.sqrt()
    }
}

/// A point's distance to a polyline.
pub fn to_polyline(p: Pt, line: &[Pt]) -> f64 {
    match line {
        [] => f64::INFINITY,
        [a] => len(sub(p, *a)),
        _ => line.windows(2).fold(f64::INFINITY, |m, w| {
            let d = to_segment(p, w[0], w[1]);
            if d < m { d } else { m }
        }),
    }
}

/// Repeated neighbours (cyclic) dropped.
fn deduped(r: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(r.len());
    for &p in r {
        if out.last().is_none_or(|q| len(sub(p, *q)) > SAME) {
            out.push(p);
        }
    }
    while out.len() > 1 && len(sub(out[0], out[out.len() - 1])) <= SAME {
        out.pop();
    }
    out
}

/// Corners on a straight edge (exactly, or within `SAME` of their neighbours' line) dropped.
fn straightened(mut r: Vec<Pt>) -> Vec<Pt> {
    loop {
        let n = r.len();
        if n < 3 {
            return r;
        }
        let at = (0..n).find(|&i| {
            let (a, b, c) = (r[(i + n - 1) % n], r[i], r[(i + 1) % n]);
            let ac = sub(c, a);
            turn(a, b, c) == 0 || (len(ac) > SAME && cross(ac, sub(b, a)).abs() / len(ac) <= SAME)
        });
        match at {
            Some(i) => {
                r.remove(i);
            }
            None => return r,
        }
    }
}

/// A footprint made ready (counter-clockwise, convex), about `o`.
fn footprint(ring: &[Pt], o: Pt, k: usize) -> Result<Vec<Pt>, SeamFault> {
    let local: Vec<Pt> = ring.iter().map(|p| sub(*p, o)).collect();
    let mut r = straightened(deduped(&local));
    if r.len() < 3 {
        return Err(SeamFault::Degenerate(k));
    }
    let a = ring_area(&r);
    if !(a.abs() > 1e-12) {
        return Err(SeamFault::Degenerate(k));
    }
    if a < 0.0 {
        r.reverse();
    }
    let n = r.len();
    if (0..n).any(|i| turn(r[(i + n - 1) % n], r[i], r[(i + 1) % n]) < 0) {
        return Err(SeamFault::NotConvex(k));
    }
    Ok(r)
}

/// Where segment p–q crosses the line a→b.
fn cut(a: Pt, b: Pt, p: Pt, q: Pt) -> Pt {
    let ab = sub(b, a);
    let (sp, sq) = (cross(ab, sub(p, a)), cross(ab, sub(q, a)));
    lerp(p, q, sp / (sp - sq))
}

/// The first convex ring clipped by the second (Sutherland–Hodgman), counter-clockwise, cleaned.
fn clip(subject: &[Pt], clipper: &[Pt]) -> Vec<Pt> {
    let mut out = subject.to_vec();
    let n = clipper.len();
    for i in 0..n {
        let (a, b) = (clipper[i], clipper[(i + 1) % n]);
        let input = std::mem::take(&mut out);
        let m = input.len();
        for j in 0..m {
            let (p, q) = (input[j], input[(j + 1) % m]);
            let (sp, sq) = (turn(a, b, p), turn(a, b, q));
            if sp >= 0 {
                out.push(p);
            }
            if (sp > 0 && sq < 0) || (sp < 0 && sq > 0) {
                out.push(cut(a, b, p, q));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    straightened(deduped(&out))
}

/// Whether segment u–v lies on one of the ring's edges.
fn on_ring(ring: &[Pt], u: Pt, v: Pt) -> bool {
    let n = ring.len();
    let mid = lerp(u, v, 0.5);
    (0..n).any(|i| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        to_segment(u, a, b) <= ON && to_segment(v, a, b) <= ON && to_segment(mid, a, b) <= ON
    })
}

/// Two footprints made ready for their seam.
#[derive(Clone, Debug)]
pub struct Footprints {
    origin: Pt,
    first: Vec<Pt>,
    second: Vec<Pt>,
    overlap: Vec<Pt>,
    kinds: Vec<EdgeKind>,
    /// The first's end run (on the second's boundary) and the second's, as polylines.
    first_end: Vec<Pt>,
    second_end: Vec<Pt>,
    start: Pt,
    end: Pt,
    start_crossing: bool,
    end_crossing: bool,
}

/// A run of edges `[from, to]` (cyclic, inclusive) of one kind.
#[derive(Clone, Copy, Debug)]
struct Run {
    kind: EdgeKind,
    from: usize,
    to: usize,
}

fn runs(kinds: &[EdgeKind]) -> Vec<Run> {
    let n = kinds.len();
    let Some(s) = (0..n).find(|&i| kinds[i] != kinds[(i + n - 1) % n]) else {
        return vec![Run { kind: kinds[0], from: 0, to: n - 1 }];
    };
    let mut out = Vec::new();
    let mut k = 0;
    while k < n {
        let i = (s + k) % n;
        let mut e = k;
        while e + 1 < n && kinds[(s + e + 1) % n] == kinds[i] {
            e += 1;
        }
        out.push(Run { kind: kinds[i], from: i, to: (s + e) % n });
        k = e + 1;
    }
    out
}

impl Footprints {
    /// The two footprints' overlap and ends, or why they have no seam.
    pub fn new(first: &[Pt], second: &[Pt]) -> Result<Footprints, SeamFault> {
        let origin = *first.first().ok_or(SeamFault::Degenerate(0))?;
        let a = footprint(first, origin, 0)?;
        let b = footprint(second, origin, 1)?;
        let mut overlap = clip(&a, &b);
        if overlap.len() < 3 || ring_area(&overlap) <= 1e-6 {
            return Err(SeamFault::Apart);
        }
        // From its southernmost corner (the westernmost of equals): the ring does not depend on the clip's order.
        let first_corner = (0..overlap.len())
            .min_by(|&i, &j| overlap[i][1].total_cmp(&overlap[j][1]).then(overlap[i][0].total_cmp(&overlap[j][0])))
            .unwrap_or(0);
        overlap.rotate_left(first_corner);
        let n = overlap.len();
        let kinds: Vec<EdgeKind> = (0..n)
            .map(|i| {
                let (u, v) = (overlap[i], overlap[(i + 1) % n]);
                match (on_ring(&a, u, v), on_ring(&b, u, v)) {
                    (true, true) => EdgeKind::Shared,
                    (false, true) => EdgeKind::FirstEnd,
                    // An edge on neither cannot come from the clip; read it as the first's.
                    _ => EdgeKind::SecondEnd,
                }
            })
            .collect();
        let rs = runs(&kinds);
        let count = |k: EdgeKind| rs.iter().filter(|r| r.kind == k).count();
        if count(EdgeKind::FirstEnd) == 0 {
            return Err(SeamFault::Inside(0));
        }
        if count(EdgeKind::SecondEnd) == 0 {
            return Err(SeamFault::Inside(1));
        }
        if count(EdgeKind::FirstEnd) > 1 || count(EdgeKind::SecondEnd) > 1 {
            return Err(SeamFault::Crossed);
        }
        let run_of = |k: EdgeKind| *rs.iter().find(|r| r.kind == k).unwrap_or(&rs[0]);
        let line = |r: Run| -> Vec<Pt> {
            let mut pts = vec![overlap[r.from]];
            let mut i = r.from;
            loop {
                pts.push(overlap[(i + 1) % n]);
                if i == r.to {
                    break;
                }
                i = (i + 1) % n;
            }
            pts
        };
        let (r1, r2) = (run_of(EdgeKind::FirstEnd), run_of(EdgeKind::SecondEnd));
        let (first_end, second_end) = (line(r1), line(r2));
        let mut fp = Footprints {
            origin,
            first: a,
            second: b,
            overlap,
            kinds,
            first_end,
            second_end,
            start: [0.0; 2],
            end: [0.0; 2],
            start_crossing: false,
            end_crossing: false,
        };
        // After the first's run, the boundary reaches the second's: the start; after the second's, the end. A shared
        // run between is walked from the run before it (its `f` < 0 after the first's run, > 0 after the second's).
        let after = |r: Run| -> (usize, usize) { ((r.to + 1) % n, if r.kind == EdgeKind::FirstEnd { r2.from } else { r1.from }) };
        for (k, r) in [r1, r2].into_iter().enumerate() {
            let (s, stop) = after(r);
            let (at, crossing) = if s == stop {
                (fp.overlap[s], true)
            } else {
                let mut chain = vec![fp.overlap[s]];
                let mut i = s;
                while i != stop {
                    i = (i + 1) % n;
                    chain.push(fp.overlap[i]);
                }
                let sign = if k == 0 { 1.0 } else { -1.0 };
                (fp.root_along(&chain, sign), false)
            };
            if k == 0 {
                fp.start = at;
                fp.start_crossing = crossing;
            } else {
                fp.end = at;
                fp.end_crossing = crossing;
            }
        }
        Ok(fp)
    }

    /// `f(p)`: the distance to the first's own part − to the second's.
    fn f(&self, p: Pt) -> f64 {
        to_polyline(p, &self.first_end) - to_polyline(p, &self.second_end)
    }

    /// The first point of a chain (its first corner where `sign · f` < 0) where `f` reaches 0.
    fn root_along(&self, chain: &[Pt], sign: f64) -> Pt {
        let g = |p: Pt| sign * self.f(p);
        for w in chain.windows(2) {
            if g(w[1]) >= 0.0 {
                let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
                for _ in 0..HALVINGS {
                    let mid = (lo + hi) / 2.0;
                    if g(lerp(w[0], w[1], mid)) >= 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                return lerp(w[0], w[1], (lo + hi) / 2.0);
            }
        }
        chain[chain.len() - 1]
    }

    fn global(&self, p: Pt) -> Pt {
        [p[0] + self.origin[0], p[1] + self.origin[1]]
    }

    fn local(&self, p: Pt) -> Pt {
        sub(p, self.origin)
    }

    /// The overlap (counter-clockwise).
    pub fn overlap(&self) -> Vec<Pt> {
        self.overlap.iter().map(|p| self.global(*p)).collect()
    }

    /// The overlap's area.
    pub fn overlap_area(&self) -> f64 {
        ring_area(&self.overlap)
    }

    /// The footprint's area (0 the first).
    pub fn area(&self, k: usize) -> f64 {
        ring_area(if k == 0 { &self.first } else { &self.second })
    }

    /// The overlap edges' places, in its order.
    pub fn kinds(&self) -> &[EdgeKind] {
        &self.kinds
    }

    /// The seam's start and end, each with whether the footprints' boundaries cross there.
    pub fn ends(&self) -> [(Pt, bool); 2] {
        [(self.global(self.start), self.start_crossing), (self.global(self.end), self.end_crossing)]
    }

    /// Whether a point lies inside the overlap (on its boundary too).
    pub fn in_overlap(&self, p: Pt) -> bool {
        let p = self.local(p);
        let n = self.overlap.len();
        (0..n).all(|i| turn(self.overlap[i], self.overlap[(i + 1) % n], p) >= 0)
    }

    /// The line across the chord at `s` (0 the start, 1 the end): its point on the chord, its direction (the chord's
    /// left normal, towards the first's run) and its stretch inside the overlap.
    fn across(&self, s: f64) -> Option<(Pt, Pt, f64, f64)> {
        let chord = sub(self.end, self.start);
        let l = len(chord);
        let nu = [-chord[1] / l, chord[0] / l];
        let c = lerp(self.start, self.end, s);
        let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
        let n = self.overlap.len();
        for i in 0..n {
            let (u, v) = (self.overlap[i], self.overlap[(i + 1) % n]);
            let e = sub(v, u);
            let (a, b) = (cross(e, sub(c, u)), cross(e, nu));
            if b > 0.0 {
                let t = -a / b;
                if t > lo {
                    lo = t;
                }
            } else if b < 0.0 {
                let t = -a / b;
                if t < hi {
                    hi = t;
                }
            } else if a < 0.0 {
                return None;
            }
        }
        (lo < hi).then_some((c, nu, lo, hi))
    }

    /// The seam's point on the line across the chord at `s`: where `f` changes sign, by bisection.
    fn root(&self, s: f64) -> Option<Pt> {
        let (c, nu, lo, hi) = self.across(s)?;
        let at = |t: f64| [c[0] + nu[0] * t, c[1] + nu[1] * t];
        // The first's run is on the left (hi): f < 0 there, > 0 at lo.
        let (mut pos, mut neg) = (lo, hi);
        if !(self.f(at(pos)) >= 0.0 && self.f(at(neg)) <= 0.0) {
            return None;
        }
        for _ in 0..HALVINGS {
            let mid = (pos + neg) / 2.0;
            if self.f(at(mid)) >= 0.0 {
                pos = mid;
            } else {
                neg = mid;
            }
        }
        Some(at((pos + neg) / 2.0))
    }

    /// The nearest elements of the two runs to a point.
    fn pair(&self, p: Pt) -> (Near, Near) {
        (nearest(&self.first_end, p), nearest(&self.second_end, p))
    }

    /// The bisector line (`n · p = c`) of a pair through point `q` of it, when it is straight.
    fn line_of(&self, pair: (Near, Near), q: Pt) -> Option<(Pt, f64)> {
        let (g1, g2) = (&self.first_end, &self.second_end);
        let edge = |g: &[Pt], k: usize| (g[k], g[k + 1]);
        let unit = |v: Pt| {
            let l = len(v);
            [v[0] / l, v[1] / l]
        };
        let normal = |a: Pt, b: Pt| {
            let d = unit(sub(b, a));
            [-d[1], d[0]]
        };
        let corner_edge = |v: Pt, a: Pt, b: Pt| -> Option<(Pt, f64)> {
            let d = unit(sub(b, a));
            (cross(d, sub(v, a)).abs() <= SAME).then(|| (d, dot(d, v)))
        };
        match pair {
            (Near::Edge(i), Near::Edge(j)) => {
                let ((a0, a1), (b0, b1)) = (edge(g1, i), edge(g2, j));
                let (na, nb) = (normal(a0, a1), normal(b0, b1));
                let sa = if dot(na, sub(q, a0)) < 0.0 { -1.0 } else { 1.0 };
                let sb = if dot(nb, sub(q, b0)) < 0.0 { -1.0 } else { 1.0 };
                let n = [sa * na[0] - sb * nb[0], sa * na[1] - sb * nb[1]];
                (len(n) > 1e-12).then(|| (n, sa * dot(na, a0) - sb * dot(nb, b0)))
            }
            (Near::Corner(i), Near::Corner(j)) => {
                let (v, w) = (g1[i], g2[j]);
                let n = sub(w, v);
                (len(n) > SAME).then(|| (n, (dot(w, w) - dot(v, v)) / 2.0))
            }
            (Near::Corner(i), Near::Edge(j)) => {
                let (b0, b1) = edge(g2, j);
                corner_edge(g1[i], b0, b1)
            }
            (Near::Edge(i), Near::Corner(j)) => {
                let (a0, a1) = edge(g1, i);
                corner_edge(g2[j], a0, a1)
            }
        }
    }

    /// The chord parameter of a point (0 the start, 1 the end).
    fn param(&self, p: Pt) -> f64 {
        let chord = sub(self.end, self.start);
        dot(sub(p, self.start), chord) / dot(chord, chord)
    }

    /// Between two roots of different pairs: the turns (and the roots of a curved piece between) in order.
    #[allow(clippy::too_many_arguments)]
    fn resolve(&self, sa: f64, qa: Pt, pa: (Near, Near), sb: f64, qb: Pt, pb: (Near, Near), depth: u32, out: &mut Vec<(Pt, bool)>) {
        if let (Some(la), Some(lb)) = (self.line_of(pa, qa), self.line_of(pb, qb))
            && let Some(x) = meet(la, lb)
        {
            let s = self.param(x);
            if s >= sa - 1e-9 && s <= sb + 1e-9 && self.in_overlap(self.global(x)) && self.f(x).abs() <= 1e-6 {
                out.push((x, true));
                return;
            }
        }
        let mid = (sa + sb) / 2.0;
        let Some(qm) = (if depth < 60 && sb - sa > 1e-12 { self.root(mid) } else { None }) else {
            // Found to a double's precision without a straight turn: the root there.
            out.push((lerp(qa, qb, 0.5), false));
            return;
        };
        let pm = self.pair(qm);
        if pm == pa {
            self.resolve(mid, qm, pm, sb, qb, pb, depth + 1, out);
        } else if pm == pb {
            self.resolve(sa, qa, pa, mid, qm, pm, depth + 1, out);
        } else {
            // A third piece between: its turns on both sides; its own root only when it is curved.
            self.resolve(sa, qa, pa, mid, qm, pm, depth + 1, out);
            if self.line_of(pm, qm).is_none() {
                out.push((qm, false));
            }
            self.resolve(mid, qm, pm, sb, qb, pb, depth + 1, out);
        }
    }

    /// The Voronoi seam (docs/adr/4700) from the start to the end: its turns exactly, a curved piece's points on the
    /// lines across the chord every `step` metres.
    pub fn voronoi(&self, step: f64) -> Vec<Pt> {
        let l = len(sub(self.end, self.start));
        let q = l / step;
        let n = js_max(if (q - q.round_ties_even()).abs() <= 1e-9 { q.round_ties_even() } else { q.ceil() }, 2.0) as usize;
        // Every root with its pair; the first and last a hair inside the ends, for the pieces they start and end.
        let mut stations: Vec<(f64, Pt, (Near, Near))> = Vec::with_capacity(n + 1);
        let hair = 1e-6;
        for k in 0..=n {
            let s = if k == 0 { hair } else if k == n { 1.0 - hair } else { k as f64 / n as f64 };
            if let Some(p) = self.root(s) {
                stations.push((s, p, self.pair(p)));
            }
        }
        // Each item: a point and whether it is a straight turn (kept) or a curved piece's root (kept when curved).
        let mut items: Vec<(Pt, bool)> = Vec::new();
        for w in stations.windows(2) {
            let ((sa, qa, pa), (sb, qb, pb)) = (w[0], w[1]);
            if pa != pb {
                self.resolve(sa, qa, pa, sb, qb, pb, 0, &mut items);
            }
            if self.line_of(pb, qb).is_none() && sb < 1.0 - hair {
                items.push((qb, false));
            }
        }
        let mut seam = vec![self.start];
        for (p, _) in items {
            seam.push(p);
        }
        seam.push(self.end);
        deduped_open(&seam).into_iter().map(|p| self.global(p)).collect()
    }

    /// The two polygons' rings (counter-clockwise) for a seam from the start to the end: the first's boundary from
    /// the end to the start, then the seam; the second's from the start to the end, then the seam back.
    pub fn rings(&self, seam: &[Pt]) -> [Vec<Pt>; 2] {
        let s: Vec<Pt> = seam.iter().map(|p| self.local(*p)).collect();
        let inner = if s.len() > 2 { &s[1..s.len() - 1] } else { &[][..] };
        let arc = |ring: &[Pt], from: Pt, to: Pt| -> Vec<Pt> {
            let (a, b) = (position(ring, from), position(ring, to));
            let n = ring.len() as f64;
            let span = if b > a { b - a } else { b - a + n };
            let mut out = vec![from];
            let mut k = a.floor() + 1.0;
            while k < a + span - 1e-12 {
                let v = ring[(k as usize) % ring.len()];
                out.push(v);
                k += 1.0;
            }
            out.push(to);
            out
        };
        let mut first = arc(&self.first, self.end, self.start);
        first.extend_from_slice(inner);
        let mut second = arc(&self.second, self.start, self.end);
        second.extend(inner.iter().rev());
        [deduped(&first), deduped(&second)].map(|r| r.into_iter().map(|p| self.global(p)).collect())
    }
}

/// Neighbours within `SAME` merged, an open line.
fn deduped_open(r: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(r.len());
    for &p in r {
        if out.last().is_none_or(|q| len(sub(p, *q)) > SAME) {
            out.push(p);
        }
    }
    out
}

/// A point's place on a ring: its edge's index plus the fraction along it (a corner: a whole number).
fn position(ring: &[Pt], p: Pt) -> f64 {
    let n = ring.len();
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if len(sub(p, a)) <= SAME {
            return i as f64;
        }
        let d = to_segment(p, a, b);
        if d < best.0 {
            let ab = sub(b, a);
            let t = (dot(sub(p, a), ab) / dot(ab, ab)).clamp(0.0, 1.0);
            best = (d, if len(sub(p, b)) <= SAME { ((i + 1) % n) as f64 } else { i as f64 + t });
        }
    }
    best.1
}

/// The nearest element of a run: an edge when the foot falls inside it, else a corner; equals to an edge, then the
/// first.
fn nearest(g: &[Pt], p: Pt) -> Near {
    let mut best: Option<(f64, Near)> = None;
    for k in 0..g.len().saturating_sub(1) {
        let (a, b) = (g[k], g[k + 1]);
        let ab = sub(b, a);
        let t = dot(sub(p, a), ab) / dot(ab, ab);
        let (d, near) = if t <= 0.0 {
            (len(sub(p, a)), Near::Corner(k))
        } else if t >= 1.0 {
            (len(sub(p, b)), Near::Corner(k + 1))
        } else {
            (cross(ab, sub(p, a)).abs() / len(ab), Near::Edge(k))
        };
        let better = match best {
            None => true,
            Some((m, was)) => {
                let tie = (d - m).abs() <= 1e-12 * js_max(1.0, m);
                (d < m && !tie) || (tie && matches!((near, was), (Near::Edge(_), Near::Corner(_))))
            }
        };
        if better {
            best = Some((d, near));
        }
    }
    best.map_or(Near::Corner(0), |b| b.1)
}

/// Where two lines (`n · p = c`) meet; none when parallel.
fn meet(a: (Pt, f64), b: (Pt, f64)) -> Option<Pt> {
    let ((n1, c1), (n2, c2)) = (a, b);
    let det = cross(n1, n2);
    (det.abs() > 1e-12 * len(n1) * len(n2)).then(|| [(c1 * n2[1] - c2 * n1[1]) / det, (n1[0] * c2 - n2[0] * c1) / det])
}

/// A cell's queue entry: smaller key first, then the smaller index.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Entry {
    key: f64,
    at: usize,
}

impl Eq for Entry {}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed: BinaryHeap pops the greatest.
        other.key.total_cmp(&self.key).then_with(|| other.at.cmp(&self.at))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The eight moves (row, column), in the order they are tried.
const MOVES: [(i64, i64); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

/// A path through the grid: its cells from `from` to `to`, and its largest difference.
#[derive(Clone, Debug, PartialEq)]
pub struct GridPath {
    pub cells: Vec<usize>,
    pub bottleneck: f64,
}

/// The least-difference path (docs/adr/4700) through a `w` × `h` grid of differences (row by row; NaN: a cell not
/// searched) from cell `from` to cell `to`: the path whose largest difference is the smallest, then among the cells
/// not above it the path of least cost. None when no path joins them.
pub fn grid_path(w: usize, h: usize, diff: &[f64], from: usize, to: usize) -> Option<GridPath> {
    let searched = |i: usize| !diff[i].is_nan();
    if diff.len() != w * h || from >= diff.len() || to >= diff.len() || !searched(from) || !searched(to) {
        return None;
    }
    let neighbours = |i: usize| {
        let (r, c) = ((i / w) as i64, (i % w) as i64);
        MOVES.iter().filter_map(move |&(dr, dc)| {
            let (nr, nc) = (r + dr, c + dc);
            (nr >= 0 && nc >= 0 && nr < h as i64 && nc < w as i64)
                .then(|| ((nr as usize) * w + nc as usize, if dr != 0 && dc != 0 { std::f64::consts::SQRT_2 } else { 1.0 }))
        })
    };
    // The bottleneck: Dijkstra on the largest difference so far.
    let mut best = vec![f64::INFINITY; diff.len()];
    let mut heap = BinaryHeap::new();
    best[from] = diff[from];
    heap.push(Entry { key: best[from], at: from });
    while let Some(Entry { key, at }) = heap.pop() {
        if key > best[at] {
            continue;
        }
        if at == to {
            break;
        }
        for (q, _) in neighbours(at) {
            if !searched(q) {
                continue;
            }
            let cand = js_max(key, diff[q]);
            if cand < best[q] {
                best[q] = cand;
                heap.push(Entry { key: cand, at: q });
            }
        }
    }
    let bottleneck = best[to];
    if !bottleneck.is_finite() {
        return None;
    }
    // The least cost among the cells not above it.
    let mut cost = vec![f64::INFINITY; diff.len()];
    let mut prev = vec![usize::MAX; diff.len()];
    let mut heap = BinaryHeap::new();
    cost[from] = 0.0;
    heap.push(Entry { key: 0.0, at: from });
    while let Some(Entry { key, at }) = heap.pop() {
        if key > cost[at] {
            continue;
        }
        if at == to {
            break;
        }
        for (q, step) in neighbours(at) {
            if !searched(q) || diff[q] > bottleneck {
                continue;
            }
            let cand = key + step * (1.0 + (diff[at] + diff[q]) / 2.0);
            if cand < cost[q] {
                cost[q] = cand;
                prev[q] = at;
                heap.push(Entry { key: cand, at: q });
            }
        }
    }
    let mut cells = vec![to];
    let mut at = to;
    while at != from {
        at = prev[at];
        if at == usize::MAX {
            return None;
        }
        cells.push(at);
    }
    cells.reverse();
    Some(GridPath { cells, bottleneck })
}

/// A grid path's turning cells: the first, every cell where the move changes direction, the last.
pub fn turns(cells: &[usize], w: usize) -> Vec<usize> {
    if cells.len() <= 2 {
        return cells.to_vec();
    }
    let step = |a: usize, b: usize| ((b / w) as i64 - (a / w) as i64, (b % w) as i64 - (a % w) as i64);
    let mut out = vec![cells[0]];
    for k in 1..cells.len() - 1 {
        if step(cells[k - 1], cells[k]) != step(cells[k], cells[k + 1]) {
            out.push(cells[k]);
        }
    }
    out.push(cells[cells.len() - 1]);
    out
}

/// Whether a ring is simple: no two edges meet but neighbours at their shared corner.
pub fn simple(ring: &[Pt]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let seg = |i: usize| (ring[i], ring[(i + 1) % n]);
    for i in 0..n {
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let ((a, b), (c, d)) = (seg(i), seg(j));
            let (o1, o2, o3, o4) = (turn(a, b, c), turn(a, b, d), turn(c, d, a), turn(c, d, b));
            if o1 * o2 < 0 && o3 * o4 < 0 {
                return false;
            }
            let touches = (o1 == 0 && to_segment(c, a, b) <= SAME)
                || (o2 == 0 && to_segment(d, a, b) <= SAME)
                || (o3 == 0 && to_segment(a, c, d) <= SAME)
                || (o4 == 0 && to_segment(b, c, d) <= SAME);
            if touches {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
#[path = "seamlines_tests.rs"]
mod tests;
