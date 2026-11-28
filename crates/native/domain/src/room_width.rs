//! A room's net width (docs/adr/8033; docs/adr/8057 §53): the diameter of the largest circle inside its net outline,
//! the regulations' "dar kenar" (Planlı Alanlar İmar Yönetmeliği 29/1 and 29/3). One measure for Mimari denetim's
//! rules, the program check and the audit facts an extension's rule reads.
//!
//! - **Its net outline:** its inner faces found again round its seed ([`net_parts`]), else the rings drawn.
//! - **To within [`PRECISION`]:** the pole of inaccessibility; its centre is the outline's centroid when that is as
//!   good (a rectangle's middle, not a point along its long axis), where a finding on the room points.
//! - **A passage's least width** ([`least_of`], docs/adr/8110): its narrowest neck, the width a corridor is by (its
//!   largest circle is its widest place: an L, a T or a lobby makes it wider than it is).

use kentos_contracts::{SpaceFields, SpaceMethod, SpacePart, Vec2};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge};
use kentos_geometry_core::geometry::point_in_rings;

use crate::space_follow::SpaceSearch;

/// How near the measure comes to the true width, metres.
pub const PRECISION: f64 = 1e-4;

fn core(p: &Vec2) -> V {
    V::new(p.x, p.y)
}

/// A part's rings, its arcs as chords: the outline first, then its holes.
fn rings_of(p: &SpacePart) -> Vec<Vec<V>> {
    use kentos_geometry_core::tessellate::bulge_path;
    std::iter::once(&p.outer)
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
        .collect()
}

/// A passage's least width (docs/adr/8110): the shortest chord across its net outline that leaves, on each side, room
/// for a circle as wide as the chord is long (else its largest circle); the least of its parts'. None without a ring.
pub fn least_of(parts: &[SpacePart]) -> Option<kentos_geometry_core::geom::neck::LeastWidth> {
    parts
        .iter()
        .filter_map(|p| kentos_geometry_core::geom::neck::least_width(&rings_of(p), PRECISION))
        .reduce(|a, b| if b.width < a.width { b } else { a })
}

/// The net width of `parts`: the largest circle inside, its diameter, and its centre; 0 and none without a ring.
pub fn width_of(parts: &[SpacePart]) -> (f64, Option<V>) {
    parts
        .iter()
        .filter_map(|p| {
            let rings = rings_of(p);
            kentos_geometry_core::labels::geom::pole(&rings, PRECISION).map(|(p, d)| {
                let c = centroid(&rings[0]);
                let inside = c.is_some_and(|c| point_in_rings(c, &rings));
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
    let twice = 2.0 * kentos_geometry_core::geometry::ring_area(ring);
    (twice.abs() > 1e-12)
        .then(|| kentos_geometry_core::geometry::ring_centroid(ring))
        .flatten()
        .map(|[x, y]| V::new(x, y))
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

    /// A point on the edge two rings share is in exactly one of them (docs/adr/9401 §9; a case of
    /// fixtures/geometry/v1/point_in_rings.json): the middle of the wall line between two rooms.
    #[test]
    fn a_point_on_a_shared_edge_is_in_one_ring() {
        use kentos_geometry_core::Vec2 as V;
        let (a, b) = (V::new(3.35, 34.25), V::new(-4.45, 42.08));
        let p = V::new(-0.55, 38.165);
        let left = vec![a, b, V::new(-34.45, 42.08), V::new(-34.45, 34.25)];
        let right = vec![b, a, V::new(33.35, 34.25), V::new(33.35, 42.08)];
        assert_ne!(
            super::point_in_rings(p, &[left]),
            super::point_in_rings(p, &[right])
        );
    }

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

#[cfg(test)]
mod centroid_tests {
    /// A ring's centroid at Transverse Mercator coordinates is where the ring is (docs/adr/9401 §9.4): summed from raw
    /// coordinates it strayed metres (a 10 m square's by 3.6 m, a small column's by more than a kilometre).
    #[test]
    fn a_centroid_at_large_coordinates_is_in_its_ring() {
        use kentos_geometry_core::Vec2 as V;
        let (e, n) = (486_512.34, 4_420_187.52);
        let ring = [
            V::new(e, n),
            V::new(e + 10.0, n),
            V::new(e + 10.0, n + 10.0),
            V::new(e, n + 10.0),
        ];
        let c = super::centroid(&ring).expect("a ring with area");
        assert!(
            (c.x - (e + 5.0)).abs() < 1e-6 && (c.y - (n + 5.0)).abs() < 1e-6,
            "{c:?}"
        );
    }
}
