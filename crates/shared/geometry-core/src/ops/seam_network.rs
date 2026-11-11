//! A mosaic's seamline network and a raster's footprint from its pixels (plan/map-cad.md MAP-12, docs/adr/5260).
//!
//! **Network.** Every place of the footprints' union goes to the image, among those whose (convex) footprint covers
//! it, whose footprint's centre (its area centroid) is nearest; the first of equals. Image i's element is its footprint
//! less, for every other image j, the part of j's footprint nearer j's centre (j before i: or as near), each such part
//! a convex polygon clipped by the bisector; the difference is the overlay engine's. The arithmetic runs about the first
//! footprint's first corner. A ring is counter-clockwise (a hole clockwise), its collinear corners dropped, started at
//! its southernmost corner (the westernmost of equals); parts follow by their starts. A junction is a corner three or
//! more elements share (within 10⁻⁶ of the coordinates' size).
//!
//! **Outline.** The valid pixels' boundary on their edges, by 4-connectivity: a corner two valid pixels share only
//! diagonally keeps them apart. Corners are pixel corners (column, row); the rings are traced with the valid pixels on
//! the right in (column, row) (rows down), so an outer ring runs clockwise there and a hole anticlockwise; collinear
//! corners dropped. The hull is the valid pixels' corners' convex hull (Andrew's monotone chain), counter-clockwise in
//! (column, row) turned to the ground by the caller.

use std::collections::HashMap;

use crate::geom::arrangement::{Area, Ring};
use crate::geom::region::subtract_areas;
use crate::vec2::Vec2;

pub type Pt = [f64; 2];

/// A part of an element: its outer ring and its holes.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub ring: Vec<Pt>,
    pub holes: Vec<Vec<Pt>>,
}

/// An image's element: its parts and area.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub parts: Vec<Part>,
    pub area: f64,
}

/// The network: each image's element (none when it keeps nothing) and the junctions (their place and images).
#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    pub elements: Vec<Element>,
    pub junctions: Vec<(Pt, Vec<usize>)>,
}

/// Twice the ring's signed area (counter-clockwise positive).
pub fn signed2(r: &[Pt]) -> f64 {
    let n = r.len();
    (0..n).map(|k| {
        let (a, b) = (r[k], r[(k + 1) % n]);
        a[0] * b[1] - b[0] * a[1]
    })
    .sum()
}

/// The area centroid of a simple ring.
pub fn centroid(r: &[Pt]) -> Pt {
    let n = r.len();
    let o = r[0];
    let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for k in 0..n {
        let (p, q) = ([r[k][0] - o[0], r[k][1] - o[1]], [r[(k + 1) % n][0] - o[0], r[(k + 1) % n][1] - o[1]]);
        let c = p[0] * q[1] - q[0] * p[1];
        a2 += c;
        cx += (p[0] + q[0]) * c;
        cy += (p[1] + q[1]) * c;
    }
    [o[0] + cx / (3.0 * a2), o[1] + cy / (3.0 * a2)]
}

/// The convex polygon `poly` (counter-clockwise) kept where `n · p ≤ c` (a point on the line kept).
fn clip(poly: &[Pt], n: Pt, c: f64) -> Vec<Pt> {
    let side = |p: Pt| n[0] * p[0] + n[1] * p[1] - c;
    let mut out = Vec::new();
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        let (sa, sb) = (side(a), side(b));
        if sa <= 0.0 {
            out.push(a);
        }
        if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
            let t = sa / (sa - sb);
            out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    out
}

fn area_of(ring: &[Pt]) -> Area {
    Area { outer: Ring { pts: ring.iter().map(|p| Vec2::new(p[0], p[1])).collect(), bulges: None }, holes: Vec::new() }
}

/// The ring tidied (module's rule): closing repeat and collinear corners dropped, turned `ccw` or not, started at its
/// southernmost corner (westernmost of equals). `tol` is the length below which a corner's turn is none.
pub fn tidy(ring: &[Pt], ccw: bool, tol: f64) -> Vec<Pt> {
    let mut r: Vec<Pt> = ring.to_vec();
    if r.len() > 1 && r.first() == r.last() {
        r.pop();
    }
    r.dedup_by(|a, b| (a[0] - b[0]).abs() <= tol && (a[1] - b[1]).abs() <= tol);
    loop {
        let n = r.len();
        if n < 3 {
            break;
        }
        let drop = (0..n).find(|&k| {
            let (a, b, c) = (r[(k + n - 1) % n], r[k], r[(k + 1) % n]);
            let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            let cross = u[0] * v[1] - u[1] * v[0];
            let len = (u[0].hypot(u[1])).max(v[0].hypot(v[1]));
            cross.abs() <= tol * len || u[0].hypot(u[1]) <= tol
        });
        match drop {
            Some(k) => {
                r.remove(k);
            }
            None => break,
        }
    }
    if (signed2(&r) > 0.0) != ccw {
        r.reverse();
    }
    if let Some(s) = (0..r.len()).min_by(|&a, &b| r[a][1].total_cmp(&r[b][1]).then(r[a][0].total_cmp(&r[b][0]))) {
        r.rotate_left(s);
    }
    r
}

/// The network of `footprints` (convex, counter-clockwise, at least three corners each); `tol` the length below which
/// two corners are one (10⁻⁶ of the coordinates' size is the caller's).
pub fn network(footprints: &[Vec<Pt>], tol: f64) -> Network {
    let o = footprints.first().and_then(|f| f.first()).copied().unwrap_or([0.0, 0.0]);
    let local: Vec<Vec<Pt>> = footprints.iter().map(|f| f.iter().map(|p| [p[0] - o[0], p[1] - o[1]]).collect()).collect();
    let centres: Vec<Pt> = local.iter().map(|f| centroid(f)).collect();
    let mut elements = Vec::new();
    for (i, fi) in local.iter().enumerate() {
        let mut cutters = Vec::new();
        for (j, fj) in local.iter().enumerate() {
            if i == j {
                continue;
            }
            let (ci, cj) = (centres[i], centres[j]);
            // Nearer j than i: 2 p · (ci − cj) ≤ |ci|² − |cj|² (on the line: j's when it comes first; the line itself
            // has no area, so both sides keep it and the overlay draws it once).
            let n = [2.0 * (ci[0] - cj[0]), 2.0 * (ci[1] - cj[1])];
            let c = ci[0] * ci[0] + ci[1] * ci[1] - cj[0] * cj[0] - cj[1] * cj[1];
            let part = if n[0] == 0.0 && n[1] == 0.0 {
                if j < i { fj.clone() } else { Vec::new() }
            } else {
                clip(fj, n, c)
            };
            if part.len() >= 3 && signed2(&part).abs() > tol * tol {
                cutters.push(area_of(&part));
            }
        }
        let rest = subtract_areas(&[area_of(fi)], &cutters);
        let ring = |r: &Ring, ccw: bool| -> Vec<Pt> {
            tidy(&r.pts.iter().map(|v| [v.x + o[0], v.y + o[1]]).collect::<Vec<_>>(), ccw, tol)
        };
        let mut parts: Vec<Part> = rest
            .iter()
            .map(|a| Part { ring: ring(&a.outer, true), holes: a.holes.iter().map(|h| ring(h, false)).collect() })
            .filter(|p| p.ring.len() >= 3)
            .collect();
        for p in &mut parts {
            p.holes.retain(|h| h.len() >= 3);
            p.holes.sort_by(|a, b| a[0][1].total_cmp(&b[0][1]).then(a[0][0].total_cmp(&b[0][0])));
        }
        parts.sort_by(|a, b| a.ring[0][1].total_cmp(&b.ring[0][1]).then(a.ring[0][0].total_cmp(&b.ring[0][0])));
        let area = parts
            .iter()
            .map(|p| {
                let rel = |r: &[Pt]| signed2(&r.iter().map(|q| [q[0] - o[0], q[1] - o[1]]).collect::<Vec<_>>()) / 2.0;
                rel(&p.ring) + p.holes.iter().map(|h| rel(h)).sum::<f64>()
            })
            .sum();
        elements.push(Element { parts, area });
    }
    let mut corners: Vec<(Pt, Vec<usize>)> = Vec::new();
    for (i, e) in elements.iter().enumerate() {
        for p in &e.parts {
            for q in p.ring.iter().chain(p.holes.iter().flatten()) {
                match corners.iter_mut().find(|(at, _)| (at[0] - q[0]).abs() <= tol && (at[1] - q[1]).abs() <= tol) {
                    Some((_, who)) => {
                        if !who.contains(&i) {
                            who.push(i);
                        }
                    }
                    None => corners.push((*q, vec![i])),
                }
            }
        }
    }
    let mut junctions: Vec<(Pt, Vec<usize>)> = corners
        .into_iter()
        .filter(|(_, who)| who.len() >= 3)
        .map(|(at, mut who)| {
            who.sort_unstable();
            (at, who)
        })
        .collect();
    junctions.sort_by(|a, b| a.0[1].total_cmp(&b.0[1]).then(a.0[0].total_cmp(&b.0[0])));
    Network { elements, junctions }
}

/// The valid pixels' outline (module's rule): rings of pixel corners (column, row), outer rings clockwise in (column,
/// row), holes anticlockwise; and whether each is a hole.
pub fn outline(valid: &[bool], w: usize, h: usize) -> Vec<(Vec<[i64; 2]>, bool)> {
    let at = |c: i64, r: i64| c >= 0 && r >= 0 && (c as usize) < w && (r as usize) < h && valid[r as usize * w + c as usize];
    // Directed edges with the valid pixel on the right (rows down): top edge left→right, right edge down, bottom right→
    // left, left edge up.
    let mut out_of: HashMap<[i64; 2], Vec<[i64; 2]>> = HashMap::new();
    for r in 0..h as i64 {
        for c in 0..w as i64 {
            if !at(c, r) {
                continue;
            }
            if !at(c, r - 1) {
                out_of.entry([c, r]).or_default().push([c + 1, r]);
            }
            if !at(c + 1, r) {
                out_of.entry([c + 1, r]).or_default().push([c + 1, r + 1]);
            }
            if !at(c, r + 1) {
                out_of.entry([c + 1, r + 1]).or_default().push([c, r + 1]);
            }
            if !at(c - 1, r) {
                out_of.entry([c, r + 1]).or_default().push([c, r]);
            }
        }
    }
    let mut starts: Vec<[i64; 2]> = out_of.keys().copied().collect();
    starts.sort_by_key(|p| (p[1], p[0]));
    let mut rings = Vec::new();
    for s in starts {
        while let Some(next) = out_of.get_mut(&s).and_then(|v| v.pop()) {
            let mut ring = vec![s];
            let (mut prev, mut cur) = (s, next);
            while cur != s {
                ring.push(cur);
                let d = [cur[0] - prev[0], cur[1] - prev[1]];
                let choices = out_of.get_mut(&cur).expect("a boundary corner has a way on");
                // Two ways on (two valid pixels touching only at this corner): turn right, keeping the pixel just
                // passed apart from its diagonal neighbour (4-connectivity). Right of (dx, dy) with rows down is
                // (−dy, dx).
                let right = [-d[1], d[0]];
                let k = if choices.len() > 1 {
                    choices.iter().position(|q| [q[0] - cur[0], q[1] - cur[1]] == right).unwrap_or(0)
                } else {
                    0
                };
                let q = choices.remove(k);
                prev = cur;
                cur = q;
            }
            rings.push(ring);
        }
    }
    rings
        .into_iter()
        .map(|r| {
            let mut t: Vec<[i64; 2]> = Vec::new();
            let n = r.len();
            for k in 0..n {
                let (a, b, c) = (r[(k + n - 1) % n], r[k], r[(k + 1) % n]);
                let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
                if cross != 0 {
                    t.push(b);
                }
            }
            // Clockwise in (column, row) with rows down is a positive shoelace in those numbers: an outer ring.
            let s2: i64 = (0..t.len()).map(|k| t[k][0] * t[(k + 1) % t.len()][1] - t[(k + 1) % t.len()][0] * t[k][1]).sum();
            (t, s2 < 0)
        })
        .collect()
}

/// The convex hull (Andrew's monotone chain) of the valid pixels' corners, counter-clockwise in (column, row) taken as
/// x and y; collinear corners dropped.
pub fn hull(valid: &[bool], w: usize, h: usize) -> Vec<[i64; 2]> {
    let mut pts: Vec<[i64; 2]> = Vec::new();
    for r in 0..h {
        let row = &valid[r * w..(r + 1) * w];
        let (Some(first), Some(last)) = (row.iter().position(|v| *v), row.iter().rposition(|v| *v)) else { continue };
        for c in [first, last + 1] {
            pts.push([c as i64, r as i64]);
            pts.push([c as i64, r as i64 + 1]);
        }
    }
    pts.sort_unstable();
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: [i64; 2], a: [i64; 2], b: [i64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut lower: Vec<[i64; 2]> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<[i64; 2]> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_squares_meet_at_their_centres_circumcentre() {
        let sq = |x: f64, y: f64| vec![[x, y], [x + 10.0, y], [x + 10.0, y + 10.0], [x, y + 10.0]];
        let n = network(&[sq(0.0, 0.0), sq(6.0, 0.0), sq(3.0, 6.0)], 1e-9);
        let total: f64 = n.elements.iter().map(|e| e.area).sum();
        // The union: two overlapping squares (10 × 16) and the third's 10 × 10 less its overlap with them (10 × 4).
        assert!((total - (160.0 + 100.0 - 40.0)).abs() < 1e-9, "{total}");
        assert_eq!(n.junctions.len(), 1, "{:?}", n.junctions);
        let ([x, y], who) = &n.junctions[0];
        assert_eq!(who, &vec![0, 1, 2]);
        // The centres (5, 5), (11, 5), (8, 11): the circumcentre is on x = 8, 9 + (y − 5)² = (y − 11)², y = 87/12.
        assert!((x - 8.0).abs() < 1e-9 && (y - 7.25).abs() < 1e-9, "{x} {y}");
    }

    #[test]
    fn the_outline_keeps_holes_and_diagonal_pixels_apart() {
        // 4 × 3: a ring of valid pixels round one hole, and a lone pixel touching it only at a corner.
        let v = [true, true, true, false, true, false, true, false, true, true, true, false];
        let mut v2 = v.to_vec();
        v2.extend([false, false, false, true]);
        let rings = outline(&v2, 4, 4);
        let holes = rings.iter().filter(|r| r.1).count();
        assert_eq!((rings.len(), holes), (3, 1), "{rings:?}");
        assert_eq!(hull(&v, 4, 3), vec![[0, 0], [3, 0], [3, 3], [0, 3]]);
    }
}
