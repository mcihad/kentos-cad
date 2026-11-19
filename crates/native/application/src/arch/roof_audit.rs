//! Çatının ölçüleri (docs/adr/8044 §9, 8159; M44): each roof measured on the drawing for the standards package's roof
//! rules (`catalog/standards/mimari-mevzuat`, `cati_olculeri`: Planlı Alanlar İmar Yönetmeliği 40/4, 40/5, 42/1, the
//! fire regulation's 58/2 and Sivas İmar Yönetmeliği 40/5, 42/5), which the audit facts carry (`building.roofs[].form`).
//!
//! - **Mahya:** the ridge's top over the saçak seviyesi (PAİY 4/ppp: "son kat tavan döşemesi üst kotu", the top of the
//!   storey the roof stands on), its kot farkı counted; a sloping roof's only.
//! - **Saçak:** a roof drawn from its walls overhangs them by its overhang; none for another.
//! - **Eğimler, kar siperliği:** each edge's slope (none: a gable or a flat edge) and whether a snow guard is given.
//! - **Parçalar:** each chimney's, vent's and window's own values (the rules' fixes write them back), a chimney's top
//!   over the ridge and a vent's over the roof at its highest corner (its own, else its kind's); a window's outline's
//!   nearest distance to the building's facade lines (the walls' outer faces the roof was drawn from, its eaves its
//!   overhang in), and between two windows the shortest distance between their outlines on the roof.
//!
//! The measures are the drawing's; the rules decide.

use kentos_commands::{Entity, RoofPart, RoofPartKind};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::roof as core;

/// A roof part as the rules read it: its drawn values, its top's rise (a chimney's or a vent's: its own, else its
/// kind's) and a window's distance to the facade lines.
#[derive(Clone, Debug, PartialEq)]
pub struct PartMeasure {
    pub part: RoofPart,
    pub top: Option<f64>,
    pub facade: Option<f64>,
}

/// A roof as the rules read it (docs/adr/8159).
#[derive(Clone, Debug, PartialEq)]
pub struct RoofMeasure {
    pub slot: Slot,
    /// Its middle in plan: where a finding about it points.
    pub at: V,
    /// One of its edges slopes.
    pub sloping: bool,
    /// The ridge's top over the saçak seviyesi, m: its rise and its kot farkı; none for a flat roof.
    pub ridge: Option<f64>,
    pub overhang: Option<f64>,
    /// Each edge's slope, per cent; none: a gable or a level edge.
    pub slopes: Vec<Option<f64>>,
    pub snow_guard: bool,
    /// Its parts in their order (the drawn order a fix writes back).
    pub parts: Vec<PartMeasure>,
    /// Two windows (their indices among the parts) and the shortest distance between their outlines on the roof, m.
    pub window_gaps: Vec<(usize, usize, f64)>,
}

/// Every roof's measures, in the drawing's order; a roof whose shape cannot be built is left out.
pub fn measures(doc: &Document) -> Vec<RoofMeasure> {
    let mut out = Vec::new();
    for e in doc.entities() {
        let Entity::Roof(r) = e else { continue };
        let Shape::Roof { pts, roof: spec } = crate::geometry::roof_shape(r, doc.settings()) else {
            continue;
        };
        if pts.len() < 3 {
            continue;
        }
        let at = V::new(
            pts.iter().map(|p| p.x).sum::<f64>() / pts.len() as f64,
            pts.iter().map(|p| p.y).sum::<f64>() / pts.len() as f64,
        );
        let sloping = spec.slopes.iter().any(Option::is_some);
        let ridge = sloping.then(|| core::ridge_rise(&pts, &spec).ok()).flatten().map(|rise| rise + r.roof.offset);
        // The windows' outlines in plan and on the roof, by their index among the parts (the spec's are the drawn
        // parts in their order).
        let overhang = r.roof.overhang.unwrap_or(0.0);
        let windows: Vec<(usize, Vec<[f64; 3]>, Vec<V>)> = spec
            .parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.kind == core::PartKind::Window)
            .filter_map(|(i, p)| {
                let ring = core::part_ring(&pts, &spec, p)?;
                let lifted = ring
                    .iter()
                    .map(|q| [q.x, q.y, core::height_at(&pts, &spec, *q).unwrap_or(spec.base)])
                    .collect();
                Some((i, lifted, ring))
            })
            .collect();
        let parts = r
            .roof
            .parts
            .iter()
            .enumerate()
            .map(|(i, part)| PartMeasure {
                part: part.clone(),
                top: matches!(part.kind, RoofPartKind::Chimney | RoofPartKind::Vent).then(|| part.rise_m()).flatten(),
                facade: windows.iter().find(|(k, ..)| *k == i).map(|(_, _, ring)| {
                    (ring.iter().map(|q| inside(&pts, *q)).fold(f64::INFINITY, f64::min) - overhang).max(0.0)
                }),
            })
            .collect();
        let mut window_gaps = Vec::new();
        for (n, (i, a, _)) in windows.iter().enumerate() {
            for (j, b, _) in windows.iter().skip(n + 1) {
                window_gaps.push((*i, *j, ring_distance(a, b)));
            }
        }
        out.push(RoofMeasure {
            slot: Slot(e.base().id),
            at,
            sloping,
            ridge,
            overhang: r.roof.overhang,
            slopes: r.roof.slopes.clone(),
            snow_guard: r.roof.snow_guard.is_some(),
            parts,
            window_gaps,
        });
    }
    out
}

/// How far in from the eaves line `pts` (convex, either way round) the plan point `q` is: its nearest edge's line.
fn inside(pts: &[V], q: V) -> f64 {
    let n = pts.len();
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let turn = if area >= 0.0 { 1.0 } else { -1.0 };
    (0..n)
        .filter_map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let l = (b.x - a.x).hypot(b.y - a.y);
            (l > 1e-12).then(|| turn * ((b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x)) / l)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The shortest distance between two closed rings in space: their edges' nearest points.
fn ring_distance(a: &[[f64; 3]], b: &[[f64; 3]]) -> f64 {
    let edges = |r: &[[f64; 3]]| -> Vec<([f64; 3], [f64; 3])> {
        (0..r.len()).map(|i| (r[i], r[(i + 1) % r.len()])).collect()
    };
    let (ea, eb) = (edges(a), edges(b));
    ea.iter()
        .flat_map(|(p, q)| {
            eb.iter()
                .map(move |(s, t)| segment_distance(*p, *q, *s, *t))
        })
        .fold(f64::INFINITY, f64::min)
}

/// The shortest distance between the segments `p`→`q` and `s`→`t` in space (Ericson, Real-Time Collision Detection
/// 5.1.9: the closest points of two segments).
fn segment_distance(p: [f64; 3], q: [f64; 3], s: [f64; 3], t: [f64; 3]) -> f64 {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let at = |o: [f64; 3], d: [f64; 3], k: f64| [o[0] + d[0] * k, o[1] + d[1] * k, o[2] + d[2] * k];
    let (d1, d2, r) = (sub(q, p), sub(t, s), sub(p, s));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (k1, k2) = if a <= 1e-18 && e <= 1e-18 {
        (0.0, 0.0)
    } else if a <= 1e-18 {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= 1e-18 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut k1 = if denom > 1e-18 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut k2 = (b * k1 + f) / e;
            if k2 < 0.0 {
                k2 = 0.0;
                k1 = (-c / a).clamp(0.0, 1.0);
            } else if k2 > 1.0 {
                k2 = 1.0;
                k1 = ((b - c) / a).clamp(0.0, 1.0);
            }
            (k1, k2)
        }
    };
    let gap = sub(at(p, d1, k1), at(s, d2, k2));
    dot(gap, gap).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_apart_in_space() {
        // Parallel, 2 m apart; crossing at a height of 1 m; end to end.
        let d = segment_distance(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [1.0, 2.0, 0.0],
        );
        assert!((d - 2.0).abs() < 1e-12);
        let d = segment_distance(
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
        );
        assert!((d - 1.0).abs() < 1e-12);
        let d = segment_distance(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [5.0, 0.0, 0.0],
        );
        assert!((d - 3.0).abs() < 1e-12);
    }

    #[test]
    fn a_point_in_from_the_eaves() {
        let rect = [
            V::new(0.0, 0.0),
            V::new(12.0, 0.0),
            V::new(12.0, 9.0),
            V::new(0.0, 9.0),
        ];
        assert!((inside(&rect, V::new(3.0, 1.5)) - 1.5).abs() < 1e-12);
        let clockwise: Vec<V> = rect.iter().rev().copied().collect();
        assert!((inside(&clockwise, V::new(11.0, 4.0)) - 1.0).abs() < 1e-12);
    }

}
