//! Grafik's shapes (docs/adr/0213 §2.5): a pie's slices from the north
//! clockwise, bars side by side, a stacked bar, as rings in the world
//! around the object's place. Arcs in steps of at most 5°, their points by
//! turning the first one step by step (a sine and a cosine per slice, not
//! per point: 10 000 pies are 720 000 points).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::jsmath::{PI, cos, sin};

use super::model::ChartKind;
use super::place::positive;

/// The longest step of a pie's arc, radians.
const ARC_STEP: f64 = 5.0 * PI / 180.0;

/// `n + 1` points of the arc of radius `r` round `c` from `from` over `sweep` (radians, from the north
/// clockwise), equally spaced: the first by its sine and cosine, each next turned by the step's.
fn arc(out: &mut Vec<Vec2>, c: Vec2, r: f64, from: f64, sweep: f64, n: usize) {
    let step = sweep / n as f64;
    let (sd, cd) = (sin(step), cos(step));
    let (mut s, mut k) = (sin(from), cos(from));
    for _ in 0..=n {
        out.push(Vec2::new(c.x + r * s, c.y + r * k));
        (s, k) = (s * cd + k * sd, k * cd - s * sd);
    }
}

/// One shape of a chart: its ring and the value's place in the list.
pub struct Piece {
    pub field: usize,
    pub ring: Vec<Vec2>,
}

/// A pie of diameter `d` (world) at `c`: a slice per positive value, in
/// order; a single value is a whole circle.
pub fn pie(c: Vec2, d: f64, values: &[f64]) -> Vec<Piece> {
    let total: f64 = values.iter().filter(|v| **v > 0.0).sum();
    if !positive(total) || !positive(d) {
        return Vec::new();
    }
    let r = d / 2.0;
    let shown = values.iter().filter(|v| **v > 0.0).count();
    let mut out = Vec::new();
    let mut from = 0.0;
    for (field, &v) in values.iter().enumerate() {
        if !positive(v) {
            continue;
        }
        let sweep = 2.0 * PI * v / total;
        if shown == 1 {
            let n = (2.0 * PI / ARC_STEP).ceil() as usize;
            let mut ring = Vec::with_capacity(n + 1);
            arc(&mut ring, c, r, 0.0, 2.0 * PI, n);
            // The closing point is the first.
            ring.pop();
            out.push(Piece { field, ring });
            break;
        }
        let n = ((sweep / ARC_STEP).ceil() as usize).max(1);
        let mut ring = Vec::with_capacity(n + 2);
        ring.push(c);
        arc(&mut ring, c, r, from, sweep, n);
        out.push(Piece { field, ring });
        from += sweep;
    }
    out
}

/// Bars (`Bar`) or one stacked bar (`Stacked`): `h` is the height of
/// `max_value`, `w` a bar's width (both world); a negative value goes down.
pub fn bars(
    kind: ChartKind,
    c: Vec2,
    h: f64,
    w: f64,
    max_value: f64,
    values: &[f64],
) -> Vec<Piece> {
    if !positive(max_value) || !positive(w) {
        return Vec::new();
    }
    let rect = |x0: f64, x1: f64, y0: f64, y1: f64| {
        vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ]
    };
    let mut out = Vec::new();
    match kind {
        ChartKind::Stacked => {
            let mut y = c.y;
            for (field, &v) in values.iter().enumerate() {
                if !positive(v) {
                    continue;
                }
                let top = y + h * v / max_value;
                out.push(Piece {
                    field,
                    ring: rect(c.x - w / 2.0, c.x + w / 2.0, y, top),
                });
                y = top;
            }
        }
        _ => {
            let left = c.x - w * values.len() as f64 / 2.0;
            for (field, &v) in values.iter().enumerate() {
                if v == 0.0 || !v.is_finite() {
                    continue;
                }
                let x0 = left + w * field as f64;
                let top = c.y + h * v / max_value;
                let (y0, y1) = if top >= c.y { (c.y, top) } else { (top, c.y) };
                out.push(Piece {
                    field,
                    ring: rect(x0, x0 + w, y0, y1),
                });
            }
        }
    }
    out
}
