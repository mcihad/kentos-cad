//! Points, bounds, rings and bearings (`apps/web/src/model/geometry.ts`), ported
//! operation for operation (docs/adr/0008).

use crate::api::Op;
use crate::jsmath::{PI, atan2, hypot, maximum, minimum};
use crate::op;
use crate::predicates::orientation;
use crate::vec2::Vec2;

/// Axis-aligned box; empty when min > max (`emptyBounds`: ±∞).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

crate::json_struct!(Bounds { min_x => "minX", min_y => "minY", max_x => "maxX", max_y => "maxY" });

pub fn dist(a: Vec2, b: Vec2) -> f64 {
    hypot(b.x - a.x, b.y - a.y)
}

pub fn empty_bounds() -> Bounds {
    Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    }
}

pub fn extend_bounds(b: &mut Bounds, p: Vec2, pad: f64) {
    b.min_x = minimum(b.min_x, p.x - pad);
    b.min_y = minimum(b.min_y, p.y - pad);
    b.max_x = maximum(b.max_x, p.x + pad);
    b.max_y = maximum(b.max_y, p.y + pad);
}

pub fn is_empty_bounds(b: &Bounds) -> bool {
    !(b.max_x >= b.min_x && b.max_y >= b.min_y)
}

/// Signed shoelace area, positive for counter-clockwise rings. Coordinates
/// are taken relative to the first vertex: products of raw TM coordinates
/// (4.4·10⁶ m) would cancel away the fourth decimal of a parcel area.
pub fn signed_area(pts: &[Vec2]) -> f64 {
    ring_area(pts)
}

/// [`signed_area`] for any point type that gives its `[x, y]` (this core's `Vec2`, the contracts' `Vec2`, a pair of
/// numbers): the sum is written once, so no crate keeps a copy that multiplies raw coordinates (a 10 m square at
/// Transverse Mercator coordinates came 99.99976 m² from such a copy; docs/adr/9401 §9).
pub fn ring_area<P: Copy + Into<[f64; 2]>>(ring: &[P]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let [ox, oy] = ring[0].into();
    let [mut jx, mut jy] = ring[ring.len() - 1].into();
    let mut a = 0.0;
    for &q in ring {
        let [ix, iy] = q.into();
        a += (jx - ox) * (iy - oy) - (ix - ox) * (jy - oy);
        (jx, jy) = (ix, iy);
    }
    a / 2.0
}

/// The area centroid of a ring, for any point type that gives its `[x, y]`; none for a ring of fewer than three
/// corners or without area. Coordinates are taken relative to the first vertex, as in [`ring_area`]: summed from raw
/// Transverse Mercator coordinates the products cancel away metres (a 10 m square's centroid strayed 3.6 m, a small
/// column's more than a kilometre; docs/adr/0122, docs/adr/9401 §9.4). The one sum of its kind: no crate keeps a copy.
pub fn ring_centroid<P: Copy + Into<[f64; 2]>>(ring: &[P]) -> Option<[f64; 2]> {
    if ring.len() < 3 {
        return None;
    }
    let [ox, oy] = ring[0].into();
    let [lx, ly] = ring[ring.len() - 1].into();
    let (mut jx, mut jy) = (lx - ox, ly - oy);
    let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for &q in ring {
        let [x, y] = q.into();
        let (ix, iy) = (x - ox, y - oy);
        let f = jx * iy - ix * jy;
        a2 += f;
        cx += (jx + ix) * f;
        cy += (jy + iy) * f;
        (jx, jy) = (ix, iy);
    }
    (a2 != 0.0 && a2.is_finite()).then(|| [ox + cx / (3.0 * a2), oy + cy / (3.0 * a2)])
}

/// Length along the vertices; a closed path adds the closing segment.
pub fn path_length(pts: &[Vec2], closed: bool) -> f64 {
    let mut l = 0.0;
    for i in 1..pts.len() {
        l += dist(pts[i - 1], pts[i]);
    }
    if closed && pts.len() > 2 {
        l += dist(pts[pts.len() - 1], pts[0]);
    }
    l
}

/// The area's centroid; the vertices' mean for a ring without area.
/// Coordinates are taken relative to the first vertex, as in `signed_area`:
/// with raw TM coordinates the products cancel away metres (a 25 × 18 m
/// parcel's centroid strayed 5 m; docs/adr/0122).
pub fn centroid(pts: &[Vec2]) -> Vec2 {
    let Some(&o) = pts.first() else {
        return Vec2::new(f64::NAN, f64::NAN);
    };
    let a = signed_area(pts);
    if a.abs() < 1e-9 {
        let (mut sx, mut sy) = (0.0, 0.0);
        for p in pts {
            sx += p.x - o.x;
            sy += p.y - o.y;
        }
        let n = pts.len() as f64;
        return Vec2::new(o.x + sx / n, o.y + sy / n);
    }
    let (mut cx, mut cy) = (0.0, 0.0);
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xj, yj) = (pts[j].x - o.x, pts[j].y - o.y);
        let (xi, yi) = (pts[i].x - o.x, pts[i].y - o.y);
        let f = xj * yi - xi * yj;
        cx += (xj + xi) * f;
        cy += (yj + yi) * f;
        j = i;
    }
    Vec2::new(o.x + cx / (6.0 * a), o.y + cy / (6.0 * a))
}

/// Even-odd point in ring test. For each edge crossing the horizontal
/// through p, whether the crossing lies to p's right is decided exactly (p
/// left of an upward edge, right of a downward one: `orientation`, CLAUDE.md
/// §23.3), not by comparing p with a rounded crossing. A point exactly on an
/// edge may still fall either way, but always the same way.
pub fn point_in_polygon(p: Vec2, pts: &[Vec2]) -> bool {
    point_in_ring(p, pts)
}

/// [`point_in_polygon`] for any point type that gives its `[x, y]` (this core's `Vec2`, the contracts' `Vec2`): the
/// test is written once, so no crate keeps a copy of it for its own points (docs/adr/9401 §9).
pub fn point_in_ring<P: Copy + Into<[f64; 2]>>(p: P, ring: &[P]) -> bool {
    let at = |q: P| {
        let [x, y] = q.into();
        Vec2::new(x, y)
    };
    let Some(&last) = ring.last() else {
        return false;
    };
    let p = at(p);
    let mut inside = false;
    let mut b = at(last);
    for &q in ring {
        let a = at(q);
        if ray_crosses(p, a, b) {
            inside = !inside;
        }
        b = a;
    }
    inside
}

/// Whether the ray from `p` to its right crosses the edge between `a` and `b` (in either order): one end is above
/// `p` and the other is not, and the crossing lies to `p`'s right, decided exactly (`orientation`). The one edge
/// decision of the even-odd tests; a walk that does more with each edge (the label engine's distance) asks it here.
pub fn ray_crosses(p: Vec2, a: Vec2, b: Vec2) -> bool {
    if (a.y > p.y) == (b.y > p.y) {
        return false;
    }
    let side = orientation(a, b, p);
    if b.y > a.y { side > 0 } else { side < 0 }
}

/// Even-odd point in rings test: whether `p` is inside an odd number of `rings` (an area's outer ring with its holes; a
/// multipart area's rings). Each ring is decided as [`point_in_polygon`] decides it, on exact values: a point on the
/// edge two areas share is in exactly one of them. A crossing computed in floating point and then compared put such a
/// point in both areas or in neither (fixtures/geometry/v1/point_in_rings.json, docs/adr/9401 §9).
pub fn point_in_rings<P: Copy + Into<[f64; 2]>, R: AsRef<[P]>>(p: P, rings: &[R]) -> bool {
    rings
        .iter()
        .fold(false, |odd, ring| odd != point_in_ring(p, ring.as_ref()))
}

pub fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        maximum(
            0.0,
            minimum(1.0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2),
        )
    };
    hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy))
}

/// Angle in degrees, counter-clockwise from east (CAD convention).
pub fn angle_deg(a: Vec2, b: Vec2) -> f64 {
    (atan2(b.y - a.y, b.x - a.x) * 180.0) / PI
}

/// Surveying bearing (semt) in grads, clockwise from grid north.
pub fn bearing_grad(a: Vec2, b: Vec2) -> f64 {
    let g = (atan2(b.x - a.x, b.y - a.y) * 200.0) / PI;
    (g + 400.0) % 400.0
}

pub(crate) static OPS: &[Op] = &[
    op!("dist", |a: Vec2, b: Vec2| dist(a, b)),
    op!("signedArea", |pts: Vec<Vec2>| signed_area(&pts)),
    op!("pathLength", |pts: Vec<Vec2>, closed: Option<bool>| {
        path_length(&pts, closed.unwrap_or(false))
    }),
    op!("centroid", |pts: Vec<Vec2>| centroid(&pts)),
    op!("pointInPolygon", |p: Vec2, pts: Vec<Vec2>| {
        point_in_polygon(p, &pts)
    }),
    op!("distToSegment", |p: Vec2, a: Vec2, b: Vec2| {
        dist_to_segment(p, a, b)
    }),
    op!("angleDeg", |a: Vec2, b: Vec2| angle_deg(a, b)),
    op!("bearingGrad", |a: Vec2, b: Vec2| bearing_grad(a, b)),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The test before §23.3: p compared with the rounded crossing x.
    fn rounded_crossing(p: Vec2, pts: &[Vec2]) -> bool {
        let mut inside = false;
        let mut j = pts.len() - 1;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[j]);
            if (a.y > p.y) != (b.y > p.y) && p.x < ((b.x - a.x) * (p.y - a.y)) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// Whether p lies left of a→b, exactly: TM coordinates lie on the 2⁻³⁴
    /// grid, so scaled they are integers and i128 holds the cross product.
    fn exactly_left(a: Vec2, b: Vec2, p: Vec2) -> bool {
        let k = |x: f64| (x * (1u64 << 34) as f64) as i128;
        (k(b.x) - k(a.x)) * (k(p.y) - k(a.y)) - (k(b.y) - k(a.y)) * (k(p.x) - k(a.x)) > 0
    }

    fn step(x: f64, up: bool) -> f64 {
        let bits = x.to_bits();
        f64::from_bits(if (x > 0.0) == up { bits + 1 } else { bits - 1 })
    }

    /// A 76 m² parcel at TM coordinates: its centroid is the exact one
    /// (Python `fractions`) to 10 nm. Taken on the raw coordinates it was
    /// 1.9 m off (docs/adr/0122).
    #[test]
    fn a_small_parcels_centroid_at_tm_coordinates_is_exact() {
        let pts = [
            (486_125.439_14, 4_420_808.742_16),
            (486_123.215_69, 4_420_805.328_36),
            (486_117.317_63, 4_420_805.780_16),
            (486_113.390_28, 4_420_812.095_89),
            (486_118.100_35, 4_420_815.166_15),
            (486_119.876_27, 4_420_813.2),
            (486_124.9, 4_420_812.6),
        ]
        .map(|(x, y)| Vec2::new(x, y));
        let c = centroid(&pts);
        assert!((c.x - 486_119.795_251_411_9).abs() < 1e-8, "{c:?}");
        assert!((c.y - 4_420_809.901_225_769).abs() < 1e-8, "{c:?}");
    }

    /// Points within an ulp or two of a long parcel edge at TM coordinates:
    /// inside exactly when they lie left of the counter-clockwise edge. The
    /// rounded crossing misjudges some of them; the exact test none.
    #[test]
    fn points_next_to_an_edge_are_decided_exactly() {
        let a = Vec2::new(486_512.34, 4_420_187.52);
        let b = Vec2::new(486_931.87, 4_420_446.09);
        let ring = [a, b, Vec2::new(486_600.11, 4_420_800.73)];
        let mut rounded_wrong = 0;
        for n in 0..20_000 {
            let t = (f64::from(n) + 0.5) / 20_000.0 * 0.8 + 0.1;
            let mut p = Vec2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
            p = match n % 5 {
                0 => p,
                1 => Vec2::new(step(p.x, true), p.y),
                2 => Vec2::new(step(p.x, false), p.y),
                3 => Vec2::new(p.x, step(p.y, true)),
                _ => Vec2::new(p.x, step(p.y, false)),
            };
            let want = exactly_left(a, b, p);
            assert_eq!(point_in_polygon(p, &ring), want, "{p:?}");
            if rounded_crossing(p, &ring) != want {
                rounded_wrong += 1;
            }
        }
        assert!(rounded_wrong > 0, "the edge no longer exercises rounding");
    }

    #[test]
    fn plain_rings() {
        let square = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        assert!(point_in_polygon(Vec2::new(0.5, 0.5), &square));
        assert!(!point_in_polygon(Vec2::new(1.5, 0.5), &square));
        assert!(!point_in_polygon(Vec2::new(0.5, -0.5), &square));
        assert!(!point_in_polygon(Vec2::new(0.5, 0.5), &[]));
        // Clockwise rings count the same.
        let cw: Vec<Vec2> = square.iter().rev().copied().collect();
        assert!(point_in_polygon(Vec2::new(0.25, 0.75), &cw));
    }
}
