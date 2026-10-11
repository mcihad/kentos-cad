//! Hat boyunca nokta (docs/adr/0220 §5): points along each path, every so
//! far from a start offset up to an end offset or at equal parts, the ends
//! when asked; each with its distance, the direction the path runs there
//! and its elevation.

use super::walk::Walk;
use crate::jsmath::{atan2, js_max, js_min};
use crate::vec2::Vec2;

/// How the points are spaced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spacing {
    /// Every `step` from `start`, up to `end` before the path's end.
    Interval { step: f64, start: f64, end: f64 },
    /// At equal parts.
    Parts(u64),
}

/// The distances along a path of length `l` (§5).
pub fn distances(l: f64, closed: bool, spacing: &Spacing, ends: bool) -> Vec<f64> {
    let eps = 1e-9 * js_max(1.0, l);
    let mut out = Vec::new();
    match *spacing {
        Spacing::Interval { step, start, end } => {
            if step > 0.0 {
                let mut k = 0u64;
                loop {
                    let s = start + (k as f64) * step;
                    if !(s <= l - end + eps) {
                        break;
                    }
                    out.push(js_min(s, l));
                    k += 1;
                }
            }
            if closed {
                // Round a closed path its length is its start again.
                out.retain(|&s| s <= l - eps);
            }
            if ends {
                if out.first().is_none_or(|&f| f > eps) {
                    out.insert(0, 0.0);
                }
                if !closed && out.last().is_none_or(|&s| s < l - eps) {
                    out.push(l);
                }
            }
        }
        Spacing::Parts(n) => {
            let n = n.max(1);
            if ends {
                out.push(0.0);
            }
            for k in 1..n {
                out.push((l * k as f64) / n as f64);
            }
            if ends && !closed {
                out.push(l);
            }
        }
    }
    out
}

/// At most how many distances a path of length `l` gets: the limit is read before anything is built.
pub fn how_many(l: f64, spacing: &Spacing, ends: bool) -> f64 {
    let extra = if ends { 2.0 } else { 0.0 };
    match *spacing {
        Spacing::Interval { step, start, end } => {
            let eps = 1e-9 * js_max(1.0, l);
            let span = l - end + eps - start;
            let n = if step > 0.0 && span >= 0.0 {
                (span / step).floor() + 1.0
            } else {
                0.0
            };
            n + extra
        }
        Spacing::Parts(n) => (n.max(1) as f64) + 1.0,
    }
}

/// A point Hat boyunca nokta writes: where, its elevation, its distance
/// along its path, the path's bearing there (radians, clockwise from north)
/// and the path's place in the object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub p: Vec2,
    pub z: Option<f64>,
    pub s: f64,
    pub bearing: f64,
    pub path: usize,
}

/// A direction's bearing: clockwise from north, in [0, 2π).
pub fn bearing_of(d: Vec2) -> f64 {
    let b = atan2(d.x, d.y);
    if b < 0.0 {
        b + std::f64::consts::TAU
    } else {
        b
    }
}

/// An object's points along its paths, in order.
pub fn marks(walks: &[Walk], spacing: &Spacing, ends: bool) -> Vec<Mark> {
    let mut out = Vec::new();
    for (k, w) in walks.iter().enumerate() {
        for s in distances(w.length, w.closed, spacing, ends) {
            let spot = w.at(s);
            out.push(Mark {
                p: spot.p,
                z: spot.z,
                s,
                bearing: bearing_of(spot.dir),
                path: k,
            });
        }
    }
    out
}
