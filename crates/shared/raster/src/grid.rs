//! The grid a raster made from points or lines is worked out on (docs/adr/0232
//! §3): a box's grid, its cell size given or chosen and its edges on the
//! cell size's multiples, or another raster's grid as it is. A cell's value
//! is taken at its centre.

use kentos_geometry_core::vec2::Vec2;

use crate::job::{MOST_CELLS, MOST_WIDTH};

/// A grid: the affine (ADR 0204 §2: x = x₀ + a·u + b·v, y = y₀ + c·u + d·v,
/// `[x₀, a, b, y₀, c, d]`), its width and height in cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
}

impl Grid {
    /// Cell (i, j)'s centre.
    pub fn center(&self, i: u32, j: u32) -> Vec2 {
        let [x0, a, b, y0, c, d] = self.affine;
        let (u, v) = (f64::from(i) + 0.5, f64::from(j) + 0.5);
        Vec2::new(x0 + a * u + b * v, y0 + c * u + d * v)
    }

    /// A raster's grid as it is; its size within the limits.
    pub fn of(affine: [f64; 6], width: u32, height: u32) -> Result<Grid, String> {
        if affine.iter().any(|v| !v.is_finite()) || width == 0 || height == 0 {
            return Err("Izgara rasterinin yeri ya da boyu okunamadı.".into());
        }
        let [_, a, b, _, c, d] = affine;
        if a * d - b * c == 0.0 {
            return Err("Izgara rasterinin pikselleri alansız (afin tersinmez).".into());
        }
        limits(width, height)?;
        Ok(Grid {
            affine,
            width,
            height,
        })
    }

    /// The grid of a box (least x, least y, largest x, largest y): cell size
    /// `cell` when above 0, else the box's shorter side / 250 (its longer
    /// when the shorter is 0) rounded down to a nice size; its edges on the
    /// cell size's multiples.
    pub fn of_box(b: [f64; 4], cell: f64) -> Result<Grid, String> {
        let [minx, miny, maxx, maxy] = b;
        if b.iter().any(|v| !v.is_finite()) {
            return Err("Kutunun köşeleri okunamadı.".into());
        }
        let s = if cell > 0.0 {
            cell
        } else {
            let (w, h) = (maxx - minx, maxy - miny);
            let short = if w > 0.0 && h > 0.0 {
                w.min(h)
            } else {
                w.max(h)
            };
            if !(short > 0.0) {
                return Err(
                    "Noktalar tek bir yerde: hücre boyu kendiliğinden seçilemez. Hücre boyunu yazın.".into(),
                );
            }
            nice_cell(short / 250.0)
        };
        if !(s > 0.0) || !s.is_finite() {
            return Err("Hücre boyu 0'dan büyük olmalı.".into());
        }
        let x0 = (minx / s).floor() * s;
        let y1 = (maxy / s).ceil() * s;
        let width = ((maxx - x0) / s).ceil().max(1.0);
        let height = ((y1 - miny) / s).ceil().max(1.0);
        if width > f64::from(MOST_WIDTH) || height > f64::from(u32::MAX) {
            return Err(too_big(width, height));
        }
        let (width, height) = (width as u32, height as u32);
        limits(width, height)?;
        Ok(Grid {
            affine: [x0, s, 0.0, y1, 0.0, -s],
            width,
            height,
        })
    }

    /// The cell's size along its rows (the first axis's length).
    pub fn cell(&self) -> f64 {
        let [_, a, _, _, c, _] = self.affine;
        (a * a + c * c).sqrt()
    }
}

fn too_big(width: f64, height: f64) -> String {
    format!(
        "Izgara çok büyük ({width} × {height} hücre): genişlik en çok {MOST_WIDTH}, hücre sayısı en çok 2³¹. Hücre boyunu büyütün."
    )
}

fn limits(width: u32, height: u32) -> Result<(), String> {
    if width > MOST_WIDTH || u64::from(width) * u64::from(height) > MOST_CELLS {
        return Err(too_big(f64::from(width), f64::from(height)));
    }
    Ok(())
}

/// The largest of 1, 2, 2.5 and 5 times a power of ten not above `s`.
pub fn nice_cell(s: f64) -> f64 {
    // Ten to the |k|-th by products (exact up to 10²²).
    let ten = |k: i32| (0..k.unsigned_abs()).fold(1.0f64, |p, _| p * 10.0);
    let value = |m: f64, k: i32| if k >= 0 { m * ten(k) } else { m / ten(k) };
    let k = libm::floor(libm::log10(s)) as i32;
    for kk in [k + 1, k, k - 1] {
        for m in [5.0, 2.5, 2.0, 1.0] {
            let c = value(m, kk);
            if c <= s {
                return c;
            }
        }
    }
    value(1.0, k - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_sizes() {
        assert_eq!(nice_cell(3.7), 2.5);
        assert_eq!(nice_cell(2.5), 2.5);
        assert_eq!(nice_cell(0.83), 0.5);
        assert_eq!(nice_cell(12.0), 10.0);
        assert_eq!(nice_cell(0.24), 0.2);
        assert_eq!(nice_cell(1.0), 1.0);
        assert_eq!(nice_cell(999.0), 500.0);
    }

    #[test]
    fn a_box_on_the_cells_multiples() {
        let g =
            Grid::of_box([500_000.123, 4_420_000.9, 500_100.0, 4_420_050.0], 0.0).expect("a grid");
        // Shorter side 49.1 m: / 250 = 0.196 → 0.1; the edges on its multiples.
        let [x0, a, b, y1, c, d] = g.affine;
        assert!((x0 - 500_000.1).abs() < 1e-9 && (y1 - 4_420_050.0).abs() < 1e-9);
        assert_eq!([a, b, c, d], [0.1, 0.0, 0.0, -0.1]);
        assert_eq!(g.width, ((500_100.0 - x0) / 0.1f64).ceil() as u32);
        let one = Grid::of_box([5.0, 5.0, 5.0, 5.0], 0.0);
        assert!(one.is_err());
        let one = Grid::of_box([5.0, 5.0, 5.0, 5.0], 2.0).expect("given size");
        assert_eq!((one.width, one.height), (1, 1));
        assert!(Grid::of_box([0.0, 0.0, 1e6, 1e6], 0.001).is_err());
    }
}
