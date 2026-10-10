//! Isı haritası's picture (docs/adr/0213 §2.6), as QGIS's heat map renderer
//! makes it: the box in cells of `quality` pixels, every point cut to its
//! cell, the quartic kernel `(1 − (d/R)²)²` of radius `R` cells summed row
//! by row (a precomputed stamp, so the sums vectorise), the values over the
//! box's largest (dynamic) or a fixed maximum through the ramp in 1024 steps;
//! an empty cell clear. Rows run from the top of the box down, as a
//! picture's do.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::{js_max, js_min, js_round};

use super::model::Rgba;
use super::place::positive;
use super::thematic::ramp_color;

/// The most cells a picture has (about 4 million, a 2 700 × 1 500 box at full quality).
pub const MOST_CELLS: f64 = 4_194_304.0;
/// The longest side, cells.
pub const MOST_SIDE: f64 = 4096.0;
/// The ramp's steps.
pub const STEPS: usize = 1024;

/// The grid over a box: where its first cell is, its cells' side (metres), its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub min_x: f64,
    pub max_y: f64,
    pub cell: f64,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    /// The box in cells of `quality` pixels at `px_per_m`, made coarser when it would pass the limits.
    pub fn over(b: &Bounds, px_per_m: f64, quality: u32) -> Option<Grid> {
        let (w, h) = (b.max_x - b.min_x, b.max_y - b.min_y);
        if !positive(w) || !positive(h) || !positive(px_per_m) || !(w * h).is_finite() {
            return None;
        }
        let cell = js_max(
            js_max(f64::from(quality.max(1)) / px_per_m, w / MOST_SIDE),
            js_max(h / MOST_SIDE, (w * h / MOST_CELLS).sqrt()),
        );
        Some(Grid {
            min_x: b.min_x,
            max_y: b.max_y,
            cell,
            width: js_max((w / cell).ceil(), 1.0) as usize,
            height: js_max((h / cell).ceil(), 1.0) as usize,
        })
    }

    /// The point's cell (column, row), as QGIS cuts it (toward −∞).
    pub fn cell_of(&self, p: Vec2) -> (i64, i64) {
        (
            ((p.x - self.min_x) / self.cell).floor() as i64,
            ((self.max_y - p.y) / self.cell).floor() as i64,
        )
    }
}

/// The kernel's stamp: `(2R + 1)²` values, rows from `−R`.
pub fn stamp(r: i64) -> Vec<f64> {
    let side = (2 * r + 1) as usize;
    let mut out = vec![0.0; side * side];
    let r2 = (r * r) as f64;
    for dj in -r..=r {
        for di in -r..=r {
            let d2 = (di * di + dj * dj) as f64;
            if d2 <= r2 {
                let k = 1.0 - d2 / r2;
                out[((dj + r) as usize) * side + (di + r) as usize] = k * k;
            }
        }
    }
    out
}

/// The heat values of weighted points over the grid, `R` cells round each
/// (a point beyond `R` cells of the box adds nothing).
pub fn values(grid: &Grid, points: &[(Vec2, f64)], r: i64) -> Vec<f64> {
    let (w, h) = (grid.width as i64, grid.height as i64);
    let mut out = vec![0.0f64; grid.width * grid.height];
    let r = r.max(1);
    let k = stamp(r);
    let side = (2 * r + 1) as usize;
    for &(p, weight) in points {
        if !positive(weight) || !weight.is_finite() {
            continue;
        }
        let (i, j) = grid.cell_of(p);
        if i + r < 0 || i - r >= w || j + r < 0 || j - r >= h {
            continue;
        }
        let (x0, x1) = ((i - r).max(0), (i + r).min(w - 1));
        for dj in -r..=r {
            let row = j + dj;
            if row < 0 || row >= h {
                continue;
            }
            let from = (row * w + x0) as usize;
            let to = (row * w + x1) as usize + 1;
            let kfrom = ((dj + r) as usize) * side + (x0 - (i - r)) as usize;
            let cells = &mut out[from..to];
            let ks = &k[kfrom..kfrom + cells.len()];
            for (v, s) in cells.iter_mut().zip(ks) {
                *v += weight * s;
            }
        }
    }
    out
}

/// The ramp's 1024 colours with the picture's opacity.
pub fn table(ramp: &[Rgba], opacity: f64) -> Vec<Rgba> {
    let o = opacity.clamp(0.0, 1.0);
    (0..STEPS)
        .map(|k| {
            let c = ramp_color(ramp, k as f64 / (STEPS - 1) as f64);
            [c[0], c[1], c[2], js_round(f64::from(c[3]) * o) as u8]
        })
        .collect()
}

/// The picture's pixels (RGBA, straight alpha, rows from the top): a value's
/// share of `max` (at most 1) rounded to the table's step, an empty cell clear.
pub fn colors(values: &[f64], max: f64, table: &[Rgba]) -> Vec<u8> {
    let mut out = vec![0u8; values.len() * 4];
    if !positive(max) || table.is_empty() {
        return out;
    }
    let top = (table.len() - 1) as f64;
    for (px, &v) in out.chunks_exact_mut(4).zip(values) {
        if !positive(v) {
            continue;
        }
        let k = js_round(top * js_min(v / max, 1.0)) as usize;
        px.copy_from_slice(&table[k.min(table.len() - 1)]);
    }
    out
}

/// The values' largest (0 when there are none).
pub fn largest(values: &[f64]) -> f64 {
    values.iter().copied().fold(0.0, js_max)
}
