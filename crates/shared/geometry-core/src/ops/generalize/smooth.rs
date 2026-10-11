//! Yumuşat (docs/adr/0221 §2): Chaikin's corner cutting, QGIS's Smooth.
//! Each straight edge of a path gives two points a fraction along it from
//! either end; a fixed vertex (an open path's ends, an arc edge's ends, a
//! vertex turning more than the largest angle) stays itself and arcs stay
//! whole. Elevations go along by the same fraction.

use super::Track;
use crate::jsmath::atan2;
use crate::vec2::Vec2;

/// Yumuşat's settings: how many times, the fraction cut off each end of an
/// edge, the largest turn (radians) of a vertex that is smoothed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Smoothing {
    pub iterations: u32,
    pub offset: f64,
    pub max_turn: f64,
}

/// How far the path turns at vertex `i` (0 … π): the angle between the
/// way in and the way out.
fn turn(t: &Track, i: usize) -> f64 {
    let n = t.pts.len();
    let (p, c, q) = (t.pts[(i + n - 1) % n], t.pts[i], t.pts[(i + 1) % n]);
    let (ux, uy, wx, wy) = (c.x - p.x, c.y - p.y, q.x - c.x, q.y - c.y);
    if (ux == 0.0 && uy == 0.0) || (wx == 0.0 && wy == 0.0) {
        return 0.0;
    }
    atan2(ux * wy - uy * wx, ux * wx + uy * wy).abs()
}

fn fixed(t: &Track, max_turn: f64) -> Vec<bool> {
    let n = t.pts.len();
    (0..n)
        .map(|i| {
            if !t.closed && (i == 0 || i + 1 == n) {
                return true;
            }
            t.on_arc(i) || turn(t, i) > max_turn
        })
        .collect()
}

fn along(za: Option<f64>, zb: Option<f64>, f: f64) -> Option<f64> {
    Some(za? + f * (zb? - za?))
}

/// One pass of corner cutting.
fn step(t: &Track, o: f64, max_turn: f64) -> Track {
    let n = t.pts.len();
    let fix = fixed(t, max_turn);
    let back = 1.0 - o;
    let mut out = Track::empty(t.closed);
    for i in 0..t.edges() {
        let j = (i + 1) % n;
        let (a, c) = (t.pts[i], t.pts[j]);
        if t.bulges[i] != 0.0 {
            out.push(a, t.bulges[i], t.zs[i]);
            continue;
        }
        if fix[i] {
            out.push(a, 0.0, t.zs[i]);
        } else {
            let q = Vec2::new(a.x + o * (c.x - a.x), a.y + o * (c.y - a.y));
            out.push(q, 0.0, along(t.zs[i], t.zs[j], o));
        }
        if fix[j] {
            out.push(c, 0.0, t.zs[j]);
        } else {
            let r = Vec2::new(a.x + back * (c.x - a.x), a.y + back * (c.y - a.y));
            out.push(r, 0.0, along(t.zs[i], t.zs[j], back));
        }
    }
    if !t.closed && n > 0 {
        // A path ending in an arc: the arc's end.
        out.push(t.pts[n - 1], 0.0, t.zs[n - 1]);
    }
    out.close_up();
    out
}

/// A path smoothed; one too short to have a vertex to cut stays.
pub fn smooth_track(t: &Track, s: &Smoothing) -> Track {
    let mut cur = t.clone();
    for _ in 0..s.iterations {
        if cur.pts.len() < 3 {
            break;
        }
        cur = step(&cur, s.offset, s.max_turn);
    }
    cur
}
