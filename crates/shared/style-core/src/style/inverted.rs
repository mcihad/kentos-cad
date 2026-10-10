//! Ters alan's region (docs/adr/0213 §2.9): the box less the areas, as
//! trapezoids. The areas' rings are clipped to the box (Sutherland–Hodgman)
//! and swept upward: their non-horizontal edges are kept in order across,
//! between the box's sides; a gap between two neighbours holds one open
//! trapezoid from the level it opened at, closed only when one of its two
//! edges ends, a new edge comes into it or its edges cross (Seidel's merged
//! trapezoids: O(n) of them). A gap is in the region when the areas' edges
//! to its left leave it outside: an even number of them (even-odd, QGIS's
//! default), or a winding of 0 (non-zero, `merge`: overlaps stay empty; the
//! outer rings are turned counter-clockwise, the holes clockwise first).
//! Edges that cross meet at a level found between two levels for each pair
//! of neighbours.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::{Bounds, signed_area};
use kentos_geometry_core::jsmath::js_min;

use super::place::positive;

/// Overlaps of two areas: painted again (even-odd) or empty (non-zero).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    EvenOdd,
    NonZero,
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    lo: Vec2,
    hi: Vec2,
    /// +1 drawn upward, −1 downward: crossing it rightward changes the winding by `−dir`.
    dir: i32,
}

impl Edge {
    fn x_at(&self, y: f64) -> f64 {
        if y <= self.lo.y {
            self.lo.x
        } else if y >= self.hi.y {
            self.hi.x
        } else {
            self.lo.x + (self.hi.x - self.lo.x) * (y - self.lo.y) / (self.hi.y - self.lo.y)
        }
    }

    fn slope(&self) -> f64 {
        (self.hi.x - self.lo.x) / (self.hi.y - self.lo.y)
    }
}

/// A ring clipped to the box (its points on the box's sides where it leaves it).
fn clip(ring: &[Vec2], b: &Bounds) -> Vec<Vec2> {
    let mut pts: Vec<Vec2> = ring.to_vec();
    // Each side: inside test and where the segment meets it.
    for side in 0..4 {
        if pts.is_empty() {
            break;
        }
        let inside = |p: Vec2| match side {
            0 => p.x >= b.min_x,
            1 => p.x <= b.max_x,
            2 => p.y >= b.min_y,
            _ => p.y <= b.max_y,
        };
        let cut = |a: Vec2, c: Vec2| match side {
            0 | 1 => {
                let x = if side == 0 { b.min_x } else { b.max_x };
                Vec2::new(x, a.y + (c.y - a.y) * (x - a.x) / (c.x - a.x))
            }
            _ => {
                let y = if side == 2 { b.min_y } else { b.max_y };
                Vec2::new(a.x + (c.x - a.x) * (y - a.y) / (c.y - a.y), y)
            }
        };
        let mut out = Vec::with_capacity(pts.len() + 4);
        let n = pts.len();
        for i in 0..n {
            let a = pts[(i + n - 1) % n];
            let c = pts[i];
            match (inside(a), inside(c)) {
                (true, true) => out.push(c),
                (true, false) => out.push(cut(a, c)),
                (false, true) => {
                    out.push(cut(a, c));
                    out.push(c);
                }
                (false, false) => {}
            }
        }
        pts = out;
    }
    pts
}

/// A level of the sweep, smallest first in the heap.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Level(f64);

impl Eq for Level {}

impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Level {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0)
    }
}

/// The left and the right side of the box, after the edges.
const LEFT: usize = usize::MAX - 1;
const RIGHT: usize = usize::MAX;

/// The region of `b` outside `areas` (each its outer ring, then its holes)
/// as trapezoids (four points, lower left first, counter-clockwise; a side
/// may have no length).
pub fn region(b: &Bounds, areas: &[&[Vec<Vec2>]], rule: Rule) -> Vec<[Vec2; 4]> {
    if !positive(b.max_x - b.min_x) || !positive(b.max_y - b.min_y) {
        return Vec::new();
    }
    let mut edges: Vec<Edge> = Vec::new();
    for rings in areas {
        for (k, ring) in rings.iter().enumerate() {
            if ring.len() < 3 {
                continue;
            }
            // Outer rings counter-clockwise, holes clockwise: a winding counts the areas over a point.
            let ccw = signed_area(ring) > 0.0;
            let flip = (k == 0) != ccw;
            let clipped = clip(ring, b);
            let n = clipped.len();
            if n < 3 {
                continue;
            }
            for i in 0..n {
                let (mut a, mut c) = (clipped[i], clipped[(i + 1) % n]);
                if flip {
                    std::mem::swap(&mut a, &mut c);
                }
                if a.y == c.y {
                    continue;
                }
                edges.push(if a.y < c.y {
                    Edge {
                        lo: a,
                        hi: c,
                        dir: 1,
                    }
                } else {
                    Edge {
                        lo: c,
                        hi: a,
                        dir: -1,
                    }
                });
            }
        }
    }
    let x_of = |e: usize, y: f64| match e {
        LEFT => b.min_x,
        RIGHT => b.max_x,
        _ => edges[e].x_at(y),
    };
    // The order across at `y`: x, then the slope above `y`; the box's sides first and last.
    let key = |e: usize, y: f64| -> (u8, f64, f64, usize) {
        match e {
            LEFT => (0, 0.0, 0.0, 0),
            RIGHT => (2, 0.0, 0.0, 0),
            _ => (1, edges[e].x_at(y), edges[e].slope(), e),
        }
    };
    let cmp_key = |a: &(u8, f64, f64, usize), c: &(u8, f64, f64, usize)| {
        a.0.cmp(&c.0)
            .then(a.1.total_cmp(&c.1))
            .then(a.2.total_cmp(&c.2))
            .then(a.3.cmp(&c.3))
    };
    let mut starts: Vec<usize> = (0..edges.len()).collect();
    starts.sort_by(|&a, &c| edges[a].lo.y.total_cmp(&edges[c].lo.y));
    let mut levels: Vec<f64> = edges.iter().flat_map(|e| [e.lo.y, e.hi.y]).collect();
    levels.push(b.min_y);
    levels.push(b.max_y);
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    let mut crossings: BinaryHeap<Level> = BinaryHeap::new();

    // The edges across, and for the gap to the right of each: when its trapezoid opened, and
    // whether it is in the region.
    let mut active: Vec<usize> = vec![LEFT, RIGHT];
    let mut open: Vec<f64> = vec![b.min_y, f64::NAN];
    let mut inside: Vec<bool> = vec![true, false];
    let mut out: Vec<[Vec2; 4]> = Vec::new();
    let mut next_start = 0;
    let mut li = 0;

    let emit = |out: &mut Vec<[Vec2; 4]>, l: usize, r: usize, from: f64, to: f64| {
        if !positive(to - from) {
            return;
        }
        let (bl, br) = (x_of(l, from), x_of(r, from));
        let (tl, tr) = (x_of(l, to), x_of(r, to));
        if br - bl <= 0.0 && tr - tl <= 0.0 {
            return;
        }
        out.push([
            Vec2::new(bl, from),
            Vec2::new(br, from),
            Vec2::new(tr, to),
            Vec2::new(tl, to),
        ]);
    };

    loop {
        // The next level: a vertex's or a crossing's, whichever is lower.
        let vertex = levels.get(li).copied();
        let crossing = crossings.peek().map(|l| l.0);
        let y = match (vertex, crossing) {
            (Some(v), Some(c)) if c < v => {
                crossings.pop();
                c
            }
            (Some(v), Some(c)) if c == v => {
                crossings.pop();
                li += 1;
                v
            }
            (Some(v), _) => {
                li += 1;
                v
            }
            (None, Some(c)) => {
                crossings.pop();
                c
            }
            (None, None) => break,
        };
        if y < b.min_y {
            continue;
        }
        if y > b.max_y {
            break;
        }
        // What changes at `y`: edges ending, edges starting, neighbours out of order.
        let mut lo = usize::MAX;
        let mut hi = 0usize;
        let mut ending = false;
        for (k, &e) in active.iter().enumerate() {
            if e < edges.len() && edges[e].hi.y <= y {
                lo = lo.min(k);
                hi = hi.max(k);
                ending = true;
            }
        }
        let _ = ending;
        let mut new_edges: Vec<usize> = Vec::new();
        while next_start < starts.len() && edges[starts[next_start]].lo.y <= y {
            let e = starts[next_start];
            next_start += 1;
            if edges[e].hi.y <= y {
                continue;
            }
            new_edges.push(e);
            // Where it goes: after the last edge left of it.
            let ke = key(e, y);
            let at = active.partition_point(|&a| cmp_key(&key(a, y), &ke) == Ordering::Less);
            lo = lo.min(at.saturating_sub(1).max(1));
            hi = hi.max(at.saturating_sub(1).max(1));
        }
        for k in 1..active.len().saturating_sub(1) {
            if k + 1 < active.len() - 1 {
                let (a, c) = (active[k], active[k + 1]);
                if cmp_key(&key(a, y), &key(c, y)) == Ordering::Greater {
                    lo = lo.min(k);
                    hi = hi.max(k + 1);
                }
            }
        }
        if lo == usize::MAX {
            continue;
        }
        // The span `lo..=hi` of edges is built again: the gaps to the right of `lo − 1` … `hi` close.
        let lo = lo.max(1);
        let hi = hi.min(active.len() - 2).max(lo.saturating_sub(1));
        for k in lo - 1..=hi {
            if inside[k] {
                emit(&mut out, active[k], active[k + 1], open[k], y);
            }
        }
        let mut span: Vec<usize> = active[lo..=hi]
            .iter()
            .copied()
            .filter(|&e| edges[e].hi.y > y)
            .collect();
        span.extend(new_edges);
        span.sort_by(|&a, &c| cmp_key(&key(a, y), &key(c, y)));
        // The windings: from the gap left of the span.
        let mut winding = 0i32;
        let mut odd = false;
        for &e in &active[1..lo] {
            winding -= edges[e].dir;
            odd = !odd;
        }
        let mut gaps_open = Vec::with_capacity(span.len() + 1);
        let mut gaps_inside = Vec::with_capacity(span.len() + 1);
        let region = |w: i32, odd: bool| match rule {
            Rule::EvenOdd => !odd,
            Rule::NonZero => w == 0,
        };
        gaps_open.push(y);
        gaps_inside.push(region(winding, odd));
        for &e in &span {
            winding -= edges[e].dir;
            odd = !odd;
            gaps_open.push(y);
            gaps_inside.push(region(winding, odd));
        }
        // The gap right of the span's last edge keeps its own (the next edge left it as it was).
        let last_open = gaps_open.pop().unwrap_or(y);
        let last_inside = gaps_inside.pop().unwrap_or(false);
        active.splice(lo..=hi, span.iter().copied());
        open.splice(lo - 1..=hi, gaps_open.iter().copied().chain([last_open]));
        inside.splice(
            lo - 1..=hi,
            gaps_inside.iter().copied().chain([last_inside]),
        );
        // Neighbours that cross before the next vertex level meet at a level of their own.
        let next = levels.get(li).copied().unwrap_or(b.max_y);
        let from = (lo.saturating_sub(1)).max(1);
        let to = (lo + span.len() + 1).min(active.len() - 2);
        for k in from..to {
            let (a, c) = (active[k], active[k + 1]);
            if a >= edges.len() || c >= edges.len() {
                continue;
            }
            let (ea, ec) = (edges[a], edges[c]);
            let top = js_min(js_min(next, ea.hi.y), ec.hi.y);
            if ea.x_at(top) > ec.x_at(top) {
                let (sa, sc) = (ea.slope(), ec.slope());
                if sa != sc {
                    let t = (ec.lo.x - ea.lo.x + sa * ea.lo.y - sc * ec.lo.y) / (sa - sc);
                    if t > y && t < top {
                        crossings.push(Level(t));
                    }
                }
            }
        }
        if y >= b.max_y {
            break;
        }
    }
    // What is still open closes at the top of the box.
    for k in 0..active.len().saturating_sub(1) {
        if inside[k] {
            emit(&mut out, active[k], active[k + 1], open[k], b.max_y);
        }
    }
    out
}
