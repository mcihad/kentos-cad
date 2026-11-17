//! A room's net width (docs/adr/8033; docs/adr/8057 §53): the diameter of the largest circle inside its net outline,
//! the regulations' "dar kenar" (Planlı Alanlar İmar Yönetmeliği 29/1 and 29/3). One measure for Mimari denetim's
//! rules, the program check and the audit facts an extension's rule reads.
//!
//! - **Its net outline:** its inner faces found again round its seed ([`net_parts`]), else the rings drawn.
//! - **To within [`PRECISION`]:** the pole of inaccessibility; its centre is the outline's centroid when that is as
//!   good (a rectangle's middle, not a point along its long axis), where a finding on the room points.

use kentos_contracts::{SpaceFields, SpaceMethod, SpacePart, Vec2};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge};

use crate::space_follow::SpaceSearch;

/// How near the measure comes to the true width, metres.
pub const PRECISION: f64 = 1e-4;

fn core(p: &Vec2) -> V {
    V::new(p.x, p.y)
}

/// The net width of `parts`: the largest circle inside, its diameter, and its centre; 0 and none without a ring.
pub fn width_of(parts: &[SpacePart]) -> (f64, Option<V>) {
    use kentos_geometry_core::tessellate::bulge_path;
    parts
        .iter()
        .filter_map(|p| {
            let rings: Vec<Vec<V>> = std::iter::once(&p.outer)
                .chain(&p.holes)
                .map(|r| {
                    let pts: Vec<V> = r.pts.iter().map(core).collect();
                    bulge_path(
                        &pts,
                        r.bulges.as_deref(),
                        true,
                        kentos_geometry_core::geom::curve_outline::CURVE_TOL,
                    )
                })
                .collect();
            kentos_geometry_core::labels::geom::pole(&rings, PRECISION).map(|(p, d)| {
                let c = centroid(&rings[0]);
                let inside = c.is_some_and(|c| rings_hold(&rings, c));
                match c.filter(|_| inside).map(|c| (c, ring_distance(&rings, c))) {
                    Some((c, dc)) if dc >= d - PRECISION => (2.0 * d, c),
                    _ => (2.0 * d, p),
                }
            })
        })
        .fold(
            (0.0, None),
            |best, (w, p)| {
                if w > best.0 { (w, Some(p)) } else { best }
            },
        )
}

/// A room's net outline: its inner faces found again from its seed, else its own parts.
pub fn net_parts(search: &mut SpaceSearch<'_>, s: &SpaceFields) -> Vec<SpacePart> {
    if s.method != SpaceMethod::Net && s.seed.is_some() {
        let mut net = s.clone();
        net.method = SpaceMethod::Net;
        let found = search.found_preserving_legacy(&net);
        if found.broken.is_none() && !found.parts.is_empty() {
            return found.parts;
        }
    }
    s.parts.clone()
}

fn centroid(ring: &[V]) -> Option<V> {
    let n = ring.len();
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let k = p.x * q.y - q.x * p.y;
        a += k;
        cx += (p.x + q.x) * k;
        cy += (p.y + q.y) * k;
    }
    (a.abs() > 1e-12).then(|| V::new(cx / (3.0 * a), cy / (3.0 * a)))
}

/// Whether a point lies inside the first ring and outside the others (even–odd).
fn rings_hold(rings: &[Vec<V>], p: V) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
        }
    }
    inside
}

fn ring_distance(rings: &[Vec<V>], p: V) -> f64 {
    rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .map(|(a, b)| closest_on_edge(&Edge::Seg { a, b }, p).d)
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{SpacePart, SpaceRing, Vec2};

    use super::*;

    fn rect(w: f64, h: f64) -> SpacePart {
        let pts = vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: w, y: 0.0 },
            Vec2 { x: w, y: h },
            Vec2 { x: 0.0, y: h },
        ];
        SpacePart {
            outer: SpaceRing { pts, bulges: None },
            holes: Vec::new(),
        }
    }

    /// A rectangle's net width is its narrow side, its centre its middle; no ring, no width.
    #[test]
    fn a_rooms_width_is_its_narrow_side() {
        let (w, c) = width_of(&[rect(4.0, 3.0)]);
        assert!((w - 3.0).abs() < 2.0 * PRECISION, "{w}");
        let c = c.expect("its centre");
        assert!((c.x - 2.0).abs() < 1e-6 && (c.y - 1.5).abs() < 1e-6);
        assert_eq!(width_of(&[]), (0.0, None));
    }
}
