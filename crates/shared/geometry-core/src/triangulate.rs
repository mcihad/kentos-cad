//! Ear-clipping triangulation of a ring with holes (`apps/web/src/render/triangulate.ts`),
//! ported operation for operation (docs/adr/0008). Holes are first joined to
//! the outer ring by bridges (Eberly, "Triangulation by Ear Clipping"), giving
//! one weakly simple ring for the ear clipper.
//!
//! Triangles are vertex indices into the caller's points: a bridge only
//! repeats vertices, it never makes new ones, so a fill built from the indices
//! carries the input's own coordinates (the renderer makes them
//! origin-relative, CLAUDE.md §4.9). Many polygons go in one call
//! ([`triangulate_many`]), as a layer's fills do.

use std::ops::Range;

use crate::api::Op;
use crate::geometry::signed_area;
use crate::jsmath::{atan2, js_cmp, js_hypot, js_max, js_min, stable_sort};
use crate::op;
use crate::vec2::Vec2;

/// Ear-clipping rounds before giving up on a ring (the TypeScript's guard).
const GUARD: usize = 100_000;

/// Appends the triangles of one polygon to `out`, three vertex indices (into
/// `pts`) per triangle. `rings` are ranges of `pts`: the outer ring, then its
/// holes. Holes of fewer than three vertices are ignored, as is a hole
/// outside the ring.
pub fn triangulate_into(pts: &[Vec2], rings: &[Range<usize>], out: &mut Vec<u32>) {
    let Some(outer) = rings.first() else {
        return;
    };
    let mut ring = orient(pts, outer.clone(), true);
    // Rightmost holes first: a later bridge then never has to cross an earlier one.
    let mut hs: Vec<(Vec<usize>, usize)> = rings[1..]
        .iter()
        .filter(|h| h.len() >= 3)
        .map(|h| {
            let h = orient(pts, h.clone(), false);
            let m = rightmost(pts, &h);
            (h, m)
        })
        .collect();
    stable_sort(&mut hs, &mut |a, b| {
        js_cmp(pts[b.0[b.1]].x - pts[a.0[a.1]].x, 0.0)
    });
    for (h, m) in &hs {
        ring = bridge(pts, ring, h, *m);
    }
    ear_clip(pts, ring, out);
}

/// The triangles of one polygon as origin-relative coordinates (x, y for each
/// corner), the way the TypeScript wrote them into a fill.
pub fn triangulate(outer: &[Vec2], holes: &[Vec<Vec2>], origin: Vec2) -> Vec<f64> {
    let mut pts = Vec::with_capacity(outer.len() + holes.iter().map(Vec::len).sum::<usize>());
    let mut rings = Vec::with_capacity(holes.len() + 1);
    for r in std::iter::once(outer).chain(holes.iter().map(Vec::as_slice)) {
        rings.push(pts.len()..pts.len() + r.len());
        pts.extend_from_slice(r);
    }
    let mut idx = Vec::new();
    triangulate_into(&pts, &rings, &mut idx);
    let mut out = Vec::with_capacity(idx.len() * 2);
    for i in idx {
        let p = pts[i as usize];
        out.push(p.x - origin.x);
        out.push(p.y - origin.y);
    }
    out
}

/// The triangles of many polygons in one call: `pts` holds every ring one
/// after another, `ring_sizes` each ring's vertex count and `poly_rings` each
/// polygon's ring count (its outer ring, then its holes). The result is three
/// vertex indices (into `pts`) per triangle, polygon after polygon. Sizes
/// beyond the data are clamped, never a panic.
pub fn triangulate_many(pts: &[Vec2], ring_sizes: &[usize], poly_rings: &[usize]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut rings: Vec<Range<usize>> = Vec::new();
    let mut next_ring = 0usize;
    let mut at = 0usize;
    for &k in poly_rings {
        rings.clear();
        for _ in 0..k {
            let n = ring_sizes.get(next_ring).copied().unwrap_or(0);
            let start = at.min(pts.len());
            let end = start.saturating_add(n).min(pts.len());
            rings.push(start..end);
            at = end;
            next_ring += 1;
        }
        triangulate_into(pts, &rings, &mut out);
    }
    out
}

/// The ring's vertex indices, counter-clockwise when `ccw`, else clockwise.
fn orient(pts: &[Vec2], r: Range<usize>, ccw: bool) -> Vec<usize> {
    if (signed_area(&pts[r.clone()]) > 0.0) == ccw {
        r.collect()
    } else {
        r.rev().collect()
    }
}

/// Position (in `ring`) of the rightmost vertex, the lowest of equals.
fn rightmost(pts: &[Vec2], ring: &[usize]) -> usize {
    let mut m = 0;
    for i in 1..ring.len() {
        let (p, q) = (pts[ring[i]], pts[ring[m]]);
        if p.x > q.x || (p.x == q.x && p.y < q.y) {
            m = i;
        }
    }
    m
}

fn cross(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn same(p: Vec2, q: Vec2) -> bool {
    p.x == q.x && p.y == q.y
}

fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
}

/// Joins a clockwise hole to a counter-clockwise ring: from the hole's
/// rightmost vertex M a ray to the right meets the ring at I on an edge; the
/// edge's right end P is visible from M unless a reflex vertex lies in
/// triangle M-I-P, in which case the one closest in angle to the ray is.
fn bridge(pts: &[Vec2], ring: Vec<usize>, hole: &[usize], mi: usize) -> Vec<usize> {
    let m = pts[hole[mi]];
    let mut best_x = f64::INFINITY;
    let mut edge = None;
    let n = ring.len();
    for i in 0..n {
        let a = pts[ring[i]];
        let b = pts[ring[(i + 1) % n]];
        if a.y == b.y || m.y < js_min(a.y, b.y) || m.y > js_max(a.y, b.y) {
            continue;
        }
        let x = a.x + ((m.y - a.y) * (b.x - a.x)) / (b.y - a.y);
        if x >= m.x && x < best_x {
            best_x = x;
            edge = Some(i);
        }
    }
    // A hole outside the ring: nothing sensible to join.
    let Some(edge) = edge else {
        return ring;
    };
    let hit = Vec2::new(best_x, m.y);
    let mut pi = if pts[ring[edge]].x >= pts[ring[(edge + 1) % n]].x {
        edge
    } else {
        (edge + 1) % n
    };
    let p = pts[ring[pi]];
    if !same(p, hit) {
        let mut best_angle = f64::INFINITY;
        let mut best_dist = f64::INFINITY;
        // M, I, P as a counter-clockwise triangle for the inside test.
        let (t0, t1, t2) = if cross(m, hit, p) >= 0.0 {
            (m, hit, p)
        } else {
            (m, p, hit)
        };
        for i in 0..n {
            // `pi` moves as better candidates are found, as in the TypeScript.
            if i == pi {
                continue;
            }
            let v = pts[ring[i]];
            let reflex = cross(pts[ring[(i + n - 1) % n]], v, pts[ring[(i + 1) % n]]) < 0.0;
            if !reflex || !in_triangle(v, t0, t1, t2) {
                continue;
            }
            let ang = atan2(v.y - m.y, v.x - m.x).abs();
            let d = js_hypot(v.x - m.x, v.y - m.y);
            if ang < best_angle - 1e-12 || ((ang - best_angle).abs() <= 1e-12 && d < best_dist) {
                best_angle = ang;
                best_dist = d;
                pi = i;
            }
        }
    }
    let mut out = Vec::with_capacity(n + hole.len() + 2);
    out.extend_from_slice(&ring[..=pi]);
    out.extend_from_slice(&hole[mi..]);
    out.extend_from_slice(&hole[..=mi]);
    out.push(ring[pi]);
    out.extend_from_slice(&ring[pi + 1..]);
    out
}

/// Ear clipping of a counter-clockwise (weakly simple) ring of vertex indices.
fn ear_clip(pts: &[Vec2], mut idx: Vec<usize>, out: &mut Vec<u32>) {
    let mut guard = 0;
    while idx.len() > 3 && guard < GUARD {
        guard += 1;
        let mut clipped = false;
        let len = idx.len();
        for i in 0..len {
            let ia = idx[(i + len - 1) % len];
            let ib = idx[i];
            let ic = idx[(i + 1) % len];
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            let mut ear = true;
            for &k in &idx {
                // A vertex repeated by a bridge has the same index or the
                // same coordinates: either way a copy of a corner does not
                // block the ear.
                if k == ia || k == ib || k == ic {
                    continue;
                }
                let p = pts[k];
                if same(p, a) || same(p, b) || same(p, c) {
                    continue;
                }
                if in_triangle(p, a, b, c) {
                    ear = false;
                    break;
                }
            }
            if !ear {
                continue;
            }
            out.extend([ia as u32, ib as u32, ic as u32]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // Only flat (zero-area) corners left, e.g. a bridge's two edges: drop one.
            let len = idx.len();
            let flat = (0..len).position(|i| {
                cross(
                    pts[idx[(i + len - 1) % len]],
                    pts[idx[i]],
                    pts[idx[(i + 1) % len]],
                ) == 0.0
            });
            match flat {
                Some(f) => {
                    idx.remove(f);
                }
                None => return, // degenerate ring; drop the remainder
            }
        }
    }
    if idx.len() == 3 && cross(pts[idx[0]], pts[idx[1]], pts[idx[2]]) > 0.0 {
        out.extend([idx[0] as u32, idx[1] as u32, idx[2] as u32]);
    }
}

pub(crate) static OPS: &[Op] =
    &[op!("triangulate", |outer: Vec<Vec2>,
                          holes: Vec<Vec<Vec2>>,
                          origin: Vec2| {
        triangulate(&outer, &holes, origin)
    })];

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn square(x0: f64, y0: f64, s: f64) -> Vec<Vec2> {
        vec![v(x0, y0), v(x0 + s, y0), v(x0 + s, y0 + s), v(x0, y0 + s)]
    }

    /// Sum of the triangle areas; every triangle must be counter-clockwise.
    fn covered(pts: &[Vec2], idx: &[u32]) -> f64 {
        assert_eq!(idx.len() % 3, 0);
        idx.chunks_exact(3)
            .map(|t| {
                let a = cross(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]) / 2.0;
                assert!(a >= 0.0, "clockwise triangle {t:?}");
                a
            })
            .sum()
    }

    #[test]
    fn a_square_with_a_hole_either_orientation() {
        let mut pts = square(0.0, 0.0, 10.0);
        pts.reverse();
        pts.extend(square(4.0, 4.0, 2.0));
        let mut idx = Vec::new();
        triangulate_into(&pts, &[0..4, 4..8], &mut idx);
        assert!((covered(&pts, &idx) - 96.0).abs() < 1e-9);
    }

    #[test]
    fn coordinates_are_the_inputs_own_minus_the_origin() {
        let o = v(486000.0, 4420000.0);
        let outer: Vec<Vec2> = square(0.0, 0.0, 10.0)
            .iter()
            .map(|p| v(p.x + o.x + 0.125, p.y + o.y + 0.375))
            .collect();
        let xy = triangulate(&outer, &[], o);
        assert_eq!(xy.len(), 2 * 3 * 2);
        for c in xy.chunks_exact(2) {
            assert!(outer.iter().any(|p| p.x - o.x == c[0] && p.y - o.y == c[1]));
        }
    }

    #[test]
    fn many_polygons_in_one_call_match_one_by_one() {
        let mut pts = square(0.0, 0.0, 10.0);
        pts.extend(square(2.0, 2.0, 1.0));
        pts.extend(square(6.0, 6.0, 2.0));
        pts.extend(square(20.0, 0.0, 5.0));
        pts.extend([v(40.0, 0.0), v(41.0, 0.0)]); // too short to fill
        let many = triangulate_many(&pts, &[4, 4, 4, 4, 2], &[3, 1, 1]);
        let mut one = Vec::new();
        triangulate_into(&pts, &[0..4, 4..8, 8..12], &mut one);
        triangulate_into(&pts, std::slice::from_ref(&(12..16)), &mut one);
        assert_eq!(many, one);
        assert!((covered(&pts, &many[..one.len()]) - (100.0 - 1.0 - 4.0 + 25.0)).abs() < 1e-9);
    }

    #[test]
    fn sizes_beyond_the_data_are_clamped_not_a_panic() {
        let pts = square(0.0, 0.0, 1.0);
        assert_eq!(triangulate_many(&pts, &[4], &[1]).len(), 6);
        assert_eq!(triangulate_many(&pts, &[9, 9], &[2, 3]).len(), 6);
        assert!(triangulate_many(&pts, &[], &[1, 1]).is_empty());
        assert!(triangulate_many(&[], &[3], &[1]).is_empty());
        let mut idx = Vec::new();
        triangulate_into(&pts, &[], &mut idx);
        assert!(idx.is_empty());
    }
}
