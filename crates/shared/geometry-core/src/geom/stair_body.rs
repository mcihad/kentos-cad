//! A stair's body (docs/adr/8042 §3, 8045 §2, 8071 §9, 8080): its flights, a balanced stair's winders, a curved stair's
//! treads and centre post, its landings, each a plan outline with its top and its underside, over the floor the stair
//! starts from. The section model draws it; the regulation's areas read its underside: the place under a stair lower
//! than 1.80 m is out of a unit's net area (Planlı Alanlar İmar Yönetmeliği 4/1-ğ-3) and is closed off (31/6).
//!
//! - **Flight:** each straight leg past the landings (and a balanced stair's turns): its top the treads' heights, its
//!   underside the waist under the steps' roots, measured upright, never below its floor (the floor it starts from, or
//!   under a landing the landing's underside).
//! - **Winder:** a balanced stair's tread between two risers: its top its height, its bottom a riser and the waist under
//!   it, never under its floor; a stepped underside (the warped one a balanced stair's concrete has is not modelled).
//! - **Curved tread:** each tread of a curved stair, a piece of the ring round its walking line's centre with its arcs
//!   true, standing as a winder does; its centre post a cylinder from its floor to where it arrives.
//! - **Landing:** the stair's band between a landing's ends, as thick as the waist.
//!
//! The waist is the structure of the slab the stair arrives at (else the one it starts from); without one the body is
//! solid down to its floor.

use crate::entity::{Shape, polygon_ring};
use crate::geom::arch::{
    StairSpec, Walk, riser_stations, stair_band, stair_landings, tread_ring, winder_turns,
};
use crate::geom::bulge::{bulge_of_sweep, bulge_ring_area};
use crate::geom::section::Flight;
use crate::jsmath::{js_max, js_min};
use crate::vec2::Vec2;

/// A part of a stair's body.
#[derive(Clone, Debug, PartialEq)]
pub enum StairPart {
    /// A straight leg's flight: its outline (convex) and its steps and underside.
    Flight { ring: Vec<Vec2>, flight: Flight },
    /// A landing, a winder, a curved tread or a curved stair's post: its outline (a curved one's with its arcs'
    /// bulges, counter-clockwise) between `bottom` and `top`.
    Prism {
        ring: Vec<Vec2>,
        bulges: Option<Vec<f64>>,
        bottom: f64,
        top: f64,
    },
}

/// The body of the stair `spec` along its walking line `pts`, over its floor `base`, its waist `waist` (none: solid down
/// to its floor); in order: a straight stair's flights, its winders, its landings; a curved stair's treads, then its
/// post. Empty for a walking line of fewer than two points, or a curved one whose arc cannot be read.
pub fn stair_body(pts: &[Vec2], spec: &StairSpec, base: f64, waist: Option<f64>) -> Vec<StairPart> {
    if pts.len() < 2 {
        return Vec::new();
    }
    if spec.bulge != 0.0 {
        return curved(pts, spec, base, waist);
    }
    let rh = spec.riser_height;
    let walk = Walk::of(pts);
    let (stations, _) = riser_stations(&walk, spec);
    let landings = stair_landings(&walk, spec);
    let winding = winding_zones(&walk, spec, &stations);
    let stops: &[(f64, f64)] = if winding.is_empty() {
        &landings
    } else {
        &winding
    };
    let half = spec.width / 2.0;
    let slope = rh / spec.tread;
    let upright = waist.map_or(f64::INFINITY, |w| w * (1.0 + slope * slope).sqrt());
    let mut out = Vec::new();
    // Each segment of the walking line: its flight's stretch past the landings.
    for k in 0..pts.len() - 1 {
        let (a, b) = (pts[k], pts[k + 1]);
        let (sa, sb) = (walk.at[k], walk.at[k + 1]);
        let len = sb - sa;
        if len <= 1e-9 {
            continue;
        }
        let dir = Vec2::new((b.x - a.x) / len, (b.y - a.y) / len);
        let across = Vec2::new(-dir.y, dir.x);
        // Its flight: from the end of a landing that holds its start (else its start) to the start of one that holds
        // its end (else its end).
        let lo = stops
            .iter()
            .find(|(x, y)| *x <= sa + 1e-9 && *y > sa + 1e-9)
            .map_or(sa, |(_, y)| *y);
        let hi = stops
            .iter()
            .find(|(x, y)| *x < sb - 1e-9 && *y >= sb - 1e-9)
            .map_or(sb, |(x, _)| *x);
        if hi - lo <= 1e-9 {
            continue;
        }
        let at = |s: f64| Vec2::new(a.x + dir.x * (s - sa), a.y + dir.y * (s - sa));
        let (fa, fb) = (at(lo), at(hi));
        let ring = vec![
            Vec2::new(fa.x - across.x * half, fa.y - across.y * half),
            Vec2::new(fb.x - across.x * half, fb.y - across.y * half),
            Vec2::new(fb.x + across.x * half, fb.y + across.y * half),
            Vec2::new(fa.x + across.x * half, fa.y + across.y * half),
        ];
        // Its risers, by their numbers along the whole stair.
        let mine: Vec<(usize, f64)> = stations
            .iter()
            .enumerate()
            .filter(|(_, st)| **st >= lo - 1e-9 && **st < hi - 1e-9)
            .map(|(i, st)| (i, *st))
            .collect();
        let Some(&(first, s0)) = mine.first() else {
            continue;
        };
        let mut steps: Vec<(f64, f64)> = Vec::new();
        if s0 > lo + 1e-9 {
            steps.push((0.0, base + rh * first as f64));
        }
        for (i, st) in &mine {
            steps.push((st - lo, base + rh * (*i + 1) as f64));
        }
        // The steps' roots: the first riser's foot, rising by the riser over the tread.
        let root = base + rh * first as f64 - (s0 - lo) * slope;
        let under_landing = if lo > sa + 1e-9 || k > 0 {
            base + rh * first as f64 - waist.unwrap_or(0.0)
        } else {
            base
        };
        out.push(StairPart::Flight {
            ring,
            flight: Flight {
                start: fa,
                dir,
                steps,
                soffit: root - upright,
                slope,
                floor: under_landing,
            },
        });
    }
    // Its winders: each tread between its two risers.
    for (a, b) in &winding {
        let mine: Vec<(usize, f64)> = stations
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, st)| *st >= a - 1e-9 && *st <= b + 1e-9)
            .collect();
        for w in mine.windows(2) {
            let ((i, s0), (_, s1)) = (w[0], w[1]);
            let ring = tread_ring(&walk, spec, s0, s1);
            if ring.len() < 3 {
                continue;
            }
            let top = base + rh * (i + 1) as f64;
            let bottom = waist.map_or(base, |wst| js_max(top - rh - wst, base));
            out.push(StairPart::Prism {
                ring,
                bulges: None,
                bottom,
                top,
            });
        }
    }
    // Its landings: the band between their ends, as thick as its waist (or down to its floor without one).
    if let Shape::Polygon { pts: band, .. } = stair_band(pts, spec) {
        for (x, y) in &landings {
            let risers_before = stations.iter().filter(|st| **st < *x + 1e-9).count();
            let top = base + rh * risers_before as f64;
            let bottom = waist.map_or(base, |w| top - w);
            let Some(ring) = band_cut(&walk, &band, *x, *y) else {
                continue;
            };
            out.push(StairPart::Prism {
                ring,
                bulges: None,
                bottom,
                top,
            });
        }
    }
    out
}

/// A balanced stair's turns (docs/adr/8045 §2) by its risers' stations: each winds from its last square riser before
/// its zone to its first after it (else the stair's last); its flights stop there as at a landing and its treads stand
/// one by one.
pub(crate) fn winding_zones(walk: &Walk, spec: &StairSpec, stations: &[f64]) -> Vec<(f64, f64)> {
    winder_turns(walk, spec)
        .iter()
        .filter_map(|t| {
            let a = stations
                .iter()
                .copied()
                .rfind(|st| *st <= t.corner - t.reach + 1e-9)?;
            let b = stations
                .iter()
                .copied()
                .find(|st| *st >= t.corner + t.reach - 1e-9)
                .or_else(|| stations.last().copied())?;
            (b > a + 1e-9).then_some((a, b))
        })
        .collect()
}

/// A curved stair's treads and post (docs/adr/0709, 8071 §9).
fn curved(pts: &[Vec2], spec: &StairSpec, base: f64, waist: Option<f64>) -> Vec<StairPart> {
    let rh = spec.riser_height;
    let n = spec.risers as usize;
    let walk = Walk::curved(pts, spec.bulge);
    let Some(arc) = walk.arc else {
        return Vec::new();
    };
    let (stations, _) = riser_stations(&walk, spec);
    let half = spec.width / 2.0;
    let turn = if arc.sweep < 0.0 { -1.0 } else { 1.0 };
    // A point `k` half widths left of the walking line `s` along it.
    let side = |s: f64, k: f64| {
        let (p, d) = walk.point(s);
        Vec2::new(p.x - d.y * half * k, p.y + d.x * half * k)
    };
    let mut out = Vec::new();
    for (i, w) in stations.windows(2).enumerate() {
        let (s0, s1) = (w[0], w[1]);
        let b = bulge_of_sweep(turn * (s1 - s0) / arc.r);
        // Counter-clockwise: from the side whose arc runs it so.
        let ring_of = |k: f64| vec![side(s0, k), side(s1, k), side(s1, -k), side(s0, -k)];
        let bulges = vec![b, 0.0, -b, 0.0];
        let mut ring = ring_of(1.0);
        if bulge_ring_area(&ring, Some(&bulges)) < 0.0 {
            ring = ring_of(-1.0);
        }
        let top = base + rh * (i + 1) as f64;
        let bottom = waist.map_or(base, |wst| js_max(top - rh - wst, base));
        out.push(StairPart::Prism {
            ring,
            bulges: Some(bulges),
            bottom,
            top,
        });
    }
    if let Some(d) = spec.post.filter(|d| *d > 0.0) {
        out.push(StairPart::Prism {
            ring: vec![
                Vec2::new(arc.c.x + d / 2.0, arc.c.y),
                Vec2::new(arc.c.x - d / 2.0, arc.c.y),
            ],
            bulges: Some(vec![1.0, 1.0]),
            bottom: base,
            top: base + rh * n as f64,
        });
    }
    out
}

/// The band `ring` (a stair's or a ramp's, round its walking line) between the line's points `x` and `y` along it,
/// cut square to the line at both ends.
pub fn band_cut(walk: &Walk, ring: &[Vec2], x: f64, y: f64) -> Option<Vec<Vec2>> {
    let (px, dx) = walk.point(x);
    let (py, dy) = walk.point(y);
    // Keep the band's part after the cross line at x (along dx) and before the one at y (along dy).
    let after = move |q: Vec2| (q.x - px.x) * dx.x + (q.y - px.y) * dx.y;
    let before = move |q: Vec2| (py.x - q.x) * dy.x + (py.y - q.y) * dy.y;
    let mut out = clip(ring, &after);
    out = clip(&out, &before);
    (out.len() >= 3).then_some(out)
}

/// A ring cut to where `keep` is not below 0. A corner on the cut line is one corner of the cut ring, not one for each
/// edge that meets it there (docs/adr/8071 §1: no edge of no length, every body closed and simple).
pub fn clip(ring: &[Vec2], keep: &dyn Fn(Vec2) -> f64) -> Vec<Vec2> {
    let n = ring.len();
    let mut out: Vec<Vec2> = Vec::new();
    let same = |p: Vec2, q: Vec2| (p.x - q.x).abs() <= 1e-12 && (p.y - q.y).abs() <= 1e-12;
    let put = |p: Vec2, out: &mut Vec<Vec2>| {
        if out.last().is_none_or(|q| !same(*q, p)) {
            out.push(p);
        }
    };
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (ka, kb) = (keep(a), keep(b));
        if ka >= 0.0 {
            put(a, &mut out);
        }
        if (ka >= 0.0) != (kb >= 0.0) {
            let t = ka / (ka - kb);
            put(Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t), &mut out);
        }
    }
    while out.len() > 1 && same(out[0], out[out.len() - 1]) {
        out.pop();
    }
    out
}

/// The clear height under a stair Planlı Alanlar İmar Yönetmeliği reads (4/1-ğ-3, 31/6), metres.
pub const UNDER_STAIR_CLEAR: f64 = 1.80;

/// The places under a stair's body lower than `clear` over its floor `base`, in plan (arcs tessellated), each with
/// whether a space stands under it (its underside over the floor) or the body is solid down to the floor there.
///
/// - With its waist known (`waist`): a flight's place runs from its start to where both its underside and its treads
///   have risen to `clear`, solid while its underside lies on its floor; a prism's is all of it when its bottom is under
///   `clear`, a space when its bottom is over its floor.
/// - Without (no slab to take it from): what is sure, the places under treads and landings lower than `clear`, taken as
///   solid (the section draws the body down to its floor).
pub fn low_under(parts: &[StairPart], base: f64, clear: f64, waist: bool) -> Vec<(Vec<Vec2>, bool)> {
    let limit = base + clear;
    let mut out = Vec::new();
    for part in parts {
        match part {
            StairPart::Flight { ring, flight: f } => {
                // Where its treads reach the limit (the underside never stands over the tread above it) …
                let by_treads = f
                    .steps
                    .iter()
                    .find(|(_, h)| *h >= limit)
                    .map_or(f64::INFINITY, |(from, _)| js_max(*from, 0.0));
                // … and where its underside line does (never, if its floor is already over it).
                let by_soffit = if !waist || f.floor >= limit {
                    0.0
                } else if f.slope > 1e-12 {
                    js_max((limit - f.soffit) / f.slope, 0.0)
                } else {
                    f64::INFINITY
                };
                let reach = js_max(by_soffit, by_treads);
                if reach <= 1e-12 {
                    continue;
                }
                // Solid where its underside lies on its floor: from its start to where the slope leaves the floor; a
                // flight over a landing's underside has a space under it from its start.
                let solid_to = if !waist {
                    reach
                } else if f.floor > base + 1e-9 {
                    0.0
                } else if f.slope > 1e-12 {
                    js_min(js_max((base - f.soffit) / f.slope, 0.0), reach)
                } else {
                    reach
                };
                let cut = |from: Option<f64>, to: f64| {
                    let r = match from {
                        Some(a) => clip(ring, &|p: Vec2| f.s(p) - a),
                        None => ring.clone(),
                    };
                    if to.is_finite() {
                        clip(&r, &|p: Vec2| to - f.s(p))
                    } else {
                        r
                    }
                };
                if solid_to > 1e-12 {
                    let r = cut(None, solid_to);
                    if r.len() >= 3 {
                        out.push((r, false));
                    }
                }
                if reach > solid_to + 1e-12 {
                    let r = cut((solid_to > 1e-12).then_some(solid_to), reach);
                    if r.len() >= 3 {
                        out.push((r, true));
                    }
                }
            }
            StairPart::Prism {
                ring,
                bulges,
                bottom,
                top,
            } => {
                let low = if waist { *bottom < limit } else { *top < limit };
                if !low {
                    continue;
                }
                let open = waist && *bottom > base + 1e-9;
                out.push((polygon_ring(ring, bulges.as_deref()), open));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(risers: f64, rh: f64, tread: f64) -> StairSpec {
        StairSpec {
            bulge: 0.0,
            width: 1.2,
            risers,
            riser_height: rh,
            tread,
            landing: None,
            cut: None,
            numbering: false,
            railings: None,
            post: None,
            upper: false,
            winders: None,
        }
    }

    fn area(r: &[Vec2]) -> f64 {
        let n = r.len();
        ((0..n).map(|i| r[i].x * r[(i + 1) % n].y - r[(i + 1) % n].x * r[i].y).sum::<f64>() / 2.0).abs()
    }

    /// A straight flight of 18 risers of 0.17 m and treads of 0.28 m from the floor at ±0.00, its waist 0.15 m:
    /// worked by hand. The slope is 0.17/0.28; the steps' roots start at 0 and the underside sits the waist measured
    /// upright, 0.15·√(1 + slope²) = 0.175 63…, under them. The underside leaves the floor at s = 0.175 63/slope =
    /// 0.289 27 m and reaches 1.80 m at s = (1.80 + 0.175 63)/slope = 3.253 9 m; the tread over it stands at 1.87 m
    /// (the 11th) from 2.80 m: the underside binds. Solid 0.289 27 × 1.2 m, a space under 1.80 m the rest to 3.253 9.
    #[test]
    fn a_straight_flight_is_solid_then_low_up_to_where_its_underside_reaches_1_80() {
        let s = spec(18.0, 0.17, 0.28);
        let pts = [Vec2::new(0.0, 0.0), Vec2::new(17.0 * 0.28, 0.0)];
        let parts = stair_body(&pts, &s, 0.0, Some(0.15));
        let low = low_under(&parts, 0.0, UNDER_STAIR_CLEAR, true);
        let slope: f64 = 0.17 / 0.28;
        let upright = 0.15 * (1.0 + slope * slope).sqrt();
        let lifts = upright / slope;
        let reach = (1.80 + upright) / slope;
        assert_eq!(low.len(), 2, "{low:?}");
        assert!(!low[0].1 && low[1].1);
        assert!((area(&low[0].0) - lifts * 1.2).abs() < 1e-9, "{}", area(&low[0].0));
        assert!((area(&low[1].0) - (reach - lifts) * 1.2).abs() < 1e-9, "{}", area(&low[1].0));
        // Without a waist the sure part: under the treads lower than 1.80 m, to the 11th tread (at 1.87 m) from 2.80 m.
        let solid = low_under(&stair_body(&pts, &s, 0.0, None), 0.0, UNDER_STAIR_CLEAR, false);
        assert_eq!(solid.len(), 1);
        assert!(!solid[0].1);
        assert!((area(&solid[0].0) - 10.0 * 0.28 * 1.2).abs() < 1e-9, "{}", area(&solid[0].0));
    }

    /// A U stair of 2 × 9 risers of 0.17 m, its landing at 1.53 m, waist 0.15: the landing's underside at 1.38 m is a
    /// low space all over; the upper flight starts over the landing's underside, its soffit at the landing's level.
    #[test]
    fn a_landing_under_1_80_is_a_low_space() {
        let s = spec(18.0, 0.17, 0.28);
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(8.0 * 0.28 + 0.6, 0.0),
            Vec2::new(8.0 * 0.28 + 0.6, 1.2),
            Vec2::new(0.0, 1.2),
        ];
        let parts = stair_body(&pts, &s, 0.0, Some(0.15));
        let landings: Vec<&StairPart> = parts.iter().filter(|p| matches!(p, StairPart::Prism { .. })).collect();
        assert!(!landings.is_empty(), "{parts:?}");
        for l in &landings {
            let StairPart::Prism { bottom, top, .. } = l else { continue };
            assert!((top - bottom - 0.15).abs() < 1e-9 && *bottom < 1.80);
        }
        let low = low_under(&parts, 0.0, UNDER_STAIR_CLEAR, true);
        assert!(low.iter().filter(|(_, open)| *open).count() > landings.len(), "{low:?}");
    }
}
