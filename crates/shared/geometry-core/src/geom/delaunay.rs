//! Delaunay triangulation (docs/adr/0232 §4): the sweep-hull method
//! (Sinclair's s-hull, in the order Mapbox's Delaunator takes it), its
//! turns and in-circle tests exact (`predicates`), so a triangulation never
//! depends on rounding. Cocircular points are not flipped: a regular grid's
//! triangulation is not unique, and this one follows the input's order, the
//! same on every target. Then a point's triangle found by walking
//! (`locate`), the points round a point (`star`) and the hole its removal
//! leaves filled again (`fill_star`): TIN'den raster, Doğal komşu and their
//! cross-validation in the raster core, and CIVIL-02's TIN.
//!
//! Triangles are counter-clockwise. Half-edge e of triangle e / 3 runs from
//! `triangles[e]` to `triangles[next(e)]`; its twin runs the other way in the
//! triangle across, or is `NONE` on the convex hull.

use crate::jsmath::{js_max, js_min};
use crate::predicates::{incircle, orient2d};
use crate::vec2::Vec2;

/// No half-edge: a hull edge's twin.
pub const NONE: u32 = u32::MAX;

/// The most points a triangulation takes (its half-edges stay below `NONE`).
pub const MOST_POINTS: usize = 400_000_000;

/// Why a set of points has no triangulation.
pub const TOO_FEW: &str = "Üçgenleme için doğru üzerinde olmayan en az üç nokta gerekir.";

/// A Delaunay triangulation of some points.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Delaunay {
    /// Three point indices a triangle, counter-clockwise.
    pub triangles: Vec<u32>,
    /// Each half-edge's twin, `NONE` on the hull.
    pub halfedges: Vec<u32>,
    /// The convex hull's points, counter-clockwise.
    pub hull: Vec<u32>,
    /// Points left out: equal to an earlier one in the sweep's order, or (on
    /// input no exact test separates) not outside the hull when their turn came.
    pub skipped: Vec<u32>,
}

/// The half-edge after `e` in its triangle.
#[inline]
pub fn next(e: u32) -> u32 {
    if e % 3 == 2 { e - 2 } else { e + 1 }
}

/// The half-edge before `e` in its triangle.
#[inline]
pub fn prev(e: u32) -> u32 {
    if e.is_multiple_of(3) { e + 2 } else { e - 1 }
}

fn dist2(a: Vec2, b: Vec2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// The square of the circumradius of a, b, c (infinite when they are collinear).
fn circumradius2(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (ex, ey) = (c.x - a.x, c.y - a.y);
    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / (dx * ey - dy * ex);
    let x = (ey * bl - dy * cl) * d;
    let y = (dx * cl - ex * bl) * d;
    let r = x * x + y * y;
    if r.is_finite() { r } else { f64::INFINITY }
}

/// The circumcentre of a, b, c (non-finite when they are collinear).
pub fn circumcenter(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (ex, ey) = (c.x - a.x, c.y - a.y);
    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / (dx * ey - dy * ex);
    Vec2::new(a.x + (ey * bl - dy * cl) * d, a.y + (dx * cl - ex * bl) * d)
}

/// A monotone stand-in for the angle of (dx, dy), in [0, 1).
fn pseudo_angle(dx: f64, dy: f64) -> f64 {
    let p = dx / (dx.abs() + dy.abs());
    (if dy > 0.0 { 3.0 - p } else { 1.0 + p }) / 4.0
}

/// The sweep's state.
struct Sweep<'p> {
    points: &'p [Vec2],
    triangles: Vec<u32>,
    halfedges: Vec<u32>,
    hull_start: u32,
    hull_prev: Vec<u32>,
    hull_next: Vec<u32>,
    hull_tri: Vec<u32>,
    stack: Vec<u32>,
}

impl Sweep<'_> {
    fn p(&self, i: u32) -> Vec2 {
        self.points[i as usize]
    }

    fn link(&mut self, a: u32, b: u32) {
        self.halfedges[a as usize] = b;
        if b != NONE {
            self.halfedges[b as usize] = a;
        }
    }

    /// A triangle i0, i1, i2 whose half-edges' twins are a, b, c; its first half-edge.
    fn add_triangle(&mut self, i0: u32, i1: u32, i2: u32, a: u32, b: u32, c: u32) -> u32 {
        let t = self.triangles.len() as u32;
        self.triangles.extend_from_slice(&[i0, i1, i2]);
        self.halfedges.extend_from_slice(&[NONE, NONE, NONE]);
        self.link(t, a);
        self.link(t + 1, b);
        self.link(t + 2, c);
        t
    }

    /// Flips `a` and the edges after it while the triangles across fail the
    /// empty-circle test; the half-edge that then holds `a`'s triangle's
    /// edge before it (Delaunator's `_legalize`, its stack unbounded).
    fn legalize(&mut self, mut a: u32) -> u32 {
        self.stack.clear();
        let mut ar;
        loop {
            let b = self.halfedges[a as usize];
            let a0 = a - a % 3;
            ar = a0 + (a + 2) % 3;
            if b == NONE {
                match self.stack.pop() {
                    Some(x) => {
                        a = x;
                        continue;
                    }
                    None => break,
                }
            }
            let b0 = b - b % 3;
            let al = a0 + (a + 1) % 3;
            let bl = b0 + (b + 2) % 3;
            let p0 = self.triangles[ar as usize];
            let pr = self.triangles[a as usize];
            let pl = self.triangles[al as usize];
            let p1 = self.triangles[bl as usize];
            if incircle(self.p(p0), self.p(pr), self.p(pl), self.p(p1)) > 0.0 {
                self.triangles[a as usize] = p1;
                self.triangles[b as usize] = p0;
                let hbl = self.halfedges[bl as usize];
                // An edge flipped on the other side of the hull: its record moves.
                if hbl == NONE {
                    let mut e = self.hull_start;
                    for _ in 0..self.points.len() {
                        if self.hull_tri[e as usize] == bl {
                            self.hull_tri[e as usize] = a;
                            break;
                        }
                        e = self.hull_prev[e as usize];
                        if e == self.hull_start {
                            break;
                        }
                    }
                }
                let har = self.halfedges[ar as usize];
                self.link(a, hbl);
                self.link(b, har);
                self.link(ar, bl);
                self.stack.push(b0 + (b + 1) % 3);
            } else {
                match self.stack.pop() {
                    Some(x) => a = x,
                    None => break,
                }
            }
        }
        ar
    }
}

/// The Delaunay triangulation of `points` (finite, at most [`MOST_POINTS`]).
/// Equal points are taken once (the others in `skipped`). Fewer than three
/// distinct points, or all on a line: [`TOO_FEW`].
pub fn triangulate(points: &[Vec2]) -> Result<Delaunay, String> {
    let n = points.len();
    if n > MOST_POINTS {
        return Err(format!(
            "Üçgenleme en çok {MOST_POINTS} nokta alır; {n} nokta verildi."
        ));
    }
    if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Err("Üçgenlemenin noktalarının koordinatları sayı olmalı.".into());
    }
    if n < 3 {
        return Err(TOO_FEW.into());
    }
    let (mut minx, mut miny, mut maxx, mut maxy) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in points {
        minx = js_min(minx, p.x);
        miny = js_min(miny, p.y);
        maxx = js_max(maxx, p.x);
        maxy = js_max(maxy, p.y);
    }
    let c = Vec2::new((minx + maxx) / 2.0, (miny + maxy) / 2.0);

    // The seed: the point nearest the box's centre, the point nearest it, and
    // the point that makes the smallest circle with them.
    let mut i0 = 0usize;
    let mut best = f64::INFINITY;
    for (i, p) in points.iter().enumerate() {
        let d = dist2(*p, c);
        if d < best {
            best = d;
            i0 = i;
        }
    }
    let p0 = points[i0];
    let mut i1 = usize::MAX;
    best = f64::INFINITY;
    for (i, p) in points.iter().enumerate() {
        let d = dist2(*p, p0);
        if i != i0 && d > 0.0 && d < best {
            best = d;
            i1 = i;
        }
    }
    if i1 == usize::MAX {
        return Err(TOO_FEW.into());
    }
    let p1 = points[i1];
    let mut i2 = usize::MAX;
    let mut least = f64::INFINITY;
    for (i, p) in points.iter().enumerate() {
        if i == i0 || i == i1 || orient2d(p0, p1, *p) == 0.0 {
            continue;
        }
        let r = circumradius2(p0, p1, *p);
        if i2 == usize::MAX || r < least {
            least = r;
            i2 = i;
        }
    }
    if i2 == usize::MAX {
        return Err(TOO_FEW.into());
    }
    if orient2d(p0, p1, points[i2]) < 0.0 {
        std::mem::swap(&mut i1, &mut i2);
    }
    let (i0, i1, i2) = (i0 as u32, i1 as u32, i2 as u32);
    let center = circumcenter(
        points[i0 as usize],
        points[i1 as usize],
        points[i2 as usize],
    );

    // The others by their distance from the seed's circumcentre.
    let dists: Vec<f64> = points.iter().map(|p| dist2(*p, center)).collect();
    let mut ids: Vec<u32> = (0..n as u32).collect();
    ids.sort_by(|&a, &b| {
        dists[a as usize]
            .total_cmp(&dists[b as usize])
            .then(a.cmp(&b))
    });

    let hash_size = ((n as f64).sqrt().ceil() as usize).max(1);
    let key = |p: Vec2| -> usize {
        let k = (pseudo_angle(p.x - center.x, p.y - center.y) * hash_size as f64).floor();
        (js_max(k, 0.0) as usize) % hash_size
    };
    let mut hull_hash = vec![NONE; hash_size];
    let most_triangles = (2 * n).saturating_sub(5).max(1);
    let mut s = Sweep {
        points,
        triangles: Vec::with_capacity(most_triangles * 3),
        halfedges: Vec::with_capacity(most_triangles * 3),
        hull_start: i0,
        hull_prev: vec![0; n],
        hull_next: vec![0; n],
        hull_tri: vec![0; n],
        stack: Vec::new(),
    };
    s.hull_next[i0 as usize] = i1;
    s.hull_prev[i2 as usize] = i1;
    s.hull_next[i1 as usize] = i2;
    s.hull_prev[i0 as usize] = i2;
    s.hull_next[i2 as usize] = i0;
    s.hull_prev[i1 as usize] = i0;
    s.hull_tri[i0 as usize] = 0;
    s.hull_tri[i1 as usize] = 1;
    s.hull_tri[i2 as usize] = 2;
    for i in [i0, i1, i2] {
        hull_hash[key(points[i as usize])] = i;
    }
    s.add_triangle(i0, i1, i2, NONE, NONE, NONE);

    let mut skipped = Vec::new();
    let mut last: Option<Vec2> = None;
    for &i in &ids {
        let p = points[i as usize];
        if last == Some(p) {
            skipped.push(i);
            continue;
        }
        last = Some(p);
        if i == i0 || i == i1 || i == i2 {
            continue;
        }
        if [i0, i1, i2].iter().any(|&j| points[j as usize] == p) {
            skipped.push(i);
            continue;
        }
        // A hull edge the point sees: from the hull point nearest its angle.
        let k = key(p);
        let mut start = NONE;
        for j in 0..hash_size {
            start = hull_hash[(k + j) % hash_size];
            if start != NONE && start != s.hull_next[start as usize] {
                break;
            }
        }
        if start == NONE {
            skipped.push(i);
            continue;
        }
        start = s.hull_prev[start as usize];
        let mut e = start;
        loop {
            let q = s.hull_next[e as usize];
            if orient2d(s.p(e), s.p(q), p) < 0.0 {
                break;
            }
            e = q;
            if e == start {
                e = NONE;
                break;
            }
        }
        if e == NONE {
            skipped.push(i);
            continue;
        }

        // The first triangle, then forward and back along the hull while the point sees its edges.
        let first = s.add_triangle(
            e,
            i,
            s.hull_next[e as usize],
            NONE,
            NONE,
            s.hull_tri[e as usize],
        );
        s.hull_tri[i as usize] = s.legalize(first + 2);
        s.hull_tri[e as usize] = first;
        let mut nn = s.hull_next[e as usize];
        loop {
            let q = s.hull_next[nn as usize];
            if !(orient2d(s.p(nn), s.p(q), p) < 0.0) {
                break;
            }
            let t = s.add_triangle(
                nn,
                i,
                q,
                s.hull_tri[i as usize],
                NONE,
                s.hull_tri[nn as usize],
            );
            s.hull_tri[i as usize] = s.legalize(t + 2);
            s.hull_next[nn as usize] = nn;
            nn = q;
        }
        if e == start {
            loop {
                let q = s.hull_prev[e as usize];
                if !(orient2d(s.p(q), s.p(e), p) < 0.0) {
                    break;
                }
                let t = s.add_triangle(
                    q,
                    i,
                    e,
                    NONE,
                    s.hull_tri[e as usize],
                    s.hull_tri[q as usize],
                );
                s.legalize(t + 2);
                s.hull_tri[q as usize] = t;
                s.hull_next[e as usize] = e;
                e = q;
            }
        }
        s.hull_start = e;
        s.hull_prev[i as usize] = e;
        s.hull_next[e as usize] = i;
        s.hull_prev[nn as usize] = i;
        s.hull_next[i as usize] = nn;
        hull_hash[key(p)] = i;
        hull_hash[key(s.p(e))] = e;
    }

    let mut hull = Vec::new();
    let mut e = s.hull_start;
    for _ in 0..n {
        hull.push(e);
        e = s.hull_next[e as usize];
        if e == s.hull_start {
            break;
        }
    }
    skipped.sort_unstable();
    Ok(Delaunay {
        triangles: s.triangles,
        halfedges: s.halfedges,
        hull,
        skipped,
    })
}

/// Where a point lies in a triangulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Located {
    /// In this triangle or on its edges.
    Inside(u32),
    /// Outside the convex hull, beyond this hull half-edge.
    Outside(u32),
}

impl Delaunay {
    /// How many triangles.
    pub fn len(&self) -> usize {
        self.triangles.len() / 3
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Triangle t's three points.
    pub fn corners(&self, t: u32) -> [u32; 3] {
        let i = 3 * t as usize;
        [
            self.triangles[i],
            self.triangles[i + 1],
            self.triangles[i + 2],
        ]
    }

    /// The triangle that holds `q`, walked to from triangle `start`: across
    /// the first edge (in the triangle's order) that has `q` on its right.
    /// A Delaunay triangulation has no cycle of such steps (Edelsbrunner),
    /// so the walk ends; on a hull edge with `q` on its right, `q` is outside.
    pub fn locate(&self, points: &[Vec2], q: Vec2, start: u32) -> Located {
        let mut t = if (start as usize) < self.len() {
            start
        } else {
            0
        };
        let most = self.len() + 8;
        'walk: for _ in 0..most {
            for k in 0..3 {
                let e = 3 * t + k;
                let a = points[self.triangles[e as usize] as usize];
                let b = points[self.triangles[next(e) as usize] as usize];
                if orient2d(a, b, q) < 0.0 {
                    let tw = self.halfedges[e as usize];
                    if tw == NONE {
                        return Located::Outside(e);
                    }
                    t = tw / 3;
                    continue 'walk;
                }
            }
            return Located::Inside(t);
        }
        // Never on a Delaunay triangulation; every triangle tried in turn.
        for t in 0..self.len() as u32 {
            let [a, b, c] = self.corners(t).map(|i| points[i as usize]);
            if orient2d(a, b, q) >= 0.0 && orient2d(b, c, q) >= 0.0 && orient2d(c, a, q) >= 0.0 {
                return Located::Inside(t);
            }
        }
        Located::Outside(self.hull_edge_of(self.hull.first().copied().unwrap_or(0)))
    }

    /// A hull half-edge starting at point `v` (or `NONE`).
    fn hull_edge_of(&self, v: u32) -> u32 {
        (0..self.halfedges.len() as u32)
            .find(|&e| self.halfedges[e as usize] == NONE && self.triangles[e as usize] == v)
            .unwrap_or(NONE)
    }

    /// For each of the `n` points a half-edge leaving it: on the hull the
    /// hull edge leaving it (so a turn round it starts there); `NONE` for a
    /// point the triangulation left out.
    pub fn out_edges(&self, n: usize) -> Vec<u32> {
        let mut out = vec![NONE; n];
        for (e, &v) in self.triangles.iter().enumerate() {
            if out[v as usize] == NONE {
                out[v as usize] = e as u32;
            }
        }
        for (e, &tw) in self.halfedges.iter().enumerate() {
            if tw == NONE {
                out[self.triangles[e] as usize] = e as u32;
            }
        }
        out
    }

    /// The points joined to point `v` counter-clockwise round it, and
    /// whether `v` is on the hull (its neighbours then run from the hull
    /// point after it to the one before it). `out` is [`Delaunay::out_edges`].
    pub fn star(&self, out: &[u32], v: u32) -> (Vec<u32>, bool) {
        let mut ring = Vec::new();
        let start = out.get(v as usize).copied().unwrap_or(NONE);
        if start == NONE {
            return (ring, false);
        }
        let mut h = start;
        for _ in 0..self.halfedges.len() {
            ring.push(self.triangles[next(h) as usize]);
            // The edge before h comes into v; its twin is the next one leaving v.
            let back = prev(h);
            let tw = self.halfedges[back as usize];
            if tw == NONE {
                ring.push(self.triangles[back as usize]);
                return (ring, true);
            }
            h = tw;
            if h == start {
                return (ring, false);
            }
        }
        (ring, false)
    }
}

/// The hole a point's removal leaves filled again: the Delaunay triangles,
/// counter-clockwise, of the star-shaped `ring` (the point's neighbours
/// counter-clockwise; at least three). Ears whose circle holds no other
/// point of the ring are cut one by one, the first in the ring's order
/// (Devillers 1999: such an ear always exists in a removal's hole; the
/// rings are short, so they are scanned).
pub fn fill_star(points: &[Vec2], ring: &[u32]) -> Vec<[u32; 3]> {
    let mut poly: Vec<u32> = ring.to_vec();
    let mut out = Vec::new();
    while poly.len() > 3 {
        let m = poly.len();
        let mut cut = None;
        for i in 0..m {
            let (a, b, c) = (poly[(i + m - 1) % m], poly[i], poly[(i + 1) % m]);
            let (pa, pb, pc) = (points[a as usize], points[b as usize], points[c as usize]);
            if !(orient2d(pa, pb, pc) > 0.0) {
                continue;
            }
            let empty = poly.iter().all(|&o| {
                o == a || o == b || o == c || !(incircle(pa, pb, pc, points[o as usize]) > 0.0)
            });
            if empty {
                cut = Some(i);
                break;
            }
        }
        let Some(i) = cut else {
            break;
        };
        let m = poly.len();
        out.push([poly[(i + m - 1) % m], poly[i], poly[(i + 1) % m]]);
        poly.remove(i);
    }
    if let [a, b, c] = poly[..]
        && orient2d(points[a as usize], points[b as usize], points[c as usize]) > 0.0
    {
        out.push([a, b, c]);
    }
    out
}
