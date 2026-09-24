//! Points, bounds, rings and bearings (`apps/web/src/model/geometry.ts`), ported
//! operation for operation (docs/adr/0008).

use crate::api::Op;
use crate::jsmath::{PI, atan2, js_hypot, js_max, js_min};
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
    js_hypot(b.x - a.x, b.y - a.y)
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
    b.min_x = js_min(b.min_x, p.x - pad);
    b.min_y = js_min(b.min_y, p.y - pad);
    b.max_x = js_max(b.max_x, p.x + pad);
    b.max_y = js_max(b.max_y, p.y + pad);
}

pub fn is_empty_bounds(b: &Bounds) -> bool {
    !(b.max_x >= b.min_x && b.max_y >= b.min_y)
}

/// Signed shoelace area, positive for counter-clockwise rings. Coordinates
/// are taken relative to the first vertex: products of raw TM coordinates
/// (4.4·10⁶ m) would cancel away the fourth decimal of a parcel area.
pub fn signed_area(pts: &[Vec2]) -> f64 {
    if pts.len() < 3 {
        return 0.0;
    }
    let ox = pts[0].x;
    let oy = pts[0].y;
    let mut a = 0.0;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        a += (pts[j].x - ox) * (pts[i].y - oy) - (pts[i].x - ox) * (pts[j].y - oy);
        j = i;
    }
    a / 2.0
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

pub fn centroid(pts: &[Vec2]) -> Vec2 {
    let a = signed_area(pts);
    if a.abs() < 1e-9 {
        let (mut sx, mut sy) = (0.0, 0.0);
        for p in pts {
            sx += p.x;
            sy += p.y;
        }
        let n = pts.len() as f64;
        return Vec2::new(sx / n, sy / n);
    }
    let (mut cx, mut cy) = (0.0, 0.0);
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let f = pts[j].x * pts[i].y - pts[i].x * pts[j].y;
        cx += (pts[j].x + pts[i].x) * f;
        cy += (pts[j].y + pts[i].y) * f;
        j = i;
    }
    Vec2::new(cx / (6.0 * a), cy / (6.0 * a))
}

/// Even-odd point in ring test. For each edge crossing the horizontal
/// through p, whether the crossing lies to p's right is decided exactly (p
/// left of an upward edge, right of a downward one: `orientation`, CLAUDE.md
/// §23.3), not by comparing p with a rounded crossing. A point exactly on an
/// edge may still fall either way, but always the same way.
pub fn point_in_polygon(p: Vec2, pts: &[Vec2]) -> bool {
    let mut inside = false;
    if pts.is_empty() {
        return false;
    }
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[j];
        if (a.y > p.y) != (b.y > p.y) {
            let side = orientation(a, b, p);
            if if b.y > a.y { side > 0 } else { side < 0 } {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

pub fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        js_max(
            0.0,
            js_min(1.0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2),
        )
    };
    js_hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy))
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
