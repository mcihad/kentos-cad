//! Malzeme bölgeleri (plan/map-cad.md MAP-15, docs/adr/3540; Civil 3D's bounded volumes in its Volumes Dashboard): the
//! difference of two surfaces inside closed boundaries, each the place of one material with its own cut and fill
//! factors.
//!
//! Each piece of the two surfaces' overlay ([`Ground::overlay`]: convex, `d = z_comparison − z_base` linear over it) is
//! fanned into triangles from its first corner; each triangle is the window that clips a zone's rings
//! (Sutherland–Hodgman: a concave ring gives a polygon whose doubled edges along the window add no area). The part is
//! split where `d` is zero and the linear `d` is integrated exactly from the part's area and first moments, the rule of
//! [`crate::stockpile`]. The outer ring turns counterclockwise and the holes clockwise, so a hole's parts subtract.
//! Coordinates are taken from the zone's first corner: large map coordinates keep their digits.
//!
//! Two zones overlap by the area of their intersection: one zone's rings clipped by the triangles of the other's rings'
//! fans, each signed by its turn (a simple ring's inside is the signed sum of its fan's triangles; a hole's clockwise
//! fan takes its inside away).

use crate::stockpile::{keep, moments, oriented, signed_area};
use crate::surface::Region;
use crate::volume::{Ground, Triangle};

type P = [f64; 2];

/// Two zones sharing less than this (m², a square millimetre) do not overlap: the rounding of a common edge.
pub const OVERLAP: f64 = 1e-6;

/// A zone's part of the difference: its plan area, the part of it both surfaces cover, the cut (comparison below the
/// base) and fill volumes and the plan areas where they are.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Measured {
    pub area: f64,
    pub covered: f64,
    pub cut: f64,
    pub fill: f64,
    pub cut_area: f64,
    pub fill_area: f64,
}

/// The region's rings taken from `o`: the outer counterclockwise, the holes clockwise.
fn rings(region: &Region, o: P) -> Vec<Vec<P>> {
    let shift = |p: &P| [p[0] - o[0], p[1] - o[1]];
    let mut out = vec![oriented(region.outer.iter().map(shift).collect(), true)];
    out.extend(region.holes.iter().map(|h| oriented(h.iter().map(shift).collect(), false)));
    out
}

/// `poly` inside the counterclockwise triangle `t` (Sutherland–Hodgman, one edge at a time).
fn inside(poly: &[P], t: [P; 3]) -> Vec<P> {
    let mut out = poly.to_vec();
    for k in 0..3 {
        let (a, b) = (t[k], t[(k + 1) % 3]);
        let left = move |p: P| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        out = keep(&out, &left);
        if out.len() < 3 {
            return Vec::new();
        }
    }
    out
}

fn turn(a: P, b: P, c: P) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Each zone's part of the difference of `comparison` over `base`, in the zones' order.
pub fn measure(comparison: &[Triangle], base: &[Triangle], zones: &[Region]) -> Vec<Measured> {
    let origins: Vec<P> = zones.iter().map(|z| z.outer.first().copied().unwrap_or([0.0, 0.0])).collect();
    let shifted: Vec<Vec<Vec<P>>> = zones.iter().zip(&origins).map(|(z, o)| rings(z, *o)).collect();
    // Each zone's box, in the drawing's coordinates, so a piece far from it is passed.
    let boxes: Vec<(P, P)> = zones
        .iter()
        .map(|z| {
            z.outer.iter().fold(([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]), |(lo, hi), p| {
                ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
            })
        })
        .collect();
    let mut out: Vec<Measured> = shifted
        .iter()
        .map(|r| Measured { area: r.iter().map(|ring| signed_area(ring)).sum(), ..Measured::default() })
        .collect();
    if zones.is_empty() {
        return out;
    }
    let mut ground = Ground::new(base);
    ground.overlay(comparison, |piece| {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for (p, _) in piece {
            lo = [lo[0].min(p[0]), lo[1].min(p[1])];
            hi = [hi[0].max(p[0]), hi[1].max(p[1])];
        }
        for (k, zone) in shifted.iter().enumerate() {
            let (zlo, zhi) = boxes[k];
            if hi[0] < zlo[0] || lo[0] > zhi[0] || hi[1] < zlo[1] || lo[1] > zhi[1] {
                continue;
            }
            let o = origins[k];
            let at = |i: usize| ([piece[i].0[0] - o[0], piece[i].0[1] - o[1]], piece[i].1);
            let m = &mut out[k];
            for i in 1..piece.len().saturating_sub(1) {
                let (a, b, c) = (at(0), at(i), at(i + 1));
                let det = turn(a.0, b.0, c.0);
                if det <= 0.0 {
                    continue;
                }
                // The difference's plane over the triangle: d = gx·x + gy·y + g0.
                let (e1, e2) = ([b.0[0] - a.0[0], b.0[1] - a.0[1]], [c.0[0] - a.0[0], c.0[1] - a.0[1]]);
                let (d1, d2) = (b.1 - a.1, c.1 - a.1);
                let gx = (d1 * e2[1] - d2 * e1[1]) / det;
                let gy = (e1[0] * d2 - e2[0] * d1) / det;
                let g0 = a.1 - gx * a.0[0] - gy * a.0[1];
                let fill = move |p: P| gx * p[0] + gy * p[1] + g0;
                let cut = move |p: P| -(gx * p[0] + gy * p[1] + g0);
                for ring in zone {
                    let poly = inside(ring, [a.0, b.0, c.0]);
                    if poly.is_empty() {
                        continue;
                    }
                    m.covered += moments(&poly).0;
                    for (filled, side) in [(true, &fill as &dyn Fn(P) -> f64), (false, &cut)] {
                        let part = keep(&poly, side);
                        if part.len() < 3 {
                            continue;
                        }
                        let (area, sx, sy) = moments(&part);
                        let integral = gx * sx + gy * sy + g0 * area;
                        if filled {
                            m.fill += integral;
                            m.fill_area += area;
                        } else {
                            m.cut -= integral;
                            m.cut_area += area;
                        }
                    }
                }
            }
        }
    });
    out
}

/// The plan area `a` and `b` share (m²).
pub fn overlap(a: &Region, b: &Region) -> f64 {
    let Some(&o) = a.outer.first() else {
        return 0.0;
    };
    let (ra, rb) = (rings(a, o), rings(b, o));
    let mut total = 0.0;
    for ring in &rb {
        for i in 1..ring.len().saturating_sub(1) {
            let (p, q, r) = (ring[0], ring[i], ring[i + 1]);
            let s = turn(p, q, r);
            if s == 0.0 {
                continue;
            }
            let (t, sign) = if s > 0.0 { ([p, q, r], 1.0) } else { ([p, r, q], -1.0) };
            for other in &ra {
                let poly = inside(other, t);
                if !poly.is_empty() {
                    total += sign * moments(&poly).0;
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plane surface over a square as two triangles.
    fn square(x0: f64, y0: f64, side: f64, z: impl Fn(f64, f64) -> f64) -> Vec<Triangle> {
        let c = |x: f64, y: f64| [x, y, z(x, y)];
        let (a, b, cc, d) = (c(x0, y0), c(x0 + side, y0), c(x0 + side, y0 + side), c(x0, y0 + side));
        vec![[a, b, cc], [a, cc, d]]
    }

    fn region(outer: &[P], holes: &[&[P]]) -> Region {
        Region { outer: outer.to_vec(), holes: holes.iter().map(|h| h.to_vec()).collect() }
    }

    #[test]
    fn a_zone_on_a_tilted_difference_takes_the_prism_inside_it() {
        // d = x − 5 over 0..10 square at map coordinates: fill east of x 5, cut west.
        let (x0, y0) = (4_500_000.0, 4_200_000.0);
        let base = square(x0, y0, 10.0, |_, _| 100.0);
        let comparison = square(x0, y0, 10.0, |x, _| 100.0 + (x - x0) - 5.0);
        // The zone x 3..9, y 2..6 (clockwise on purpose): cut over 3..5 (∫(5 − x) = 2·4 = 8), fill over 5..9 (8·4 = 32).
        let z = region(&[[x0 + 3.0, y0 + 2.0], [x0 + 3.0, y0 + 6.0], [x0 + 9.0, y0 + 6.0], [x0 + 9.0, y0 + 2.0]], &[]);
        let m = measure(&comparison, &base, &[z])[0];
        assert!((m.area - 24.0).abs() < 1e-9 && (m.covered - 24.0).abs() < 1e-9, "{m:?}");
        assert!((m.cut - 8.0).abs() < 1e-9 && (m.fill - 32.0).abs() < 1e-9, "{m:?}");
        assert!((m.cut_area - 8.0).abs() < 1e-9 && (m.fill_area - 16.0).abs() < 1e-9, "{m:?}");
    }

    #[test]
    fn a_hole_and_the_part_off_the_surfaces_are_left_out() {
        let base = square(0.0, 0.0, 10.0, |_, _| 0.0);
        let comparison = square(0.0, 0.0, 10.0, |_, _| 2.0);
        // 8 × 8 from (6, 6) runs 4 m past the surfaces both ways; a 1 × 1 hole inside.
        let z = region(&[[6.0, 6.0], [14.0, 6.0], [14.0, 14.0], [6.0, 14.0]], &[&[[7.0, 7.0], [8.0, 7.0], [8.0, 8.0], [7.0, 8.0]]]);
        let m = measure(&comparison, &base, &[z])[0];
        assert!((m.area - 63.0).abs() < 1e-9, "{m:?}");
        assert!((m.covered - 15.0).abs() < 1e-9 && (m.fill - 30.0).abs() < 1e-9 && m.cut.abs() < 1e-12, "{m:?}");
    }

    #[test]
    fn overlaps_count_the_shared_area_only() {
        let a = region(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], &[]);
        let touching = region(&[[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]], &[]);
        assert!(overlap(&a, &touching).abs() < OVERLAP);
        // A concave L over a's corner: 2 × 2 of it inside a.
        let l = region(&[[8.0, 8.0], [14.0, 8.0], [14.0, 9.0], [9.0, 9.0], [9.0, 14.0], [8.0, 14.0]], &[]);
        assert!((overlap(&a, &l) - 3.0).abs() < 1e-9, "{}", overlap(&a, &l));
        assert!((overlap(&l, &a) - 3.0).abs() < 1e-9);
        // A zone inside a's hole shares nothing.
        let holed = region(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], &[&[[2.0, 2.0], [8.0, 2.0], [8.0, 8.0], [2.0, 8.0]]]);
        let island = region(&[[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]], &[]);
        assert!(overlap(&holed, &island).abs() < OVERLAP && overlap(&island, &holed).abs() < OVERLAP);
    }
}
