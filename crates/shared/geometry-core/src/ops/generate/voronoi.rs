//! Voronoi alanları and Delaunay üçgenleri (docs/adr/0219 §2, §3) over the
//! sites' exact Delaunay triangulation (`geom::delaunay`, docs/adr/0232 §4).
//! A site's cell is the centres of the triangles round it in turn; a site on
//! the hull has an open cell, its two hull edges' bisectors running out from
//! their triangles' centres; every cell is cut to the frame. Neighbouring
//! cells share their corners bit for bit: a triangle's centre is worked out
//! once (cocircular neighbours, `incircle` exactly zero, share the first
//! one's), a ray's far point once from its triangle's centre, and an edge
//! meets the frame where both cells work it out alike, from its end inside.

use std::collections::HashMap;
use std::f64::consts::FRAC_PI_4;

use crate::geom::delaunay::{Delaunay, MOST_POINTS, NONE, TOO_FEW, next, prev, triangulate};
use crate::jsmath::{atan2, cos, js_hypot, js_max, js_min, sin};
use crate::predicates::{incircle, orient2d};
use crate::vec2::Vec2;

/// Why the sites have no Voronoi cells.
pub const TOO_FEW_SITES: &str = "Voronoi alanları için en az iki ayrı yerde nokta gerekir.";

/// A box along the axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub min: Vec2,
    pub max: Vec2,
}

/// A site's cell.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    /// Its ring inside the frame, counter-clockwise from its lowest corner (the westmost of equal ones).
    Ring(Vec<Vec2>),
    /// In the same place as an earlier site: that one has the cell.
    Same,
    /// No area inside the frame.
    Empty,
}

/// The frame round the sites: their box grown on every side by `margin` (0.1 for 10 %) of its longer side.
pub fn frame_of(sites: &[Vec2], margin: f64) -> Option<Frame> {
    let first = sites.first()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
    for p in sites {
        x0 = js_min(x0, p.x);
        y0 = js_min(y0, p.y);
        x1 = js_max(x1, p.x);
        y1 = js_max(y1, p.y);
    }
    let grow = js_max(x1 - x0, y1 - y0) * margin;
    Some(Frame {
        min: Vec2::new(x0 - grow, y0 - grow),
        max: Vec2::new(x1 + grow, y1 + grow),
    })
}

/// The sites in their own places: each distinct place once, in the order of
/// its first site, and for every site its place's number and whether it is
/// that place's first.
pub struct Distinct {
    pub places: Vec<Vec2>,
    pub place_of: Vec<usize>,
    pub first: Vec<bool>,
    /// For every place its first site.
    pub site_of: Vec<usize>,
}

/// A place's key: its coordinates' bits, −0 as 0.
fn key(p: Vec2) -> (u64, u64) {
    ((p.x + 0.0).to_bits(), (p.y + 0.0).to_bits())
}

pub fn distinct(sites: &[Vec2]) -> Distinct {
    let mut seen: HashMap<(u64, u64), usize> = HashMap::with_capacity(sites.len());
    let mut d = Distinct {
        places: Vec::with_capacity(sites.len()),
        place_of: Vec::with_capacity(sites.len()),
        first: Vec::with_capacity(sites.len()),
        site_of: Vec::with_capacity(sites.len()),
    };
    for (i, &p) in sites.iter().enumerate() {
        match seen.get(&key(p)) {
            Some(&k) => {
                d.place_of.push(k);
                d.first.push(false);
            }
            None => {
                let k = d.places.len();
                seen.insert(key(p), k);
                d.places.push(p);
                d.site_of.push(i);
                d.place_of.push(k);
                d.first.push(true);
            }
        }
    }
    d
}

/// The centre of the circle through a, b and c (counter-clockwise, not on
/// one line): worked out from a, the turn exact in sign and close in size
/// (`orient2d`), so a sliver's far centre keeps its direction.
pub fn centre(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (ex, ey) = (c.x - a.x, c.y - a.y);
    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / orient2d(a, b, c);
    Vec2::new(a.x + (ey * bl - dy * cl) * d, a.y + (dx * cl - ex * bl) * d)
}

/// The unit vector to the right of `u` (outward of a counter-clockwise hull's edge).
fn right(u: Vec2) -> Vec2 {
    let l = js_hypot(u.x, u.y);
    Vec2::new(u.y / l, -u.x / l)
}

fn find(root: &mut [u32], mut t: u32) -> u32 {
    while root[t as usize] != t {
        let up = root[root[t as usize] as usize];
        root[t as usize] = up;
        t = up;
    }
    t
}

/// Every triangle's centre; triangles on one circle with a neighbour share the lowest-numbered one's.
fn centres(pts: &[Vec2], d: &Delaunay) -> Vec<Vec2> {
    let tris = d.triangles.len() / 3;
    let mut root: Vec<u32> = (0..tris as u32).collect();
    for e in 0..d.halfedges.len() {
        let tw = d.halfedges[e];
        if tw == NONE || (tw as usize) < e {
            continue;
        }
        let at = |h: u32| pts[d.triangles[h as usize] as usize];
        let e32 = e as u32;
        if incircle(at(e32), at(next(e32)), at(prev(e32)), at(prev(tw))) == 0.0 {
            let (a, b) = (find(&mut root, e32 / 3), find(&mut root, tw / 3));
            if a != b {
                let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                root[hi as usize] = lo;
            }
        }
    }
    let mut out = vec![Vec2::default(); tris];
    for t in 0..tris {
        let r = find(&mut root, t as u32) as usize;
        if r == t {
            let at = |i: usize| pts[d.triangles[3 * t + i] as usize];
            out[t] = centre(at(0), at(1), at(2));
        }
    }
    for t in 0..tris {
        let r = find(&mut root, t as u32) as usize;
        if r != t {
            out[t] = out[r];
        }
    }
    out
}

/// A ring cut to the frame (Sutherland–Hodgman, its sides in a fixed order).
/// Where an edge crosses a side the point is worked out from its end inside,
/// so the two cells sharing the edge get the same point.
pub fn clip(mut ring: Vec<Vec2>, f: Frame) -> Vec<Vec2> {
    for side in 0..4 {
        if ring.is_empty() {
            break;
        }
        let inside = |p: Vec2| match side {
            0 => p.x >= f.min.x,
            1 => p.x <= f.max.x,
            2 => p.y >= f.min.y,
            _ => p.y <= f.max.y,
        };
        let meet = |a: Vec2, b: Vec2| -> Vec2 {
            match side {
                0 | 1 => {
                    let x = if side == 0 { f.min.x } else { f.max.x };
                    Vec2::new(x, a.y + (b.y - a.y) * ((x - a.x) / (b.x - a.x)))
                }
                _ => {
                    let y = if side == 2 { f.min.y } else { f.max.y };
                    Vec2::new(a.x + (b.x - a.x) * ((y - a.y) / (b.y - a.y)), y)
                }
            }
        };
        let n = ring.len();
        let mut out = Vec::with_capacity(n + 4);
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let (ia, ib) = (inside(a), inside(b));
            if ia {
                out.push(a);
            }
            if ia != ib {
                out.push(if ia { meet(a, b) } else { meet(b, a) });
            }
        }
        ring = out;
    }
    ring
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

/// A ring without repeated corners (its closing one either), counter-clockwise
/// as it is, from its lowest corner (the westmost of equal ones).
pub fn normalised(mut ring: Vec<Vec2>) -> Vec<Vec2> {
    ring.dedup();
    while ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    let k = (0..ring.len())
        .min_by(|&a, &b| {
            ring[a]
                .y
                .total_cmp(&ring[b].y)
                .then(ring[a].x.total_cmp(&ring[b].x))
        })
        .unwrap_or(0);
    ring.rotate_left(k);
    ring
}

/// A cut ring as a cell: none when fewer than three corners or no area are left.
fn cell_of(ring: Vec<Vec2>) -> Option<Vec<Vec2>> {
    let ring = normalised(ring);
    let finite = ring.iter().all(|p| p.x.is_finite() && p.y.is_finite());
    (finite && ring.len() >= 3 && doubled_area(&ring) > 0.0).then_some(ring)
}

/// The cells of distinct sites not all on one line.
fn cells_of(pts: &[Vec2], d: &Delaunay, f: Frame) -> Vec<Option<Vec<Vec2>>> {
    let n = pts.len();
    let cc = centres(pts, d);
    // One far distance for every ray: well past the frame from any centre and any site.
    let mid = Vec2::new((f.min.x + f.max.x) / 2.0, (f.min.y + f.max.y) / 2.0);
    let reach = |p: Vec2| js_hypot(p.x - mid.x, p.y - mid.y);
    let mut span = js_hypot(f.max.x - f.min.x, f.max.y - f.min.y);
    for &c in cc.iter().chain(pts) {
        let r = reach(c);
        if r.is_finite() {
            span = js_max(span, r);
        }
    }
    let far = 8.0 * span + 1.0;
    let out = d.out_edges(n);
    (0..n)
        .map(|v| {
            let start = out[v];
            if start == NONE {
                return None;
            }
            let mut ring: Vec<Vec2> = Vec::new();
            let mut h = start;
            let mut back_edge = NONE;
            for _ in 0..d.halfedges.len() {
                ring.push(cc[(h / 3) as usize]);
                let back = prev(h);
                let tw = d.halfedges[back as usize];
                if tw == NONE {
                    back_edge = back;
                    break;
                }
                h = tw;
                if h == start {
                    break;
                }
            }
            if back_edge != NONE {
                // On the hull: out along the bisector of the hull edge leaving v from its triangle's centre, the
                // centres, out along the bisector of the hull edge coming in, round at the far distance back.
                let p = pts[v];
                let to = pts[d.triangles[next(start) as usize] as usize];
                let from = pts[d.triangles[back_edge as usize] as usize];
                let d0 = right(Vec2::new(to.x - p.x, to.y - p.y));
                let d1 = right(Vec2::new(p.x - from.x, p.y - from.y));
                let (first, last) = (ring[0], ring[ring.len() - 1]);
                let mut poly = Vec::with_capacity(ring.len() + 6);
                poly.push(Vec2::new(first.x + far * d0.x, first.y + far * d0.y));
                poly.extend(ring);
                poly.push(Vec2::new(last.x + far * d1.x, last.y + far * d1.y));
                if orient2d(from, p, to) > 0.0 {
                    // The turn from d1 to d0, counter-clockwise, below a half turn on a convex hull.
                    let turn = atan2(d1.x * d0.y - d1.y * d0.x, d1.x * d0.x + d1.y * d0.y);
                    if turn > 0.0 {
                        let steps = (turn / FRAC_PI_4).ceil() as usize;
                        let a1 = atan2(d1.y, d1.x);
                        for j in 1..steps {
                            let a = a1 + turn * (j as f64) / (steps as f64);
                            poly.push(Vec2::new(p.x + far * cos(a), p.y + far * sin(a)));
                        }
                    }
                }
                ring = poly;
            }
            cell_of(clip(ring, f))
        })
        .collect()
}

/// The ring `poly` cut to the side of the bisector of p and q nearer to p.
fn cut(poly: &[Vec2], p: Vec2, q: Vec2) -> Vec<Vec2> {
    let (mx, my) = ((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
    let (nx, ny) = (q.x - p.x, q.y - p.y);
    let side = |v: Vec2| (v.x - mx) * nx + (v.y - my) * ny;
    let n = poly.len();
    let mut out = Vec::with_capacity(n + 1);
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (sa, sb) = (side(a), side(b));
        if sa <= 0.0 {
            out.push(a);
        }
        if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
            let t = sa / (sa - sb);
            out.push(Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

/// The cells of distinct sites all on one line: strips between the bisectors of each and its neighbours along it.
fn cells_on_a_line(pts: &[Vec2], f: Frame) -> Vec<Option<Vec<Vec2>>> {
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_by(|&a, &b| {
        pts[a]
            .x
            .total_cmp(&pts[b].x)
            .then(pts[a].y.total_cmp(&pts[b].y))
    });
    let frame = vec![
        f.min,
        Vec2::new(f.max.x, f.min.y),
        f.max,
        Vec2::new(f.min.x, f.max.y),
    ];
    let mut out = vec![None; pts.len()];
    for (k, &v) in order.iter().enumerate() {
        let mut ring = frame.clone();
        if k > 0 {
            ring = cut(&ring, pts[v], pts[order[k - 1]]);
        }
        if k + 1 < order.len() {
            ring = cut(&ring, pts[v], pts[order[k + 1]]);
        }
        out[v] = cell_of(ring);
    }
    out
}

/// Every site's Voronoi cell inside the frame (§2), in the sites' order: the
/// first site in a place has the cell, the others `Same`. Fewer than two
/// places is refused.
pub fn voronoi(sites: &[Vec2], frame: Frame) -> Result<Vec<Cell>, String> {
    let places = distinct(sites);
    if places.places.len() < 2 {
        return Err(TOO_FEW_SITES.into());
    }
    let cells = match triangulate(&places.places) {
        Ok(d) => cells_of(&places.places, &d, frame),
        Err(e) if places.places.len() > MOST_POINTS => return Err(e),
        Err(e)
            if places
                .places
                .iter()
                .any(|p| !p.x.is_finite() || !p.y.is_finite()) =>
        {
            return Err(e);
        }
        Err(_) => cells_on_a_line(&places.places, frame),
    };
    Ok((0..sites.len())
        .map(|i| {
            if !places.first[i] {
                return Cell::Same;
            }
            match &cells[places.place_of[i]] {
                Some(r) => Cell::Ring(r.clone()),
                None => Cell::Empty,
            }
        })
        .collect())
}

/// The Delaunay triangles of the sites (§3): each as three sites (the first in
/// a place), counter-clockwise from the earliest, in the order of their
/// sites. Fewer than three places, or all on one line, is refused.
pub fn delaunay_triangles(sites: &[Vec2]) -> Result<Vec<[usize; 3]>, String> {
    let places = distinct(sites);
    if places.places.len() < 3 {
        return Err(TOO_FEW.into());
    }
    let d = triangulate(&places.places)?;
    let mut tris: Vec<[usize; 3]> = d
        .triangles
        .chunks_exact(3)
        .map(|t| {
            let s = [0, 1, 2].map(|i| places.site_of[t[i] as usize]);
            let k = (0..3).min_by_key(|&i| s[i]).unwrap_or(0);
            [s[k], s[(k + 1) % 3], s[(k + 2) % 3]]
        })
        .collect();
    tris.sort_unstable();
    Ok(tris)
}
