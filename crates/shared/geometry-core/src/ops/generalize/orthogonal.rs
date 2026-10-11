//! Dik açılandır (docs/adr/0221 §3): iD's, JOSM's and QGIS's orthogonalize.
//! A vertex whose corner is near a right angle or near straight moves along
//! its bisector a step towards square or straight; every vertex's step is
//! read from the same vertices, then all move; the squarest shape seen is
//! kept. An open path's ends and an arc edge's ends stay.

use super::Track;
use crate::jsmath::{PI, cos, js_min};
use crate::vec2::Vec2;

/// Below this squareness a shape is square (QGIS's 1e-8).
pub const SQUARE: f64 = 1e-8;

/// Dik açılandır's settings: the angle threshold (degrees) and the most
/// iterations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Squaring {
    pub threshold: f64,
    pub iterations: u32,
}

/// The dot products a corner is moved for: below `lower` near a right
/// angle, above `upper` near straight.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    lower: f64,
    upper: f64,
}

impl Bounds {
    fn of(threshold: f64) -> Bounds {
        Bounds {
            lower: cos((90.0 - threshold) * PI / 180.0),
            upper: cos(threshold * PI / 180.0),
        }
    }

    fn takes(&self, d: f64) -> bool {
        self.lower > d.abs() || d.abs() > self.upper
    }
}

fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn neighbours(t: &Track, i: usize) -> (Vec2, Vec2, Vec2) {
    let n = t.pts.len();
    (t.pts[(i + n - 1) % n], t.pts[i], t.pts[(i + 1) % n])
}

fn movable(t: &Track) -> Vec<bool> {
    let n = t.pts.len();
    (0..n)
        .map(|i| (t.closed || (i > 0 && i + 1 < n)) && !t.on_arc(i))
        .collect()
}

/// The step of corner `b` between `a` and `c`.
fn motion(a: Vec2, b: Vec2, c: Vec2, k: &Bounds) -> Vec2 {
    let zero = Vec2::new(0.0, 0.0);
    let (px, py, qx, qy) = (a.x - b.x, a.y - b.y, c.x - b.x, c.y - b.y);
    let (lp, lq) = (length(px, py), length(qx, qy));
    if lp == 0.0 || lq == 0.0 {
        return zero;
    }
    let scale = 2.0 * js_min(lp, lq);
    let (pnx, pny, qnx, qny) = (px / lp, py / lp, qx / lq, qy / lq);
    let mut d = pnx * qnx + pny * qny;
    if !k.takes(d) {
        return zero;
    }
    if d < -std::f64::consts::FRAC_1_SQRT_2 {
        d += 1.0;
    }
    let (vx, vy) = (pnx + qnx, pny + qny);
    let lv = length(vx, vy);
    if lv == 0.0 {
        return zero;
    }
    let f = 0.1 * d * scale;
    Vec2::new(vx / lv * f, vy / lv * f)
}

/// How far from square and straight the movable corners are.
fn squareness(t: &Track, moves: &[bool], k: &Bounds) -> f64 {
    let mut sum = 0.0;
    for (i, &m) in moves.iter().enumerate() {
        if !m {
            continue;
        }
        let (a, b, c) = neighbours(t, i);
        let (mut px, mut py, mut qx, mut qy) = (a.x - b.x, a.y - b.y, c.x - b.x, c.y - b.y);
        let (lp, lq) = (length(px, py), length(qx, qy));
        if lp > 0.0 {
            px /= lp;
            py /= lp;
        }
        if lq > 0.0 {
            qx /= lq;
            qy /= lq;
        }
        let d = px * qx + py * qy;
        if !k.takes(d) {
            continue;
        }
        sum += 2.0 * js_min((d - 1.0).abs(), js_min(d.abs(), (d + 1.0).abs()));
    }
    sum
}

/// A path squared, and whether it moved.
pub fn square_track(t: &Track, s: &Squaring) -> (Track, bool) {
    if t.pts.len() < 3 {
        return (t.clone(), false);
    }
    let k = Bounds::of(s.threshold);
    let moves = movable(t);
    let mut best = t.clone();
    let mut best_score = squareness(t, &moves, &k);
    let mut moved = false;
    let mut cur = t.clone();
    let mut steps = vec![Vec2::new(0.0, 0.0); t.pts.len()];
    for _ in 0..s.iterations {
        if best_score < SQUARE {
            break;
        }
        for (i, step) in steps.iter_mut().enumerate() {
            *step = if moves[i] {
                let (a, b, c) = neighbours(&cur, i);
                motion(a, b, c, &k)
            } else {
                Vec2::new(0.0, 0.0)
            };
        }
        for (p, m) in cur.pts.iter_mut().zip(&steps) {
            *p = Vec2::new(p.x + m.x, p.y + m.y);
        }
        let score = squareness(&cur, &moves, &k);
        if score < best_score {
            best = cur.clone();
            best_score = score;
            moved = true;
        }
    }
    (best, moved)
}
