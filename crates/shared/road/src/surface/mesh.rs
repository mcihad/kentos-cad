//! The triangulation the surface is made on: the geometry core's Delaunay half-edges (`geom::delaunay`), each
//! half-edge marked when it is a constraint's, and the flips that put the constraints in (Sloan 1993) and make the
//! rest locally Delaunay again (Lawson). Every turn and in-circle test is exact (`predicates`).

use std::collections::VecDeque;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::delaunay::{Delaunay, Located, NONE, next, prev};
use kentos_geometry_core::predicates::{incircle, orient2d};

/// Where a piece's straight line meets the triangulation from its first corner.
pub(super) enum Walk {
    /// Its edges, as corner pairs, in the order the line crosses them (none: the edge is already there).
    Edges(Vec<(u32, u32)>),
    /// A corner lies on the piece, strictly between its ends: the piece is split there.
    Through(u32),
    /// It crosses a constrained edge, about here.
    Crossed([f64; 2]),
}

#[derive(Clone, Debug)]
pub(super) struct Mesh {
    d: Delaunay,
    constrained: Vec<bool>,
    /// For each corner a half-edge leaving it.
    out: Vec<u32>,
}

fn side(x: f64) -> i8 {
    if x > 0.0 {
        1
    } else if x < 0.0 {
        -1
    } else {
        0
    }
}

/// Where the lines p–q and r–s cross, for a message only (constructed, not a decision).
fn crossing(p: Vec2, q: Vec2, r: Vec2, s: Vec2) -> [f64; 2] {
    let d = (q.x - p.x) * (s.y - r.y) - (q.y - p.y) * (s.x - r.x);
    if d == 0.0 {
        return [r.x, r.y];
    }
    let t = ((r.x - p.x) * (s.y - r.y) - (r.y - p.y) * (s.x - r.x)) / d;
    [p.x + t * (q.x - p.x), p.y + t * (q.y - p.y)]
}

impl Mesh {
    pub(super) fn new(d: Delaunay, n: usize) -> Self {
        let out = d.out_edges(n);
        let constrained = vec![false; d.halfedges.len()];
        Self { d, constrained, out }
    }

    pub(super) fn len(&self) -> usize {
        self.d.len()
    }

    pub(super) fn corners(&self, t: u32) -> [u32; 3] {
        self.d.corners(t)
    }

    pub(super) fn twin(&self, e: u32) -> u32 {
        self.d.halfedges[e as usize]
    }

    pub(super) fn is_constrained(&self, e: u32) -> bool {
        self.constrained[e as usize]
    }

    fn from(&self, e: u32) -> u32 {
        self.d.triangles[e as usize]
    }

    fn to(&self, e: u32) -> u32 {
        self.d.triangles[next(e) as usize]
    }

    pub(super) fn edge_length(&self, xy: &[Vec2], e: u32) -> f64 {
        let (a, b) = (xy[self.from(e) as usize], xy[self.to(e) as usize]);
        (b.x - a.x).hypot(b.y - a.y)
    }

    /// The constrained edges as corner pairs, the lower index first, each once.
    pub(super) fn constrained_pairs(&self) -> Vec<[u32; 2]> {
        let mut out = Vec::new();
        for e in 0..self.constrained.len() as u32 {
            let tw = self.twin(e);
            if self.constrained[e as usize] && (tw == NONE || e < tw) {
                let (a, b) = (self.from(e), self.to(e));
                out.push([a.min(b), a.max(b)]);
            }
        }
        out
    }

    /// The half-edges leaving corner `v`, counter-clockwise from the first; a hull corner's run from the hull edge
    /// leaving it.
    fn leaving(&self, v: u32) -> Vec<u32> {
        let start = self.out[v as usize];
        let mut all = Vec::new();
        if start == NONE {
            return all;
        }
        // Clockwise to the hull edge (or round to the start): the twin's next leaves v too.
        let mut e = start;
        loop {
            let tw = self.twin(e);
            if tw == NONE {
                break;
            }
            let back = next(tw);
            if back == start {
                break;
            }
            e = back;
        }
        let first = e;
        loop {
            all.push(e);
            let tw = self.twin(prev(e));
            if tw == NONE || tw == first {
                break;
            }
            e = tw;
        }
        all
    }

    /// The half-edge from `a` to `b` or from `b` to `a`.
    fn edge(&self, a: u32, b: u32) -> Option<u32> {
        self.leaving(a)
            .into_iter()
            .find(|&e| self.to(e) == b)
            .or_else(|| self.leaving(b).into_iter().find(|&e| self.to(e) == a))
    }

    fn constrain(&mut self, e: u32) {
        self.constrained[e as usize] = true;
        let tw = self.twin(e);
        if tw != NONE {
            self.constrained[tw as usize] = true;
        }
    }

    fn link(&mut self, a: u32, b: u32) {
        self.d.halfedges[a as usize] = b;
        if b != NONE {
            self.d.halfedges[b as usize] = a;
        }
    }

    /// Flips the edge of half-edge `a` (Delaunator's swap): triangles (pr, pl, p0) and (pl, pr, p1) become
    /// (p1, pl, p0) and (p0, pr, p1); the new edge is half-edges `ar` and `bl`.
    fn flip(&mut self, a: u32) -> (u32, u32) {
        let b = self.twin(a);
        let a0 = a - a % 3;
        let b0 = b - b % 3;
        let al = a0 + (a + 1) % 3;
        let ar = a0 + (a + 2) % 3;
        let bl = b0 + (b + 2) % 3;
        let br = b0 + (b + 1) % 3;
        let p0 = self.d.triangles[ar as usize];
        let pr = self.d.triangles[a as usize];
        let pl = self.d.triangles[al as usize];
        let p1 = self.d.triangles[bl as usize];
        let (c_bl, c_ar) = (
            self.constrained[bl as usize],
            self.constrained[ar as usize],
        );
        self.d.triangles[a as usize] = p1;
        self.d.triangles[b as usize] = p0;
        let hbl = self.twin(bl);
        let har = self.twin(ar);
        self.link(a, hbl);
        self.link(b, har);
        self.link(ar, bl);
        self.constrained[a as usize] = c_bl;
        self.constrained[b as usize] = c_ar;
        self.constrained[ar as usize] = false;
        self.constrained[bl as usize] = false;
        self.out[p1 as usize] = a;
        self.out[pl as usize] = al;
        self.out[p0 as usize] = b;
        self.out[pr as usize] = br;
        (ar, bl)
    }

    /// The corners across the edge of half-edge `e`: the far corners of its two triangles.
    fn opposite(&self, e: u32) -> (u32, u32) {
        (
            self.d.triangles[prev(e) as usize],
            self.d.triangles[prev(self.twin(e)) as usize],
        )
    }

    /// The piece a–b's way through the triangulation.
    pub(super) fn walk(&self, xy: &[Vec2], a: u32, b: u32) -> Walk {
        let (u, v) = (xy[a as usize], xy[b as usize]);
        let on = |w: u32| {
            let p = xy[w as usize];
            orient2d(u, v, p) == 0.0 && (p.x - u.x) * (v.x - u.x) + (p.y - u.y) * (v.y - u.y) > 0.0
        };
        let mut first = None;
        for e in self.leaving(a) {
            let (w, x) = (self.to(e), self.d.triangles[prev(e) as usize]);
            if w == b || x == b {
                return Walk::Edges(Vec::new());
            }
            if on(w) {
                return Walk::Through(w);
            }
            if on(x) {
                return Walk::Through(x);
            }
            let (pw, px) = (xy[w as usize], xy[x as usize]);
            if first.is_none() && orient2d(u, pw, v) > 0.0 && orient2d(u, px, v) < 0.0 {
                first = Some(next(e));
            }
        }
        let Some(mut h) = first else {
            // Never in a triangulation of a convex hull that holds both ends.
            return Walk::Crossed([u.x, u.y]);
        };
        let mut crossed = Vec::new();
        loop {
            let (p, q) = (self.from(h), self.to(h));
            if self.constrained[h as usize] {
                return Walk::Crossed(crossing(u, v, xy[p as usize], xy[q as usize]));
            }
            crossed.push((p, q));
            let t = self.twin(h);
            if t == NONE {
                return Walk::Crossed(crossing(u, v, xy[p as usize], xy[q as usize]));
            }
            let y = self.d.triangles[prev(t) as usize];
            if y == b {
                return Walk::Edges(crossed);
            }
            if on(y) {
                return Walk::Through(y);
            }
            let sy = side(orient2d(u, v, xy[y as usize]));
            let sp = side(orient2d(u, v, xy[p as usize]));
            // t runs q → p; the line leaves through y–q when y is on p's side, else through p–y.
            h = if sy == sp { prev(t) } else { next(t) };
        }
    }

    /// Puts the piece a–b in by flipping the edges it crosses (Sloan): an edge whose quadrilateral is strictly
    /// convex is flipped, its new edge queued again while it still crosses; then a–b is an edge, constrained.
    pub(super) fn recover(
        &mut self,
        xy: &[Vec2],
        a: u32,
        b: u32,
        crossed: Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let (u, v) = (xy[a as usize], xy[b as usize]);
        let crosses = |p: Vec2, q: Vec2| {
            side(orient2d(u, v, p)) * side(orient2d(u, v, q)) < 0
                && side(orient2d(p, q, u)) * side(orient2d(p, q, v)) < 0
        };
        let mut queue: VecDeque<(u32, u32)> = crossed.into();
        let most = 64 * (queue.len() + 1) * (queue.len() + 1) + 1024;
        let mut turns = 0;
        while let Some((p, q)) = queue.pop_front() {
            turns += 1;
            if turns > most {
                return Err(
                    "Kırık hat yüzeye yerleştirilemedi (çevirmeler bitmedi); bu bir yazılım hatasıdır, durumu \
                     bildirin."
                        .into(),
                );
            }
            let Some(e) = self.edge(p, q) else {
                continue;
            };
            let (r, s) = self.opposite(e);
            let (pp, pq, pr, ps) = (
                xy[p as usize],
                xy[q as usize],
                xy[r as usize],
                xy[s as usize],
            );
            let convex = side(orient2d(pr, ps, pp)) * side(orient2d(pr, ps, pq)) < 0;
            if !convex {
                queue.push_back((p, q));
                continue;
            }
            self.flip(e);
            if crosses(pr, ps) {
                queue.push_back((r, s));
            }
        }
        match self.edge(a, b) {
            Some(e) => {
                self.constrain(e);
                Ok(())
            }
            None => Err(
                "Kırık hat yüzeye yerleştirilemedi; bu bir yazılım hatasıdır, durumu bildirin.".into(),
            ),
        }
    }

    /// Lawson's flips: every edge no constraint holds is made locally Delaunay (the far corner not strictly inside
    /// the circle of the triangle across), so the triangulation is the constrained Delaunay one.
    pub(super) fn restore_delaunay(&mut self, xy: &[Vec2]) {
        let mut stack: Vec<u32> = (0..self.d.halfedges.len() as u32)
            .filter(|&e| {
                let tw = self.twin(e);
                tw != NONE && e < tw
            })
            .collect();
        while let Some(e) = stack.pop() {
            let tw = self.twin(e);
            if tw == NONE || self.constrained[e as usize] {
                continue;
            }
            let a0 = e - e % 3;
            let ar = a0 + (e + 2) % 3;
            let al = a0 + (e + 1) % 3;
            let p0 = xy[self.d.triangles[ar as usize] as usize];
            let pr = xy[self.d.triangles[e as usize] as usize];
            let pl = xy[self.d.triangles[al as usize] as usize];
            let p1 = xy[self.d.triangles[prev(tw) as usize] as usize];
            if incircle(pr, pl, p0, p1) > 0.0 {
                let b = tw;
                let b0 = b - b % 3;
                let br = b0 + (b + 1) % 3;
                self.flip(e);
                stack.extend([e, b, al, br]);
            }
        }
    }

    /// The height at `q` on the plain triangulation (no triangle left out), and the triangle it is in, from which
    /// the next walk may start.
    pub(super) fn height_in(&self, xy: &[Vec2], z: &[f64], q: [f64; 2], start: u32) -> (Option<f64>, u32) {
        match self.locate(xy, q, start) {
            Some(t) => (Some(self.interpolate(xy, z, t, q)), t),
            None => (None, start),
        }
    }

    pub(super) fn locate(&self, xy: &[Vec2], q: [f64; 2], start: u32) -> Option<u32> {
        match self.d.locate(xy, Vec2 { x: q[0], y: q[1] }, start) {
            Located::Inside(t) => Some(t),
            Located::Outside(_) => None,
        }
    }

    /// The height of triangle t's plane at q (barycentric, its weights the exact turns' areas).
    pub(super) fn interpolate(&self, xy: &[Vec2], z: &[f64], t: u32, q: [f64; 2]) -> f64 {
        let [i, j, k] = self.corners(t);
        let (a, b, c) = (xy[i as usize], xy[j as usize], xy[k as usize]);
        let p = Vec2 { x: q[0], y: q[1] };
        let whole = orient2d(a, b, c);
        let wa = orient2d(b, c, p) / whole;
        let wb = orient2d(c, a, p) / whole;
        let wc = orient2d(a, b, p) / whole;
        wa * z[i as usize] + wb * z[j as usize] + wc * z[k as usize]
    }
}
