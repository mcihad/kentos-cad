//! Çatının ölçüleri (docs/adr/8044 §9, 8159; M44): each roof measured on the drawing for the standards package's roof
//! rules (`catalog/standards/mimari-mevzuat`, `cati_olculeri`: Planlı Alanlar İmar Yönetmeliği 40/4, 40/5, 42/1,
//! 42/2, the fire regulation's 58/2 and Sivas İmar Yönetmeliği 40/5, 42/5), which the audit facts carry
//! (`building.roofs[].form`).
//!
//! - **Mahya:** the ridge's top over the saçak seviyesi (PAİY 4/ppp: "son kat tavan döşemesi üst kotu", the top of the
//!   storey the roof stands on), its kot farkı counted; a sloping roof's only.
//! - **Saçak:** a roof drawn from its walls overhangs them by its overhang; none for another.
//! - **Eğimler, kar siperliği:** each edge's slope (none: a gable or a flat edge) and whether a snow guard is given.
//! - **Parçalar:** each chimney's, vent's and window's own values (the rules' fixes write them back), a chimney's top
//!   over the ridge and a vent's over the roof at its highest corner (its own, else its kind's); a window's outline's
//!   nearest distance to the building's facade lines (the walls' outer faces the roof was drawn from, its eaves its
//!   overhang in), and between two windows the shortest distance between their outlines on the roof.
//! - **Parsel:** with the site plan's parcel, how far the eaves outline reaches past its boundary (PAİY 42/2,
//!   docs/adr/8161): the farthest corner of the eaves' part outside it from the boundary, 0 when within.
//! - **Çıkma:** how far the eaves reach past the projections under the roof (PAİY 42/3, docs/adr/8164): from each
//!   projection's front face (an outer side whose inside runs back to the footprint; not its sides) along its outward
//!   normal to the eaves line, at the face's two ends 1 cm in and its middle, the points under the roof only; the
//!   largest. A projection on a storey over the roof's is not under it.
//! - **Duvarlar:** the walls rising over a sloping roof's covering, by how much and what they are (PAİY 40/5,
//!   docs/adr/8165; [`crate::roof_walls`]).
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
    /// How far the eaves reach past the site plan's parcel, m (0: within it); none without a site plan.
    pub outside: Option<f64>,
    /// How far the eaves reach past the front faces of the projections under it, m (docs/adr/8164); none when no
    /// projection's front face lies under it.
    pub over_projection: Option<f64>,
    /// The walls rising over its covering (docs/adr/8165; PAİY 40/5), in the drawing's order; none for a flat roof.
    pub walls: Vec<crate::roof_walls::WallOver>,
}

/// Every roof's measures, in the drawing's order; a roof whose shape cannot be built is left out.
pub fn measures(doc: &Document) -> Vec<RoofMeasure> {
    let mut out = Vec::new();
    // The site plan's parcel, anticlockwise (docs/adr/8161).
    let parcel: Option<Vec<V>> = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.site.as_ref())
        .and_then(|s| doc.site_report_for(s))
        .map(|r| r.parcel)
        .filter(|ring| ring.len() >= 3);
    // The projections and the footprint they stand out of (docs/adr/8164), each storey's floor by its id.
    let (footprint, projections) = doc.projection_parts();
    let floor_of = |id: &str| {
        doc.settings()
            .arch
            .as_ref()?
            .levels
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.elevation)
    };
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
        let ridge = sloping
            .then(|| core::ridge_rise(&pts, &spec).ok())
            .flatten()
            .map(|rise| rise + r.roof.offset);
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
                    .map(|q| {
                        [
                            q.x,
                            q.y,
                            core::height_at(&pts, &spec, *q).unwrap_or(spec.base),
                        ]
                    })
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
                top: matches!(part.kind, RoofPartKind::Chimney | RoofPartKind::Vent)
                    .then(|| part.rise_m())
                    .flatten(),
                facade: windows.iter().find(|(k, ..)| *k == i).map(|(_, _, ring)| {
                    (ring
                        .iter()
                        .map(|q| inside(&pts, *q))
                        .fold(f64::INFINITY, f64::min)
                        - overhang)
                        .max(0.0)
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
            outside: parcel.as_ref().map(|ring| past(&pts, ring)),
            over_projection: {
                // The projections on the roof's storey and under it; all of them for a roof on no storey.
                let top = e.own_level().and_then(floor_of);
                let under: Vec<&kentos_domain::projections::ProjectionPart> = projections
                    .iter()
                    .filter(|p| top.is_none_or(|f| p.floor <= f + 1e-6))
                    .collect();
                past_projections(&pts, &under, &footprint)
            },
            walls: crate::roof_walls::walls_over(doc, &pts, &spec),
        });
    }
    out
}

/// How far the outline `pts` reaches past the closed `ring`, m: the farthest corner of its part outside the ring from
/// the ring's sides; 0 when it stays within.
fn past(pts: &[V], ring: &[V]) -> f64 {
    use kentos_geometry_core::geom::arrangement::{Area, Ring};
    let area = |p: &[V]| Area {
        outer: Ring {
            pts: p.to_vec(),
            bulges: None,
        },
        holes: Vec::new(),
    };
    let rest = kentos_geometry_core::geom::region::subtract_areas(&[area(pts)], &[area(ring)]);
    let to_ring = |q: V| {
        (0..ring.len())
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len2 = dx * dx + dy * dy;
                let t = if len2 > 0.0 {
                    (((q.x - a.x) * dx + (q.y - a.y) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                kentos_num::hypot(q.x - (a.x + dx * t), q.y - (a.y + dy * t))
            })
            .fold(f64::INFINITY, f64::min)
    };
    rest.iter()
        .flat_map(|a| a.outer.pts.iter().copied())
        .map(to_ring)
        .fold(0.0, f64::max)
}

/// How far the eaves line `pts` reaches past the projections' front faces (docs/adr/8164; PAİY 42/3), m: from each
/// front face (an outer side of a projection, the open air beyond it, whose inside runs back to the footprint) along
/// its outward normal to the eaves line, at its two ends 1 cm in and its middle, the points under the roof only; the
/// largest; none when no front face lies under the roof.
fn past_projections(
    pts: &[V],
    parts: &[&kentos_domain::projections::ProjectionPart],
    footprint: &[kentos_geometry_core::geom::arrangement::Area],
) -> Option<f64> {
    use kentos_geometry_core::geom::arrangement::{Area, Ring};
    use kentos_geometry_core::geom::region::inside_area;
    let roof = Area {
        outer: Ring {
            pts: pts.to_vec(),
            bulges: None,
        },
        holes: Vec::new(),
    };
    let within = |shape: &[Area], q: V| shape.iter().any(|a| inside_area(a, q));
    let at = |q: V, d: V, t: f64| V::new(q.x + d.x * t, q.y + d.y * t);
    let mut most: Option<f64> = None;
    for p in parts {
        for area in &p.shape {
            let ring = &area.outer.pts;
            let n = ring.len();
            if n < 3 {
                continue;
            }
            let twice: f64 = (0..n)
                .map(|i| ring[i].x * ring[(i + 1) % n].y - ring[(i + 1) % n].x * ring[i].y)
                .sum();
            for k in 0..n {
                let (a, b) = (ring[k], ring[(k + 1) % n]);
                let len = kentos_num::hypot(b.x - a.x, b.y - a.y);
                if len < 0.02 {
                    continue;
                }
                let u = V::new((b.x - a.x) / len, (b.y - a.y) / len);
                // Outward: right of the side going anticlockwise.
                let out = if twice > 0.0 {
                    V::new(u.y, -u.x)
                } else {
                    V::new(-u.y, u.x)
                };
                let mid = V::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                // An outer side: beyond it neither the footprint nor another projection of its storey.
                let beyond = at(mid, out, 0.01);
                if within(footprint, beyond)
                    || parts
                        .iter()
                        .any(|q| q.level == p.level && within(&q.shape, beyond))
                {
                    continue;
                }
                // A front face: going in, the projection runs back to the footprint (its sides run along it).
                let back = V::new(-out.x, -out.y);
                let Some(depth) = first_hit(ring, mid, back) else {
                    continue;
                };
                if !within(footprint, at(mid, back, depth + 0.01)) {
                    continue;
                }
                for q in [at(a, u, 0.01), mid, at(b, u, -0.01)] {
                    if !inside_area(&roof, q) {
                        continue;
                    }
                    if let Some(t) = first_hit(pts, q, out) {
                        most = Some(most.map_or(t, |m: f64| m.max(t)));
                    }
                }
            }
        }
    }
    most
}

/// Where the ray from `q` along the unit `d` first crosses the closed outline `ring`, its distance; none when it does
/// not (a crossing at `q` itself is not one).
fn first_hit(ring: &[V], q: V, d: V) -> Option<f64> {
    let n = ring.len();
    let mut best: Option<f64> = None;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let e = V::new(b.x - a.x, b.y - a.y);
        let den = d.x * e.y - d.y * e.x;
        if den.abs() < 1e-12 {
            continue;
        }
        let w = V::new(a.x - q.x, a.y - q.y);
        let t = (w.x * e.y - w.y * e.x) / den;
        let s = (w.x * d.y - w.y * d.x) / den;
        if t > 1e-6 && (-1e-9..=1.0 + 1e-9).contains(&s) {
            best = Some(best.map_or(t, |m: f64| m.min(t)));
        }
    }
    best
}

/// How far in from the eaves line `pts` (convex, either way round) the plan point `q` is: its nearest edge's line.
fn inside(pts: &[V], q: V) -> f64 {
    let n = pts.len();
    let area: f64 = 2.0 * kentos_geometry_core::geometry::ring_area(pts);
    let turn = if area >= 0.0 { 1.0 } else { -1.0 };
    (0..n)
        .filter_map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let l = kentos_num::hypot(b.x - a.x, b.y - a.y);
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
    fn eaves_past_the_parcel() {
        let parcel = [
            V::new(0.0, 0.0),
            V::new(10.0, 0.0),
            V::new(10.0, 8.0),
            V::new(0.0, 8.0),
        ];
        let within = [
            V::new(1.0, 1.0),
            V::new(9.0, 1.0),
            V::new(9.0, 7.0),
            V::new(1.0, 7.0),
        ];
        assert!(past(&within, &parcel).abs() < 1e-9);
        // 1.25 m past the west side, clockwise.
        let over = [
            V::new(-1.25, 1.0),
            V::new(-1.25, 7.0),
            V::new(9.0, 7.0),
            V::new(9.0, 1.0),
        ];
        assert!(
            (past(&over, &parcel) - 1.25).abs() < 1e-9,
            "{}",
            past(&over, &parcel)
        );
    }

    /// PAİY 42/3 (docs/adr/8164): a 6 m wide balcony 1.20 m deep south of a 10 × 8 m footprint; the eaves 2.00 m south
    /// of the footprint reach 0.80 m past the balcony's front face (its sides, running along the footprint, are not
    /// fronts); drawn either way round; eaves short of the balcony's front give none.
    #[test]
    fn eaves_past_a_projection() {
        use kentos_geometry_core::geom::arrangement::{Area, Ring};
        let area = |pts: Vec<V>| Area {
            outer: Ring { pts, bulges: None },
            holes: Vec::new(),
        };
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
            vec![
                V::new(x0, y0),
                V::new(x1, y0),
                V::new(x1, y1),
                V::new(x0, y1),
            ]
        };
        let footprint = [area(rect(0.0, 0.0, 10.0, 8.0))];
        let balcony = kentos_domain::projections::ProjectionPart {
            room: "balkon".into(),
            level: Some("k01".into()),
            open: true,
            area: 7.2,
            shape: vec![area(rect(2.0, -1.2, 8.0, 0.0))],
            floor: 3.0,
        };
        let roof = rect(-1.0, -2.0, 11.0, 9.0);
        let past = past_projections(&roof, &[&balcony], &footprint).expect("under the roof");
        assert!((past - 0.8).abs() < 1e-9, "{past}");
        let clockwise: Vec<V> = roof.iter().rev().copied().collect();
        let past = past_projections(&clockwise, &[&balcony], &footprint).expect("under the roof");
        assert!((past - 0.8).abs() < 1e-9, "{past}");
        // The balcony drawn clockwise reads the same.
        let mut turned = balcony.clone();
        turned.shape = vec![area(rect(2.0, -1.2, 8.0, 0.0).into_iter().rev().collect())];
        let past = past_projections(&roof, &[&turned], &footprint).expect("under the roof");
        assert!((past - 0.8).abs() < 1e-9, "{past}");
        assert_eq!(
            past_projections(&rect(-1.0, -1.0, 11.0, 9.0), &[&balcony], &footprint),
            None
        );
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
