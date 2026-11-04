//! Where two alternatives of a road part (docs/adr/0943, plan/road.md Y37; ROAD-ALIGN-12, ROAD-VOL-15): the stretches
//! of an alternative's alignment farther than a tolerance from the base's in plan, and, where they lie together, the
//! stretches whose elevations differ by more than a height tolerance.
//!
//! Both are found by marching with steps no crossing can hide in. The distance from a point moving along the
//! alternative (at unit speed in its station) to the base alignment changes at most as fast as the point moves: it is
//! 1-Lipschitz, so from a station where it is `d` it cannot reach the tolerance `t` within `|d − t|`. The elevation
//! difference changes at most as fast as the two grades allow: the alternative's largest grade plus the base's,
//! stretched by the base's station moving `1/(1 − κ·t)` times as fast where the two lie within `t` (κ the base's largest
//! curvature). Each step is that far, at least [`LEAST_STEP`]; a step that changes the side is bisected to 1e-9 m. A
//! departure shorter than the least step can be missed; nothing longer is.

use crate::alignment::Alignment;
use crate::profile::vertical::Vertical;

/// The shortest step (m): departures shorter than this along the alternative may go unseen.
pub const LEAST_STEP: f64 = 0.01;

/// Boundaries are found to this (m).
const BISECTED: f64 = 1e-9;

/// A stretch of the alternative that departs from the base: its stations on the alternative, the base's stations
/// nearest its ends (none where an end lies beyond the base), and its largest departure (m) and where (the
/// alternative's station).
#[derive(Clone, Debug, PartialEq)]
pub struct Departure {
    pub from: f64,
    pub to: f64,
    pub base_from: Option<f64>,
    pub base_to: Option<f64>,
    pub largest: f64,
    pub at: f64,
}

/// The distance from `p` to the base and the base's station there (its nearest foot, else the end it lies beyond:
/// then no station).
fn gap(base: &Alignment, p: [f64; 2]) -> (f64, Option<f64>) {
    let found = base.locate(p);
    match (found.nearest, found.beyond) {
        (Some(f), _) => (f.offset.abs(), Some(f.at.station)),
        (None, Some((_, f))) => ((p[0] - f.at.x).hypot(p[1] - f.at.y), None),
        (None, None) => (f64::INFINITY, None),
    }
}

/// The stretches of `[lo, hi]` where `value(s) > 0`, `value` changing at most `rate` per metre (∞ where undefined: the
/// stretch is not compared there; it counts as not departed).
fn march(lo: f64, hi: f64, rate: f64, value: &dyn Fn(f64) -> Option<f64>) -> Vec<(f64, f64)> {
    let out_at = |s: f64| value(s).is_some_and(|v| v > 0.0);
    let mut out = Vec::new();
    if hi <= lo {
        return out;
    }
    let mut s = lo;
    let mut inside = out_at(s);
    let mut start = inside.then_some(lo);
    while s < hi {
        let step = match value(s) {
            Some(v) => (v.abs() / rate.max(1e-12)).max(LEAST_STEP),
            None => LEAST_STEP,
        };
        let next = (s + step).min(hi);
        let now = out_at(next);
        if now != inside {
            // The side changes within (s, next]: bisect.
            let (mut a, mut b) = (s, next);
            while b - a > BISECTED {
                let m = 0.5 * (a + b);
                if out_at(m) == inside {
                    a = m;
                } else {
                    b = m;
                }
            }
            let edge = if inside { a } else { b };
            if inside {
                out.push((start.take().unwrap_or(lo), edge));
            } else {
                start = Some(edge);
            }
            inside = now;
        }
        s = next;
    }
    if let Some(a) = start {
        out.push((a, hi));
    }
    out
}

/// The largest of `value` on `[a, b]` and where: sampled every half metre at least 16 times, refined by golden section
/// around the best sample.
fn largest(a: f64, b: f64, value: &dyn Fn(f64) -> f64) -> (f64, f64) {
    let n = (((b - a) / 0.5).ceil() as usize).clamp(16, 4000);
    let h = (b - a) / n as f64;
    let (mut best, mut at) = (f64::NEG_INFINITY, a);
    for i in 0..=n {
        let s = a + h * i as f64;
        let v = value(s);
        if v > best {
            best = v;
            at = s;
        }
    }
    let (mut lo, mut hi) = ((at - h).max(a), (at + h).min(b));
    let g = 0.618_033_988_749_895;
    for _ in 0..60 {
        let (c, d) = (hi - g * (hi - lo), lo + g * (hi - lo));
        if value(c) >= value(d) {
            hi = d;
        } else {
            lo = c;
        }
    }
    let m = 0.5 * (lo + hi);
    let v = value(m);
    if v > best { (v, m) } else { (best, at) }
}

/// The stretches of `other` farther than `tolerance` (m) from `base` in plan.
pub fn plan(base: &Alignment, other: &Alignment, tolerance: f64) -> Vec<Departure> {
    let point = |s: f64| other.at(s).map(|l| [l.x, l.y]);
    let value = |s: f64| point(s).map(|p| gap(base, p).0 - tolerance);
    march(0.0, other.length, 1.0, &value)
        .into_iter()
        .map(|(from, to)| {
            let station = |s: f64| point(s).and_then(|p| gap(base, p).1);
            let (largest, at) = largest(from, to, &|s| point(s).map_or(0.0, |p| gap(base, p).0));
            Departure { from, to, base_from: station(from), base_to: station(to), largest, at }
        })
        .collect()
}

/// The largest absolute grade of a profile (its pieces' grades are monotone between their ends).
fn steepest(v: &Vertical) -> f64 {
    v.pieces
        .iter()
        .flat_map(|p| {
            let (a, b) = p.range();
            [p.at(a).1.abs(), p.at(b).1.abs()]
        })
        .fold(0.0, f64::max)
}

/// The largest absolute curvature of an alignment (each element's is linear between its ends).
fn sharpest(a: &Alignment) -> f64 {
    a.elements.iter().flat_map(|e| [e.k0.abs(), e.k1.abs()]).fold(0.0, f64::max)
}

/// Where `other` lies within `tolerance` of `base` in plan and both have elevations, the stretches whose elevations
/// differ by more than `height` (m): the alternative's elevation at its station against the base's at the station of
/// the base nearest to it.
pub fn height(base: &Alignment, base_z: &Vertical, other: &Alignment, other_z: &Vertical, tolerance: f64, height: f64) -> Vec<Departure> {
    let together = |s: f64| -> Option<f64> {
        let l = other.at(s)?;
        let (d, station) = gap(base, [l.x, l.y]);
        if d > tolerance {
            return None;
        }
        let zo = other_z.at(s)?.0;
        let zb = base_z.at(station?)?.0;
        Some(zo - zb)
    };
    let stretch = 1.0 / (1.0 - (sharpest(base) * tolerance).min(0.5));
    let rate = steepest(other_z) + steepest(base_z) * stretch + 1e-12;
    let value = |s: f64| together(s).map(|dz| dz.abs() - height);
    // Where the plans part, nothing is compared: the plan's departures say it.
    march(0.0, other.length, rate, &value)
        .into_iter()
        .map(|(from, to)| {
            let station = |s: f64| other.at(s).and_then(|l| gap(base, [l.x, l.y]).1);
            let (largest, at) = largest(from, to, &|s| together(s).map_or(0.0, f64::abs));
            Departure { from, to, base_from: station(from), base_to: station(to), largest, at }
        })
        .collect()
}
