//! Ortak kenarları koruyarak sadeleştir (docs/adr/0221 §8): the coverage's
//! edges simplified once each by Visvalingam–Whyatt, GEOS's
//! CoverageSimplifier's way: the smallest corner goes while its area is
//! under the tolerance squared, unless another vertex of any edge lies in
//! its triangle (then a line would cross) or its edge would fall onto
//! another straight edge between the same two vertices. An edge's ends, a
//! closed edge's first vertex and an arc's ends stay.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use super::Track;
use super::coverage::Coverage;
use crate::jsmath::{js_max, js_min};
use crate::predicates::orientation;
use crate::vec2::Vec2;

/// A corner waiting: its area and its vertex, the smallest first, then the first vertex.
#[derive(Clone, Copy, PartialEq)]
struct Corner {
    area: f64,
    at: usize,
}

impl Eq for Corner {}

impl Ord for Corner {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .area
            .total_cmp(&self.area)
            .then(other.at.cmp(&self.at))
    }
}

impl PartialOrd for Corner {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

type Key = (u64, u64);

/// Two vertices as a key either way round.
fn ends(a: Vec2, b: Vec2) -> (Key, Key) {
    let k = |p: Vec2| ((p.x + 0.0).to_bits(), (p.y + 0.0).to_bits());
    let (ka, kb) = (k(a), k(b));
    if ka <= kb { (ka, kb) } else { (kb, ka) }
}

fn corner_area(p: Vec2, k: Vec2, s: Vec2) -> f64 {
    let (ax, ay, bx, by) = (s.x - p.x, s.y - p.y, k.x - p.x, k.y - p.y);
    (ax * by - ay * bx).abs() / 2.0
}

/// Whether `v` lies in the triangle a, b, c or on its sides.
fn in_triangle(a: Vec2, b: Vec2, c: Vec2, v: Vec2) -> bool {
    let (o1, o2, o3) = (
        orientation(a, b, v),
        orientation(b, c, v),
        orientation(c, a, v),
    );
    (o1 >= 0 && o2 >= 0 && o3 >= 0) || (o1 <= 0 && o2 <= 0 && o3 <= 0)
}

/// Every edge's vertices in a grid of square cells (filled by counting, no
/// sorting: two million vertices in a few tens of milliseconds), and which
/// are gone.
struct Vertices {
    first: Vec<usize>,
    points: Vec<Vec2>,
    gone: Vec<bool>,
    x0: f64,
    y0: f64,
    size: f64,
    cols: usize,
    rows: usize,
    /// Each cell's vertices: `items[start[c] .. start[c + 1]]`.
    start: Vec<u32>,
    items: Vec<u32>,
}

impl Vertices {
    fn of(edges: &[Track]) -> Vertices {
        let mut first = Vec::with_capacity(edges.len());
        let mut points = Vec::new();
        for e in edges {
            first.push(points.len());
            points.extend(e.pts.iter().copied());
        }
        let n = points.len().max(1);
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for p in &points {
            x0 = js_min(x0, p.x);
            y0 = js_min(y0, p.y);
            x1 = js_max(x1, p.x);
            y1 = js_max(y1, p.y);
        }
        if !(x1 >= x0) {
            (x0, y0, x1, y1) = (0.0, 0.0, 1.0, 1.0);
        }
        // About four vertices a cell, at most four cells a vertex.
        let span = js_max(x1 - x0, y1 - y0);
        let size = js_max(
            ((x1 - x0) * (y1 - y0) * 4.0 / n as f64).sqrt(),
            span / (2.0 * (n as f64).sqrt()),
        );
        let size = if size > 0.0 && size.is_finite() {
            size
        } else {
            1.0
        };
        let cols = ((x1 - x0) / size) as usize + 1;
        let rows = ((y1 - y0) / size) as usize + 1;
        let cell = |p: &Vec2| {
            let c = (((p.x - x0) / size) as usize).min(cols - 1);
            let r = (((p.y - y0) / size) as usize).min(rows - 1);
            r * cols + c
        };
        let mut start = vec![0u32; cols * rows + 1];
        for p in &points {
            start[cell(p) + 1] += 1;
        }
        for c in 0..cols * rows {
            start[c + 1] += start[c];
        }
        let mut fill = start.clone();
        let mut items = vec![0u32; points.len()];
        for (i, p) in points.iter().enumerate() {
            let c = cell(p);
            items[fill[c] as usize] = i as u32;
            fill[c] += 1;
        }
        Vertices {
            first,
            gone: vec![false; points.len()],
            points,
            x0,
            y0,
            size,
            cols,
            rows,
            start,
            items,
        }
    }

    /// Whether a vertex of any edge (not one of the corner's three) lies in the corner's triangle.
    fn in_corner(&self, a: Vec2, b: Vec2, c: Vec2) -> bool {
        let lo_x = js_min(a.x, js_min(b.x, c.x));
        let lo_y = js_min(a.y, js_min(b.y, c.y));
        let hi_x = js_max(a.x, js_max(b.x, c.x));
        let hi_y = js_max(a.y, js_max(b.y, c.y));
        // `as` saturates: a place before the grid is its first cell.
        let at = |v: f64, o: f64, most: usize| (((v - o) / self.size) as usize).min(most - 1);
        let (c0, c1) = (at(lo_x, self.x0, self.cols), at(hi_x, self.x0, self.cols));
        let (r0, r1) = (at(lo_y, self.y0, self.rows), at(hi_y, self.y0, self.rows));
        for r in r0..=r1 {
            for col in c0..=c1 {
                let cell = r * self.cols + col;
                for &g in &self.items[self.start[cell] as usize..self.start[cell + 1] as usize] {
                    let g = g as usize;
                    let v = self.points[g];
                    if self.gone[g] || v.x < lo_x || v.x > hi_x || v.y < lo_y || v.y > hi_y {
                        continue;
                    }
                    if v != a && v != b && v != c && in_triangle(a, b, c, v) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Each edge's vertices that stay, after the coverage is simplified with
/// `tolerance` (m); `keep_boundary`: edges only one ring uses stay whole.
pub fn simplify(c: &Coverage, tolerance: f64, keep_boundary: bool) -> Vec<Track> {
    let limit = tolerance * tolerance;
    let mut all = Vertices::of(&c.edges);
    // The straight two-vertex edges, by their ends (either way round): an edge may not fall onto one.
    let mut twos: HashSet<(Key, Key)> = c
        .edges
        .iter()
        .filter(|e| !e.closed && e.pts.len() == 2 && e.bulges[0] == 0.0)
        .map(|e| ends(e.pts[0], e.pts[1]))
        .collect();
    for (k, e) in c.edges.iter().enumerate() {
        if keep_boundary && c.uses[k] < 2 {
            continue;
        }
        let n = e.pts.len();
        let base = all.first[k];
        let fixed: Vec<bool> = (0..n)
            .map(|i| i == 0 || (!e.closed && i + 1 == n) || e.on_arc(i))
            .collect();
        let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
        let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
        let mut left = n;
        // A loop edge (its ends one node) keeps two vertices inside; a closed edge three.
        let loop_edge = !e.closed && n > 1 && e.pts[0] == e.pts[n - 1];
        let least = if e.closed {
            3
        } else if loop_edge {
            4
        } else {
            2
        };
        let area_of = |prev: &[usize], next: &[usize], i: usize| {
            corner_area(e.pts[prev[i]], e.pts[i], e.pts[next[i]])
        };
        let mut queue = BinaryHeap::new();
        for i in 0..n {
            if !fixed[i] {
                queue.push(Corner {
                    area: area_of(&prev, &next, i),
                    at: i,
                });
            }
        }
        while let Some(Corner { area, at }) = queue.pop() {
            if all.gone[base + at] || area != area_of(&prev, &next, at) {
                continue;
            }
            if area >= limit || left <= least {
                break;
            }
            let (p, s) = (prev[at], next[at]);
            let (a, b, cc) = (e.pts[p], e.pts[at], e.pts[s]);
            if all.in_corner(a, b, cc) {
                continue;
            }
            // The edge would become the straight line a–c: not onto another such edge.
            if !e.closed && left == 3 && twos.contains(&ends(a, cc)) {
                continue;
            }
            all.gone[base + at] = true;
            left -= 1;
            next[p] = s;
            prev[s] = p;
            for j in [p, s] {
                if !fixed[j] {
                    queue.push(Corner {
                        area: area_of(&prev, &next, j),
                        at: j,
                    });
                }
            }
        }
        if !e.closed && left == 2 {
            twos.insert(ends(e.pts[0], e.pts[n - 1]));
        }
    }
    c.edges
        .iter()
        .enumerate()
        .map(|(k, e)| {
            let base = all.first[k];
            let mut t = Track::empty(e.closed);
            let n = e.pts.len();
            for i in 0..n {
                if all.gone[base + i] {
                    continue;
                }
                // The edge from a kept vertex: its own while the next one stayed, else straight.
                let bulge = if e.closed || i + 1 < n {
                    if all.gone[base + (i + 1) % n] {
                        0.0
                    } else {
                        e.bulges[i]
                    }
                } else {
                    0.0
                };
                t.pts.push(e.pts[i]);
                t.bulges.push(bulge);
                t.zs.push(e.zs[i]);
            }
            t
        })
        .collect()
}
