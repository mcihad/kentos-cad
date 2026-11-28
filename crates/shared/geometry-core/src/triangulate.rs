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
//!
//! A polygon whose rings cross themselves or each other, or with a hole over
//! its outline, is no ring the ear clipper can fill: it fills what it can and
//! leaves the rest. [`fill_into`] (corners, not indices) fills such a polygon
//! as its outer ring less its holes, resolved by the overlay engine
//! (docs/adr/9228).

use std::ops::Range;

use crate::api::Op;
use crate::geom::arrangement::{Area, Ring};
use crate::geom::region::{subtract_areas, union_areas};
use crate::geometry::signed_area;
use crate::jsmath::{atan2, hypot, js_cmp, stable_sort};
use crate::op;
use crate::predicates::orient2d;
use crate::vec2::Vec2;

mod earcut;

/// Ear-clipping rounds before giving up on a ring (the TypeScript's guard).
const GUARD: usize = 100_000;

/// Appends the triangles of one polygon to `out`, three vertex indices (into
/// `pts`) per triangle. `rings` are ranges of `pts`: the outer ring, then its
/// holes. Holes of fewer than three vertices are ignored, as is a hole
/// outside the ring.
///
/// Where the ear clipper's triangles do not cover the polygon (holes touching
/// each other or the outline at a vertex), earcut's are taken if they do
/// (docs/adr/9228); a polygon whose rings cross is filled by neither, and
/// [`fill_into`] resolves it.
pub fn triangulate_into(pts: &[Vec2], rings: &[Range<usize>], out: &mut Vec<u32>) {
    let start = out.len();
    clip_into(pts, rings, out);
    if covers(pts, rings, &out[start..]) {
        return;
    }
    let mut other = Vec::new();
    earcut::earcut_into(pts, rings, &mut other);
    if covers(pts, rings, &other) {
        out.truncate(start);
        out.extend(other);
    }
}

/// The ear clipper: holes bridged to the outer ring, ears cut off.
fn clip_into(pts: &[Vec2], rings: &[Range<usize>], out: &mut Vec<u32>) {
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

/// Appends the triangles of one polygon to `out` as their corners, three a
/// triangle, each counter-clockwise; `rings` as for [`triangulate_into`].
///
/// Where the ear clipper's triangles do not cover the polygon's area (its
/// rings cross themselves or each other, a hole lies over the outline or
/// outside it), the polygon is filled as its outer ring less its holes: the
/// overlay engine resolves where they cross (inside where a ring winds round
/// a point) into simple areas, and those are clipped. Their triangles then
/// have corners the rings do not have, where the rings cross. Rings that
/// cross go there straight: a ring crossing itself has a shoelace area that
/// is not its own, and triangles that cover it can still be wrong. A valid
/// polygon's triangles are [`triangulate_into`]'s (docs/adr/9228).
pub fn fill_into(pts: &[Vec2], rings: &[Range<usize>], out: &mut Vec<Vec2>) {
    let Some(first) = rings.first() else {
        return;
    };
    if !crosses(pts, rings) {
        let mut idx = Vec::new();
        triangulate_into(pts, rings, &mut idx);
        if covers(pts, rings, &idx) {
            out.extend(idx.iter().map(|&i| pts[i as usize]));
            return;
        }
    }
    let ring = |r: &Range<usize>| Ring {
        pts: pts[r.clone()].to_vec(),
        bulges: None,
    };
    let outer = [Area {
        outer: ring(first),
        holes: Vec::new(),
    }];
    let holes: Vec<Area> = rings[1..]
        .iter()
        .filter(|h| h.len() >= 3)
        .map(|h| Area {
            outer: ring(h),
            holes: Vec::new(),
        })
        .collect();
    let areas = if holes.is_empty() {
        union_areas(&outer)
    } else {
        subtract_areas(&outer, &holes)
    };
    for a in &areas {
        let mut p = a.outer.pts.clone();
        let mut rr = Vec::with_capacity(1 + a.holes.len());
        rr.push(0..p.len());
        for h in &a.holes {
            rr.push(p.len()..p.len() + h.pts.len());
            p.extend_from_slice(&h.pts);
        }
        let mut ix = Vec::new();
        triangulate_into(&p, &rr, &mut ix);
        out.extend(ix.iter().map(|&i| p[i as usize]));
    }
}

/// Whether two edges of the rings cross where neither ends: a ring crossing
/// itself or another (edges meeting at a corner, or running along each other,
/// do not cross). Swept along the longer side of the rings' box, so that a
/// long thin polygon (a road's edge running north) keeps few edges open at
/// once; the turns exact (`predicates`), their signs alike with the axes
/// swapped.
fn crosses(pts: &[Vec2], rings: &[Range<usize>]) -> bool {
    let (mut lo, mut hi) = (
        Vec2::new(f64::INFINITY, f64::INFINITY),
        Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for r in rings.iter().filter(|r| r.len() >= 3) {
        for p in &pts[r.clone()] {
            lo = Vec2::new(
                if p.x < lo.x { p.x } else { lo.x },
                if p.y < lo.y { p.y } else { lo.y },
            );
            hi = Vec2::new(
                if p.x > hi.x { p.x } else { hi.x },
                if p.y > hi.y { p.y } else { hi.y },
            );
        }
    }
    let swap = hi.y - lo.y > hi.x - lo.x;
    let along = |p: Vec2| if swap { Vec2::new(p.y, p.x) } else { p };
    let mut edges: Vec<(f64, f64, Vec2, Vec2)> = Vec::new();
    for r in rings {
        let n = r.len();
        if n < 3 {
            continue;
        }
        for k in 0..n {
            let (a, b) = (along(pts[r.start + k]), along(pts[r.start + (k + 1) % n]));
            if !same(a, b) {
                let (x0, x1) = if a.x < b.x { (a.x, b.x) } else { (b.x, a.x) };
                edges.push((x0, x1, a, b));
            }
        }
    }
    edges.sort_by(|p, q| p.0.total_cmp(&q.0));
    let side = |o: f64| {
        if o > 0.0 {
            1
        } else if o < 0.0 {
            -1
        } else {
            0
        }
    };
    let mut active: Vec<usize> = Vec::new();
    for i in 0..edges.len() {
        let (x0, _, a, b) = edges[i];
        active.retain(|&k| edges[k].1 >= x0);
        let (ylo, yhi) = if a.y < b.y { (a.y, b.y) } else { (b.y, a.y) };
        for &k in &active {
            let (_, _, c, d) = edges[k];
            if (c.y < ylo && d.y < ylo) || (c.y > yhi && d.y > yhi) {
                continue;
            }
            let (o1, o2) = (side(orient2d(a, b, c)), side(orient2d(a, b, d)));
            let (o3, o4) = (side(orient2d(c, d, a)), side(orient2d(c, d, b)));
            if o1 * o2 < 0 && o3 * o4 < 0 {
                return true;
            }
        }
        active.push(i);
    }
    false
}

/// A ring's area about a point of its own: far from the origin (a parcel in
/// TM coordinates) the shoelace on raw coordinates loses the digits the
/// comparison in [`covers`] needs.
fn area_about(pts: &[Vec2], o: Vec2) -> f64 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (a.x - o.x) * (b.y - o.y) - (b.x - o.x) * (a.y - o.y)
        })
        .sum::<f64>()
        / 2.0
}

/// Whether the triangles (counter-clockwise, as the ear clippers make them)
/// cover the polygon: its outer ring's area less its holes', within a
/// billionth of the rings' areas together.
fn covers(pts: &[Vec2], rings: &[Range<usize>], idx: &[u32]) -> bool {
    let Some(first) = rings.first() else {
        return idx.is_empty();
    };
    let Some(&o) = pts.get(first.start) else {
        return idx.is_empty();
    };
    let outer = area_about(&pts[first.clone()], o).abs();
    let (mut holes, mut all) = (0.0, outer);
    for h in rings[1..].iter().filter(|h| h.len() >= 3) {
        let a = area_about(&pts[h.clone()], o).abs();
        holes += a;
        all += a;
    }
    let got: f64 = idx
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| cross(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]) / 2.0)
        .sum();
    (got - (outer - holes)).abs() <= 1e-9 * all
}

/// The triangles of one polygon as origin-relative coordinates (x, y for each
/// corner), the way the TypeScript wrote them into a fill; a polygon the ear
/// clipper cannot fill resolved as [`fill_into`] does.
pub fn triangulate(outer: &[Vec2], holes: &[Vec<Vec2>], origin: Vec2) -> Vec<f64> {
    let mut pts = Vec::with_capacity(outer.len() + holes.iter().map(Vec::len).sum::<usize>());
    let mut rings = Vec::with_capacity(holes.len() + 1);
    for r in std::iter::once(outer).chain(holes.iter().map(Vec::as_slice)) {
        rings.push(pts.len()..pts.len() + r.len());
        pts.extend_from_slice(r);
    }
    let mut corners = Vec::new();
    fill_into(&pts, &rings, &mut corners);
    let mut out = Vec::with_capacity(corners.len() * 2);
    for p in corners {
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

/// Whether the diagonal from `a` (between `prev` and `next` on the ring) to `b`
/// starts inside the ring: `b` lies in `a`'s corner (earcut's `locallyInside`).
fn locally_inside(prev: Vec2, a: Vec2, next: Vec2, b: Vec2) -> bool {
    if cross(prev, a, next) > 0.0 {
        cross(a, b, next) <= 0.0 && cross(a, prev, b) <= 0.0
    } else {
        cross(a, b, prev) > 0.0 || cross(a, next, b) > 0.0
    }
}

/// Of two passes of the ring through one point, whether the corner of the
/// first (`m`, between `m_prev` and `m_next`) holds the second's (earcut's
/// `sectorContainsSector`): the inner pass is the one to join.
fn holds_corner(m_prev: Vec2, m: Vec2, m_next: Vec2, p_prev: Vec2, p_next: Vec2) -> bool {
    cross(m_prev, m, p_prev) > 0.0 && cross(p_next, m, m_next) > 0.0
}

/// Joins a clockwise hole to a counter-clockwise ring: from the hole's
/// rightmost vertex M a ray to the right meets the ring at I on an edge; the
/// edge's right end P is visible from M unless a vertex lies in triangle
/// M-I-P, in which case the one closest in angle to the ray is.
///
/// Holes joined before are part of the ring: a bridge's two edges lie on one
/// line and its ends are vertices the ring passes twice. So the ray meets only
/// edges with the inside on their left (going up: of a bridge's two edges, the
/// one facing M), and a vertex is joined only where its corner holds M, at its
/// innermost pass. Without both a hole could be joined across another's
/// bridge: the ring overlapped itself, the ear clipper found no ear and left
/// the rest unfilled (the SİVAS AVM DWG's gross areas lost 17–42 % of their
/// hatch, docs/adr/9228).
fn bridge(pts: &[Vec2], ring: Vec<usize>, hole: &[usize], mi: usize) -> Vec<usize> {
    let m = pts[hole[mi]];
    let mut best_x = f64::INFINITY;
    let mut edge = None;
    let n = ring.len();
    for i in 0..n {
        let a = pts[ring[i]];
        let b = pts[ring[(i + 1) % n]];
        if a.y >= b.y || m.y < a.y || m.y > b.y {
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
    let corner = |i: usize| {
        (
            pts[ring[(i + n - 1) % n]],
            pts[ring[i]],
            pts[ring[(i + 1) % n]],
        )
    };
    let holds_m = |i: usize| {
        let (prev, v, next) = corner(i);
        locally_inside(prev, v, next, m)
    };
    if same(p, hit) {
        // The ray meets the ring at a vertex: joined there, at the pass whose
        // corner holds M.
        if !holds_m(pi)
            && let Some(i) = (0..n).find(|&i| same(pts[ring[i]], p) && holds_m(i))
        {
            pi = i;
        }
    } else {
        let mut best: Option<(f64, f64, usize)> = None;
        // M, I, P as a counter-clockwise triangle for the inside test.
        let (t0, t1, t2) = if cross(m, hit, p) >= 0.0 {
            (m, hit, p)
        } else {
            (m, p, hit)
        };
        for i in 0..n {
            let v = pts[ring[i]];
            if v.x == m.x || !in_triangle(v, t0, t1, t2) || !holds_m(i) {
                continue;
            }
            let ang = atan2(v.y - m.y, v.x - m.x).abs();
            let d = hypot(v.x - m.x, v.y - m.y);
            let better = match best {
                None => true,
                Some((best_angle, best_dist, b)) => {
                    ang < best_angle - 1e-12
                        || ((ang - best_angle).abs() <= 1e-12
                            && (d < best_dist
                                || (same(v, pts[ring[b]]) && {
                                    let (bp, bv, bn) = corner(b);
                                    let (vp, _, vn) = corner(i);
                                    holds_corner(bp, bv, bn, vp, vn)
                                })))
                }
            };
            if better {
                best = Some((ang, d, i));
            }
        }
        if let Some((_, _, i)) = best {
            pi = i;
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
        idx.as_chunks::<3>()
            .0
            .iter()
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
        for c in xy.as_chunks::<2>().0.iter() {
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

    /// The rings one after another and their ranges.
    fn rings_of(rings: &[Vec<Vec2>]) -> (Vec<Vec2>, Vec<Range<usize>>) {
        let mut pts = Vec::new();
        let mut ranges = Vec::new();
        for r in rings {
            ranges.push(pts.len()..pts.len() + r.len());
            pts.extend_from_slice(r);
        }
        (pts, ranges)
    }

    fn ring(xy: &[(f64, f64)]) -> Vec<Vec2> {
        xy.iter().map(|&(x, y)| v(x, y)).collect()
    }

    /// The area `fill_into`'s corners cover; every triangle must be counter-clockwise.
    fn filled(rings: &[Vec<Vec2>]) -> (f64, Vec<Vec2>) {
        let (pts, ranges) = rings_of(rings);
        let mut corners = Vec::new();
        fill_into(&pts, &ranges, &mut corners);
        assert_eq!(corners.len() % 3, 0);
        let area = corners
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let a = cross(t[0], t[1], t[2]) / 2.0;
                assert!(a >= 0.0, "clockwise triangle {t:?}");
                a
            })
            .sum();
        (area, corners)
    }

    /// The SİVAS AVM DWG's gross area hatch cut down to what still went wrong (docs/adr/9228), moved by an exact
    /// (130, −1840) m: the third hole joined across another's bridge. The ear clipper filled 30.8 % of it, the whole
    /// hatch 57.9 %. Its numbers to the last digit: rounded to 0.1 mm, the clipper fills it.
    #[test]
    fn holes_joined_across_a_bridge_are_filled_round() {
        let rings = [
            ring(&[
                (6.092542324300467, 1.6541128264150302),
                (19.792465653785, 1.6891094123789117),
                (6.883512674236982, 4.124689124204906),
            ]),
            ring(&[
                (15.02291112323772, 2.15411282641503),
                (15.182911123237744, 2.15411282641503),
                (15.182911123237659, 1.684112826415003),
                (15.02291112323769, 1.684112826415003),
            ]),
            ring(&[
                (10.472911123237651, 1.8941128264223153),
                (10.472911123237651, 1.9341128264222789),
                (10.892911123237724, 1.9341128264222789),
                (10.892911123237724, 1.8941128264223153),
            ]),
            ring(&[
                (9.974945448948716, 2.134112826317505),
                (10.20291112323784, 2.1341128264150484),
                (10.20291112323784, 1.7341128264151848),
                (10.142911123237894, 1.7341128264151848),
                (10.142911123237894, 1.684112837020848),
                (9.974945448948716, 1.684112837031762),
            ]),
        ];
        let want = signed_area(&rings[0]).abs()
            - rings[1..].iter().map(|h| signed_area(h).abs()).sum::<f64>();
        let (pts, ranges) = rings_of(&rings);
        let mut idx = Vec::new();
        triangulate_into(&pts, &ranges, &mut idx);
        let got = covered(&pts, &idx);
        assert!((got - want).abs() < 1e-9 * want, "{got} of {want}");
    }

    /// Holes meeting at a corner are filled round, the polygon itself too.
    #[test]
    fn holes_touching_at_a_corner_are_filled_round() {
        let rings = [
            square(0.0, 0.0, 10.0),
            square(2.0, 2.0, 3.0),
            square(5.0, 5.0, 3.0),
            ring(&[(5.0, 2.0), (7.0, 2.0), (7.0, 4.0), (5.0, 4.0)]),
        ];
        let (pts, ranges) = rings_of(&rings);
        let mut idx = Vec::new();
        triangulate_into(&pts, &ranges, &mut idx);
        assert!(
            (covered(&pts, &idx) - 78.0).abs() < 1e-9,
            "{}",
            covered(&pts, &idx)
        );
        assert!((filled(&rings).0 - 78.0).abs() < 1e-9);
    }

    /// A ring crossing itself is filled where it winds round, both loops of a bow tie, with a corner where it crosses.
    #[test]
    fn a_ring_crossing_itself_is_filled_where_it_winds() {
        let bow = ring(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
        let (area, corners) = filled(std::slice::from_ref(&bow));
        assert!((area - 50.0).abs() < 1e-9, "{area}");
        assert!(
            corners
                .iter()
                .any(|p| (p.x - 5.0).abs() < 1e-9 && (p.y - 5.0).abs() < 1e-9)
        );
        // A bow-tie hole takes both of its loops out.
        let (area, _) = filled(&[square(-5.0, -5.0, 20.0), bow]);
        assert!((area - 350.0).abs() < 1e-9, "{area}");
    }

    /// A hole over the outline takes out what it covers of the area, no more.
    #[test]
    fn a_hole_over_the_outline_takes_out_only_what_it_covers() {
        let rings = [
            square(0.0, 0.0, 10.0),
            ring(&[(8.0, 4.0), (12.0, 4.0), (12.0, 6.0), (8.0, 6.0)]),
        ];
        assert!((filled(&rings).0 - 96.0).abs() < 1e-9);
    }

    /// A polygon the ear clipper fills is filled with its triangles, corner for corner.
    #[test]
    fn a_valid_polygon_is_filled_with_the_clippers_triangles() {
        let rings = [
            square(0.0, 0.0, 10.0),
            square(4.0, 4.0, 2.0),
            square(1.0, 6.0, 2.0),
        ];
        let (pts, ranges) = rings_of(&rings);
        let mut idx = Vec::new();
        triangulate_into(&pts, &ranges, &mut idx);
        let clipped: Vec<Vec2> = idx.iter().map(|&i| pts[i as usize]).collect();
        assert_eq!(filled(&rings).1, clipped);
    }
}
