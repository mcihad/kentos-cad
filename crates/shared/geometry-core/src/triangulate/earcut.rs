//! Mapbox's earcut (v3.0, JavaScript), ported for the polygons the ear clipper above leaves unfilled
//! (docs/adr/9228): holes touching each other or the outline at a vertex, rings crossing. Holes are joined
//! where their corners hold the bridge (`find_hole_bridge`, `sector_contains_sector`); a ring with no ear
//! left is filtered of repeated and collinear points, then its small crossings are cured, then it is split
//! along a valid diagonal and each half clipped. Not hashed by z-order: it runs on such polygons only.
//!
//! Its `area` keeps earcut's sign, negative for a left turn: rings run counter-clockwise (holes
//! clockwise) as the clipper above has them, and a triangle is a left turn.
//!
//! ```text
//! ISC License
//!
//! Copyright (c) 2016, Mapbox
//!
//! Permission to use, copy, modify, and/or distribute this software for any purpose
//! with or without fee is hereby granted, provided that the above copyright notice
//! and this permission notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND ISC DISCLAIMS ALL WARRANTIES WITH REGARD TO
//! THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS.
//! IN NO EVENT SHALL ISC BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR
//! CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA
//! OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
//! ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//! ```

use std::ops::Range;

use crate::jsmath::{js_cmp, stable_sort};
use crate::vec2::Vec2;

#[derive(Clone, Copy)]
struct Node {
    /// The point's index in the caller's points.
    i: u32,
    x: f64,
    y: f64,
    prev: usize,
    next: usize,
    /// A hole of one point: never filtered away.
    steiner: bool,
}

/// The rings as circular doubly linked lists in one arena.
struct List {
    n: Vec<Node>,
}

/// Appends the triangles of one polygon to `out`, three indices into `pts` a triangle; `rings` are ranges of
/// `pts`, the outer ring then its holes. Holes of fewer than three points are left out, as the clipper does.
pub(super) fn earcut_into(pts: &[Vec2], rings: &[Range<usize>], out: &mut Vec<u32>) {
    let Some(outer) = rings.first() else {
        return;
    };
    let mut l = List { n: Vec::with_capacity(rings.iter().map(|r| r.len() + 2).sum()) };
    let Some(mut outer_node) = l.linked(pts, outer.clone(), true) else {
        return;
    };
    if l.n[outer_node].next == l.n[outer_node].prev {
        return;
    }
    let holes: Vec<Range<usize>> = rings[1..].iter().filter(|h| h.len() >= 3).cloned().collect();
    if !holes.is_empty() {
        outer_node = l.eliminate_holes(pts, &holes, outer_node);
    }
    l.earcut_linked(Some(outer_node), out, 0);
}

fn min3(a: f64, b: f64, c: f64) -> f64 {
    if a < b {
        if a < c { a } else { c }
    } else if b < c {
        b
    } else {
        c
    }
}

fn max3(a: f64, b: f64, c: f64) -> f64 {
    if a > b {
        if a > c { a } else { c }
    } else if b > c {
        b
    } else {
        c
    }
}

#[allow(clippy::too_many_arguments)]
fn point_in_triangle(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, px: f64, py: f64) -> bool {
    (cx - px) * (ay - py) >= (ax - px) * (cy - py)
        && (ax - px) * (by - py) >= (bx - px) * (ay - py)
        && (bx - px) * (cy - py) >= (cx - px) * (by - py)
}

fn sign(v: f64) -> i8 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

impl List {
    fn node(&mut self, i: u32, x: f64, y: f64) -> usize {
        let k = self.n.len();
        self.n.push(Node { i, x, y, prev: k, next: k, steiner: false });
        k
    }

    fn insert(&mut self, i: usize, p: Vec2, last: Option<usize>) -> usize {
        let k = self.node(i as u32, p.x, p.y);
        if let Some(l) = last {
            let ln = self.n[l].next;
            self.n[k].next = ln;
            self.n[k].prev = l;
            self.n[ln].prev = k;
            self.n[l].next = k;
        }
        k
    }

    fn remove(&mut self, p: usize) {
        let (pr, nx) = (self.n[p].prev, self.n[p].next);
        self.n[nx].prev = pr;
        self.n[pr].next = nx;
    }

    fn equals(&self, a: usize, b: usize) -> bool {
        self.n[a].x == self.n[b].x && self.n[a].y == self.n[b].y
    }

    /// Twice the triangle's area, negative for a left turn (earcut's sign).
    fn area(&self, p: usize, q: usize, r: usize) -> f64 {
        let (p, q, r) = (&self.n[p], &self.n[q], &self.n[r]);
        (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y)
    }

    /// A ring as a list, running counter-clockwise for `clockwise` (earcut's name, its y down), else clockwise.
    fn linked(&mut self, pts: &[Vec2], r: Range<usize>, clockwise: bool) -> Option<usize> {
        let ring = &pts[r.clone()];
        let n = ring.len();
        let mut sum = 0.0;
        for k in 0..n {
            let (p, q) = (ring[(k + n - 1) % n], ring[k]);
            sum += (p.x - q.x) * (q.y + p.y);
        }
        let mut last = None;
        if clockwise == (sum > 0.0) {
            for i in r {
                last = Some(self.insert(i, pts[i], last));
            }
        } else {
            for i in r.rev() {
                last = Some(self.insert(i, pts[i], last));
            }
        }
        if let Some(l) = last
            && self.equals(l, self.n[l].next)
        {
            self.remove(l);
            last = Some(self.n[l].next);
        }
        last
    }

    /// Removes repeated and collinear points from `start` round to `end`; the node left.
    fn filter(&mut self, start: Option<usize>, end: Option<usize>) -> Option<usize> {
        let start = start?;
        let mut end = end.unwrap_or(start);
        let mut p = start;
        loop {
            let mut again = false;
            let (pr, nx) = (self.n[p].prev, self.n[p].next);
            if !self.n[p].steiner && (self.equals(p, nx) || self.area(pr, p, nx) == 0.0) {
                self.remove(p);
                p = self.n[p].prev;
                end = p;
                if p == self.n[p].next {
                    break;
                }
                again = true;
            } else {
                p = nx;
            }
            if !again && p == end {
                break;
            }
        }
        Some(end)
    }

    fn earcut_linked(&mut self, ear: Option<usize>, out: &mut Vec<u32>, pass: u8) {
        let Some(mut ear) = ear else {
            return;
        };
        let mut stop = ear;
        while self.n[ear].prev != self.n[ear].next {
            let (prev, next) = (self.n[ear].prev, self.n[ear].next);
            if self.is_ear(ear) {
                out.extend([self.n[prev].i, self.n[ear].i, self.n[next].i]);
                self.remove(ear);
                // Skipping the next vertex leaves fewer slivers.
                ear = self.n[next].next;
                stop = ear;
                continue;
            }
            ear = next;
            if ear == stop {
                match pass {
                    0 => {
                        let f = self.filter(Some(ear), None);
                        self.earcut_linked(f, out, 1);
                    }
                    1 => {
                        let f = self.filter(Some(ear), None);
                        let cured = f.and_then(|f| self.cure_local_intersections(f, out));
                        self.earcut_linked(cured, out, 2);
                    }
                    _ => self.split_earcut(ear, out),
                }
                break;
            }
        }
    }

    fn is_ear(&self, ear: usize) -> bool {
        let (a, b, c) = (self.n[ear].prev, ear, self.n[ear].next);
        if self.area(a, b, c) >= 0.0 {
            return false;
        }
        let (ax, bx, cx, ay, by, cy) =
            (self.n[a].x, self.n[b].x, self.n[c].x, self.n[a].y, self.n[b].y, self.n[c].y);
        let (x0, y0, x1, y1) = (min3(ax, bx, cx), min3(ay, by, cy), max3(ax, bx, cx), max3(ay, by, cy));
        let mut p = self.n[c].next;
        while p != a {
            let q = self.n[p];
            if q.x >= x0
                && q.x <= x1
                && q.y >= y0
                && q.y <= y1
                && !(ax == q.x && ay == q.y)
                && point_in_triangle(ax, ay, bx, by, cx, cy, q.x, q.y)
                && self.area(q.prev, p, q.next) >= 0.0
            {
                return false;
            }
            p = q.next;
        }
        true
    }

    /// Cures a ring's small crossings: a corner whose neighbours' edges cross is cut off as a triangle.
    fn cure_local_intersections(&mut self, start: usize, out: &mut Vec<u32>) -> Option<usize> {
        let mut start = start;
        let mut p = start;
        loop {
            let a = self.n[p].prev;
            let pn = self.n[p].next;
            let b = self.n[pn].next;
            if !self.equals(a, b)
                && self.intersects(a, p, pn, b)
                && self.locally_inside(a, b)
                && self.locally_inside(b, a)
            {
                out.extend([self.n[a].i, self.n[p].i, self.n[b].i]);
                self.remove(p);
                self.remove(pn);
                p = b;
                start = b;
            }
            p = self.n[p].next;
            if p == start {
                break;
            }
        }
        self.filter(Some(p), None)
    }

    /// Splits the ring along a valid diagonal and clips each half.
    fn split_earcut(&mut self, start: usize, out: &mut Vec<u32>) {
        let mut a = start;
        loop {
            let mut b = self.n[self.n[a].next].next;
            while b != self.n[a].prev {
                if self.n[a].i != self.n[b].i && self.is_valid_diagonal(a, b) {
                    let c = self.split_polygon(a, b);
                    let an = self.n[a].next;
                    let a = self.filter(Some(a), Some(an));
                    let cn = self.n[c].next;
                    let c = self.filter(Some(c), Some(cn));
                    self.earcut_linked(a, out, 0);
                    self.earcut_linked(c, out, 0);
                    return;
                }
                b = self.n[b].next;
            }
            a = self.n[a].next;
            if a == start {
                break;
            }
        }
    }

    /// Links each hole into the outer ring, left to right.
    fn eliminate_holes(&mut self, pts: &[Vec2], holes: &[Range<usize>], outer: usize) -> usize {
        let mut queue = Vec::with_capacity(holes.len());
        for h in holes {
            if let Some(list) = self.linked(pts, h.clone(), false) {
                if list == self.n[list].next {
                    self.n[list].steiner = true;
                }
                queue.push(self.leftmost(list));
            }
        }
        stable_sort(&mut queue, &mut |&a, &b| js_cmp(self.compare_xy_slope(a, b), 0.0));
        let mut outer = outer;
        for &h in &queue {
            outer = self.eliminate_hole(h, outer);
        }
        outer
    }

    /// Left to right; holes whose leftmost points meet, counter-clockwise, so that each is joined at the point
    /// they meet at.
    fn compare_xy_slope(&self, a: usize, b: usize) -> f64 {
        let (na, nb) = (self.n[a], self.n[b]);
        let mut result = na.x - nb.x;
        if result == 0.0 {
            result = na.y - nb.y;
            if result == 0.0 {
                let (an, bn) = (self.n[na.next], self.n[nb.next]);
                let a_slope = (an.y - na.y) / (an.x - na.x);
                let b_slope = (bn.y - nb.y) / (bn.x - nb.x);
                result = a_slope - b_slope;
            }
        }
        result
    }

    fn eliminate_hole(&mut self, hole: usize, outer: usize) -> usize {
        let Some(bridge) = self.find_hole_bridge(hole, outer) else {
            return outer;
        };
        let reverse = self.split_polygon(bridge, hole);
        let rn = self.n[reverse].next;
        self.filter(Some(reverse), Some(rn));
        let bn = self.n[bridge].next;
        self.filter(Some(bridge), Some(bn)).unwrap_or(bridge)
    }

    /// Eberly's bridge from the hole's leftmost point, joined where the outer ring's corner holds the hole.
    fn find_hole_bridge(&self, hole: usize, outer: usize) -> Option<usize> {
        let (hx, hy) = (self.n[hole].x, self.n[hole].y);
        let mut p = outer;
        let mut qx = f64::NEG_INFINITY;
        let mut m: Option<usize> = None;
        // A ray from the hole's leftmost point to the left meets a segment; its end with the lesser x may join,
        // or the vertex the ray meets.
        if self.equals(hole, p) {
            return Some(p);
        }
        loop {
            let nx = self.n[p].next;
            if self.equals(hole, nx) {
                return Some(nx);
            }
            let (pp, pn) = (self.n[p], self.n[nx]);
            if hy <= pp.y && hy >= pn.y && pn.y != pp.y {
                let x = pp.x + (hy - pp.y) * (pn.x - pp.x) / (pn.y - pp.y);
                if x <= hx && x > qx {
                    qx = x;
                    let cand = if pp.x < pn.x { p } else { nx };
                    m = Some(cand);
                    if x == hx {
                        // The hole touches the segment: its leftmost end joins.
                        return m;
                    }
                }
            }
            p = nx;
            if p == outer {
                break;
            }
        }
        let mut m = m?;
        // A point inside the triangle of the hole's point, the meeting point and the end: the one closest in angle
        // to the ray joins, where its corner holds the hole.
        let stop = m;
        let (mx, my) = (self.n[m].x, self.n[m].y);
        let mut tan_min = f64::INFINITY;
        p = m;
        loop {
            let q = self.n[p];
            if hx >= q.x
                && q.x >= mx
                && hx != q.x
                && point_in_triangle(
                    if hy < my { hx } else { qx },
                    hy,
                    mx,
                    my,
                    if hy < my { qx } else { hx },
                    hy,
                    q.x,
                    q.y,
                )
            {
                let tan = (hy - q.y).abs() / (hx - q.x);
                if self.locally_inside(p, hole)
                    && (tan < tan_min
                        || (tan == tan_min
                            && (q.x > self.n[m].x
                                || (q.x == self.n[m].x && self.sector_contains_sector(m, p)))))
                {
                    m = p;
                    tan_min = tan;
                }
            }
            p = q.next;
            if p == stop {
                break;
            }
        }
        Some(m)
    }

    /// Whether the corner at `m` holds the corner at `p`, the same point passed twice.
    fn sector_contains_sector(&self, m: usize, p: usize) -> bool {
        self.area(self.n[m].prev, m, self.n[p].prev) < 0.0 && self.area(self.n[p].next, m, self.n[m].next) < 0.0
    }

    fn leftmost(&self, start: usize) -> usize {
        let (mut p, mut left) = (start, start);
        loop {
            let (q, l) = (self.n[p], self.n[left]);
            if q.x < l.x || (q.x == l.x && q.y < l.y) {
                left = p;
            }
            p = q.next;
            if p == start {
                break;
            }
        }
        left
    }

    /// Whether a diagonal between two points lies inside the ring.
    fn is_valid_diagonal(&self, a: usize, b: usize) -> bool {
        let (na, nb) = (self.n[a], self.n[b]);
        self.n[na.next].i != nb.i
            && self.n[na.prev].i != nb.i
            && !self.intersects_polygon(a, b)
            && ((self.locally_inside(a, b)
                && self.locally_inside(b, a)
                && self.middle_inside(a, b)
                && (self.area(na.prev, a, nb.prev) != 0.0 || self.area(a, nb.prev, b) != 0.0))
                || (self.equals(a, b)
                    && self.area(na.prev, a, na.next) > 0.0
                    && self.area(nb.prev, b, nb.next) > 0.0))
    }

    fn intersects(&self, p1: usize, q1: usize, p2: usize, q2: usize) -> bool {
        let o1 = sign(self.area(p1, q1, p2));
        let o2 = sign(self.area(p1, q1, q2));
        let o3 = sign(self.area(p2, q2, p1));
        let o4 = sign(self.area(p2, q2, q1));
        (o1 != o2 && o3 != o4)
            || (o1 == 0 && self.on_segment(p1, p2, q1))
            || (o2 == 0 && self.on_segment(p1, q2, q1))
            || (o3 == 0 && self.on_segment(p2, p1, q2))
            || (o4 == 0 && self.on_segment(p2, q1, q2))
    }

    /// For collinear points, whether `q` lies on the segment from `p` to `r`.
    fn on_segment(&self, p: usize, q: usize, r: usize) -> bool {
        let (p, q, r) = (self.n[p], self.n[q], self.n[r]);
        q.x <= if p.x > r.x { p.x } else { r.x }
            && q.x >= if p.x < r.x { p.x } else { r.x }
            && q.y <= if p.y > r.y { p.y } else { r.y }
            && q.y >= if p.y < r.y { p.y } else { r.y }
    }

    fn intersects_polygon(&self, a: usize, b: usize) -> bool {
        let (ai, bi) = (self.n[a].i, self.n[b].i);
        let mut p = a;
        loop {
            let nx = self.n[p].next;
            let (pi, ni) = (self.n[p].i, self.n[nx].i);
            if pi != ai && ni != ai && pi != bi && ni != bi && self.intersects(p, nx, a, b) {
                return true;
            }
            p = nx;
            if p == a {
                return false;
            }
        }
    }

    fn locally_inside(&self, a: usize, b: usize) -> bool {
        let (ap, an) = (self.n[a].prev, self.n[a].next);
        if self.area(ap, a, an) < 0.0 {
            self.area(a, b, an) >= 0.0 && self.area(a, ap, b) >= 0.0
        } else {
            self.area(a, b, ap) < 0.0 || self.area(a, an, b) < 0.0
        }
    }

    fn middle_inside(&self, a: usize, b: usize) -> bool {
        let (px, py) = ((self.n[a].x + self.n[b].x) / 2.0, (self.n[a].y + self.n[b].y) / 2.0);
        let mut inside = false;
        let mut p = a;
        loop {
            let (q, n) = (self.n[p], self.n[self.n[p].next]);
            if (q.y > py) != (n.y > py) && n.y != q.y && px < (n.x - q.x) * (py - q.y) / (n.y - q.y) + q.x {
                inside = !inside;
            }
            p = q.next;
            if p == a {
                return inside;
            }
        }
    }

    /// Links `a` and `b` by a bridge: splits a ring in two, or joins a hole to its ring. The copy of `b`.
    fn split_polygon(&mut self, a: usize, b: usize) -> usize {
        let (ai, ax, ay) = (self.n[a].i, self.n[a].x, self.n[a].y);
        let (bi, bx, by) = (self.n[b].i, self.n[b].x, self.n[b].y);
        let a2 = self.node(ai, ax, ay);
        let b2 = self.node(bi, bx, by);
        let an = self.n[a].next;
        let bp = self.n[b].prev;
        self.n[a].next = b;
        self.n[b].prev = a;
        self.n[a2].next = an;
        self.n[an].prev = a2;
        self.n[b2].next = a2;
        self.n[a2].prev = b2;
        self.n[bp].next = b2;
        self.n[b2].prev = bp;
        b2
    }
}
