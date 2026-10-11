//! İçbükey zarf (docs/adr/0219 §5): the points' Delaunay triangles worn away
//! from the border (Martin Davis's ConcaveHull in JTS, GEOS's too). The
//! longest edge allowed is set by a ratio between the triangulation's shortest
//! and longest edge; border triangles go largest first, by the length of
//! their border edge, while it is not shorter than that, each only when it
//! has one border edge and its third corner is inside the rest (so the hull
//! stays one area without a point where it touches itself). With holes,
//! inner triangles touching no border whose longest edge is not shorter open
//! holes, largest first, each worn the same way without touching another
//! border. Lengths are √(dx² + dy²), as GEOS's.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::voronoi::{distinct, normalised};
use crate::geom::delaunay::{Delaunay, NONE, triangulate};
use crate::vec2::Vec2;

/// A border triangle in the queue: its size (its border edges' length), its
/// area, its number; the largest size first, then the larger area, then the
/// lower number.
#[derive(PartialEq)]
struct Queued {
    size: f64,
    area: f64,
    t: u32,
}

impl Eq for Queued {}

impl Ord for Queued {
    fn cmp(&self, o: &Self) -> Ordering {
        self.size
            .total_cmp(&o.size)
            .then(self.area.total_cmp(&o.area))
            .then(o.t.cmp(&self.t))
    }
}

impl PartialOrd for Queued {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn length(a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    (dx * dx + dy * dy).sqrt()
}

struct Hull<'a> {
    pts: &'a [Vec2],
    d: Delaunay,
    alive: Vec<bool>,
    size: Vec<f64>,
}

impl Hull<'_> {
    fn corner(&self, t: u32, i: u32) -> Vec2 {
        self.pts[self.d.triangles[(3 * t + i) as usize] as usize]
    }

    fn edge(&self, t: u32, i: u32) -> f64 {
        length(self.corner(t, i), self.corner(t, (i + 1) % 3))
    }

    /// The triangle across edge `i` of `t` (from corner `i` to corner `i + 1`) while it is there.
    fn adjacent(&self, t: u32, i: u32) -> Option<u32> {
        let tw = self.d.halfedges[(3 * t + i) as usize];
        (tw != NONE && self.alive[(tw / 3) as usize]).then_some(tw / 3)
    }

    fn count(&self, t: u32) -> usize {
        (0..3).filter(|&i| self.adjacent(t, i).is_some()).count()
    }

    fn area(&self, t: u32) -> f64 {
        let (a, b, c) = (self.corner(t, 0), self.corner(t, 1), self.corner(t, 2));
        ((c.x - a.x) * (b.y - a.y) - (b.x - a.x) * (c.y - a.y)).abs() / 2.0
    }

    /// The length of its border edges (those with no triangle across).
    fn border(&self, t: u32) -> f64 {
        let mut s = 0.0;
        for i in 0..3 {
            if self.adjacent(t, i).is_none() {
                s += self.edge(t, i);
            }
        }
        s
    }

    /// Whether corner `k` of `t` has triangles all round it.
    fn interior(&self, t: u32, k: u32) -> bool {
        let (mut cur, mut idx) = (t, k);
        for _ in 0..=self.alive.len() {
            let tw = self.d.halfedges[(3 * cur + idx) as usize];
            if tw == NONE || !self.alive[(tw / 3) as usize] {
                return false;
            }
            cur = tw / 3;
            idx = (tw % 3 + 1) % 3;
            if cur == t {
                return true;
            }
        }
        false
    }

    /// Whether taking `t` away (one border edge) would split the area: the
    /// corner between its two other edges is on a border.
    fn connecting(&self, t: u32) -> bool {
        let has = |i: u32| self.adjacent(t, i).is_some();
        let k = if has(0) && has(1) {
            1
        } else if has(1) && has(2) {
            2
        } else if has(2) && has(0) {
            0
        } else {
            return false;
        };
        !self.interior(t, k)
    }

    /// Whether a corner of `t` lies on a border though neither of its two edges in `t` is a border edge.
    fn touches(&self, t: u32) -> bool {
        (0..3).any(|k| {
            self.adjacent(t, k).is_some()
                && self.adjacent(t, (k + 2) % 3).is_some()
                && !self.interior(t, k)
        })
    }

    /// Queues a triangle that now has one border edge, its size that edge's length.
    fn push(&mut self, queue: &mut BinaryHeap<Queued>, t: u32) {
        if self.alive[t as usize] && self.count(t) == 2 {
            let size = self.border(t);
            self.size[t as usize] = size;
            queue.push(Queued {
                size,
                area: self.area(t),
                t,
            });
        }
    }

    fn remove(&mut self, queue: &mut BinaryHeap<Queued>, t: u32) {
        let around: Vec<u32> = (0..3).filter_map(|i| self.adjacent(t, i)).collect();
        self.alive[t as usize] = false;
        for n in around {
            self.push(queue, n);
        }
    }
}

/// The longest edge a ratio allows (GEOS's rule): none for 0, twice the
/// longest for 1, else the ratio of the way from the shortest to the longest.
pub fn target_length(pts: &[Vec2], d: &Delaunay, ratio: f64) -> f64 {
    if ratio <= 0.0 {
        return 0.0;
    }
    let (mut lo, mut hi) = (-1.0f64, -1.0f64);
    for t in d.triangles.chunks_exact(3) {
        for i in 0..3 {
            let l = length(pts[t[i] as usize], pts[t[(i + 1) % 3] as usize]);
            if l > hi {
                hi = l;
            }
            if lo < 0.0 || l < lo {
                lo = l;
            }
        }
    }
    if ratio >= 1.0 {
        return 2.0 * hi;
    }
    ratio * (hi - lo) + lo
}

/// The concave hull's rings: the outer counter-clockwise, then its holes
/// clockwise (by their lowest corner), each from its lowest corner; none when
/// the points have no triangle (fewer than three places, all on one line).
pub fn concave_hull(points: &[Vec2], ratio: f64, holes: bool) -> Option<Vec<Vec<Vec2>>> {
    let places = distinct(points).places;
    let d = triangulate(&places).ok()?;
    let n = d.triangles.len() / 3;
    if n == 0 {
        return None;
    }
    let target = target_length(&places, &d, ratio);
    let mut h = Hull {
        pts: &places,
        d,
        alive: vec![true; n],
        size: vec![0.0; n],
    };
    for t in 0..n as u32 {
        h.size[t as usize] = (0..3)
            .map(|i| h.edge(t, i))
            .fold(0.0, |a, b| if b > a { b } else { a });
    }
    let mut queue = BinaryHeap::new();
    for t in 0..n as u32 {
        h.push(&mut queue, t);
    }
    while let Some(q) = queue.pop() {
        if q.size < target {
            break;
        }
        if !h.alive[q.t as usize] || h.count(q.t) != 2 || h.connecting(q.t) {
            continue;
        }
        h.remove(&mut queue, q.t);
    }
    if holes {
        let mut seeds: Vec<u32> = (0..n as u32)
            .filter(|&t| {
                h.alive[t as usize]
                    && h.size[t as usize] >= target
                    && h.count(t) == 3
                    && !h.touches(t)
            })
            .collect();
        // Largest first (JTS's order), the larger area first among equal sizes, then the lower number.
        seeds.sort_by(|&a, &b| {
            h.size[b as usize]
                .total_cmp(&h.size[a as usize])
                .then(h.area(b).total_cmp(&h.area(a)))
                .then(a.cmp(&b))
        });
        for seed in seeds {
            if !h.alive[seed as usize] || h.count(seed) != 3 || h.touches(seed) {
                continue;
            }
            let mut queue = BinaryHeap::new();
            h.remove(&mut queue, seed);
            while let Some(q) = queue.pop() {
                if q.size < target {
                    break;
                }
                if !h.alive[q.t as usize] || h.count(q.t) != 2 || h.touches(q.t) {
                    continue;
                }
                h.remove(&mut queue, q.t);
            }
        }
    }
    Some(rings_of(&h))
}

/// The triangles left as rings, their border edges followed corner to corner.
fn rings_of(h: &Hull<'_>) -> Vec<Vec<Vec2>> {
    let mut to = vec![NONE; h.pts.len()];
    for t in 0..h.alive.len() as u32 {
        if !h.alive[t as usize] {
            continue;
        }
        for i in 0..3 {
            if h.adjacent(t, i).is_none() {
                let a = h.d.triangles[(3 * t + i) as usize];
                let b = h.d.triangles[(3 * t + (i + 1) % 3) as usize];
                if to[a as usize] == NONE {
                    to[a as usize] = b;
                }
            }
        }
    }
    let mut rings: Vec<Vec<Vec2>> = Vec::new();
    let mut seen = vec![false; h.pts.len()];
    for s in 0..h.pts.len() {
        if to[s] == NONE || seen[s] {
            continue;
        }
        let mut ring = Vec::new();
        let mut v = s;
        while !seen[v] && to[v] != NONE {
            seen[v] = true;
            ring.push(h.pts[v]);
            v = to[v] as usize;
        }
        if ring.len() >= 3 {
            rings.push(normalised(ring));
        }
    }
    // The outer (counter-clockwise: positive area) first, the holes by their lowest corner.
    let signed = |r: &Vec<Vec2>| {
        let o = r[0];
        let mut s = 0.0;
        for i in 1..r.len() - 1 {
            s += (r[i].x - o.x) * (r[i + 1].y - o.y) - (r[i + 1].x - o.x) * (r[i].y - o.y);
        }
        s
    };
    let (mut outer, mut inner): (Vec<_>, Vec<_>) = rings.into_iter().partition(|r| signed(r) > 0.0);
    inner.sort_by(|a, b| a[0].y.total_cmp(&b[0].y).then(a[0].x.total_cmp(&b[0].x)));
    outer.extend(inner);
    outer
}
