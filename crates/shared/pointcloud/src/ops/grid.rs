//! The height grid of Zemin süzgeci and Yüksekliğe göre sınıfla (docs/adr/0207
//! §7): cells of `cell` metres from the cloud's least corner, each the least
//! height of its points or empty; empty cells filled by a wave of 8
//! neighbours; values between cell centres bilinear, slopes by the centres'
//! differences; disc-shaped erosion and dilation with the grid's outside
//! left out. Everything is decided in a fixed order with plain double
//! arithmetic, so the independent reference (scripts/fixtures/pointcloud_ops_cases.py)
//! gives the same bits.

/// A grid of heights; NaN is an empty cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub x0: f64,
    pub y0: f64,
    pub cell: f64,
    pub cols: usize,
    pub rows: usize,
    pub v: Vec<f64>,
}

/// The most cells a grid may have.
pub const MAX_CELLS: usize = 200_000_000;

impl Grid {
    /// An empty grid over the plan `[x₁, y₁, x₂, y₂]`: ⌊(x₂ − x₁) / cell⌋ + 1 columns, rows likewise.
    pub fn over(plan: [f64; 4], cell: f64) -> Option<Grid> {
        if !(cell > 0.0) || !plan.iter().all(|v| v.is_finite()) {
            return None;
        }
        let cols = ((plan[2] - plan[0]) / cell).floor() + 1.0;
        let rows = ((plan[3] - plan[1]) / cell).floor() + 1.0;
        if !(cols >= 1.0 && rows >= 1.0 && cols * rows <= MAX_CELLS as f64) {
            return None;
        }
        let (cols, rows) = (cols as usize, rows as usize);
        Some(Grid {
            x0: plan[0],
            y0: plan[1],
            cell,
            cols,
            rows,
            v: vec![f64::NAN; cols * rows],
        })
    }

    /// The column and row of a place, clamped into the grid.
    #[inline]
    pub fn cell_of(&self, x: f64, y: f64) -> (usize, usize) {
        let c = ((x - self.x0) / self.cell).floor();
        let r = ((y - self.y0) / self.cell).floor();
        let c = if c >= 0.0 {
            (c as usize).min(self.cols - 1)
        } else {
            0
        };
        let r = if r >= 0.0 {
            (r as usize).min(self.rows - 1)
        } else {
            0
        };
        (c, r)
    }

    /// Keeps `z` in the cell of (`x`, `y`) when it is the least there.
    #[inline]
    pub fn put_min(&mut self, x: f64, y: f64, z: f64) {
        let (c, r) = self.cell_of(x, y);
        let k = r * self.cols + c;
        let old = self.v[k];
        if old.is_nan() || z < old {
            self.v[k] = z;
        }
    }

    /// Whether every cell is empty.
    pub fn is_empty(&self) -> bool {
        self.v.iter().all(|v| v.is_nan())
    }

    /// Fills the empty cells: wave by wave, every empty cell next to a filled
    /// one (of the 8 around it, filled before the wave) takes the least of
    /// them, until none is left (or none was filled at all).
    pub fn fill(&mut self) {
        fill(&mut self.v, self.cols, self.rows);
    }

    /// The height at (`x`, `y`) between the cell centres, bilinear; at the
    /// grid's edge the edge's centres' (the grid filled).
    pub fn bilinear(&self, x: f64, y: f64) -> f64 {
        let (c0, c1, tx) = axis((x - self.x0) / self.cell - 0.5, self.cols);
        let (r0, r1, ty) = axis((y - self.y0) / self.cell - 0.5, self.rows);
        let v = |c: usize, r: usize| self.v[r * self.cols + c];
        let a = v(c0, r0) + (v(c1, r0) - v(c0, r0)) * tx;
        let b = v(c0, r1) + (v(c1, r1) - v(c0, r1)) * tx;
        a + (b - a) * ty
    }

    /// The slope at the cell of (`x`, `y`): its neighbours' differences over
    /// their distance, the edge's own cell standing for one outside.
    pub fn slope(&self, x: f64, y: f64) -> f64 {
        let (c, r) = self.cell_of(x, y);
        let v = |c: usize, r: usize| self.v[r * self.cols + c];
        let (cl, cr) = (c.saturating_sub(1), (c + 1).min(self.cols - 1));
        let (rb, rt) = (r.saturating_sub(1), (r + 1).min(self.rows - 1));
        let gx = if cr > cl {
            (v(cr, r) - v(cl, r)) / ((cr - cl) as f64 * self.cell)
        } else {
            0.0
        };
        let gy = if rt > rb {
            (v(c, rt) - v(c, rb)) / ((rt - rb) as f64 * self.cell)
        } else {
            0.0
        };
        (gx * gx + gy * gy).sqrt()
    }
}

/// A place along an axis of `n` centres (in centre steps from the first): the
/// two centres and the share of the second.
#[inline]
fn axis(f: f64, n: usize) -> (usize, usize, f64) {
    if !(f > 0.0) {
        (0, 0, 0.0)
    } else if f >= (n - 1) as f64 {
        (n - 1, n - 1, 0.0)
    } else {
        let i = f.floor() as usize;
        (i, i + 1, f - i as f64)
    }
}

/// The wave fill of `v` (`cols` × `rows`, NaN empty).
pub fn fill(v: &mut [f64], cols: usize, rows: usize) {
    if v.iter().all(|x| x.is_nan()) {
        return;
    }
    let mut front: Vec<usize> = (0..v.len()).filter(|&k| v[k].is_nan()).collect();
    let mut fresh: Vec<(usize, f64)> = Vec::new();
    while !front.is_empty() {
        fresh.clear();
        for &k in &front {
            let (c, r) = (k % cols, k / cols);
            let mut least = f64::NAN;
            for dr in -1i64..=1 {
                for dc in -1i64..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let (cc, rr) = (c as i64 + dc, r as i64 + dr);
                    if cc < 0 || rr < 0 || cc >= cols as i64 || rr >= rows as i64 {
                        continue;
                    }
                    let n = v[rr as usize * cols + cc as usize];
                    if !n.is_nan() && (least.is_nan() || n < least) {
                        least = n;
                    }
                }
            }
            if !least.is_nan() {
                fresh.push((k, least));
            }
        }
        if fresh.is_empty() {
            break;
        }
        for &(k, x) in &fresh {
            v[k] = x;
        }
        front.retain(|&k| v[k].is_nan());
    }
}

/// The half widths of a disc of radius `r` cells by row: the largest w with w² + dy² ≤ r².
fn disc(r: usize) -> Vec<usize> {
    let r2 = (r * r) as i64;
    (0..=r)
        .map(|dy| {
            let mut w = 0i64;
            while (w + 1) * (w + 1) + (dy * dy) as i64 <= r2 {
                w += 1;
            }
            w as usize
        })
        .collect()
}

/// Each row's least (or most) over a window of `w` cells each side, the
/// grid's outside left out: van Herk and Gil–Werman's blocks.
fn rows_window(v: &[f64], cols: usize, rows: usize, w: usize, most: bool, out: &mut Vec<f64>) {
    out.clear();
    out.resize(v.len(), 0.0);
    let pick = |a: f64, b: f64| if most { a.max(b) } else { a.min(b) };
    let none = if most {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let k = 2 * w + 1;
    // A padded row: w of nothing each side.
    let len = cols + 2 * w;
    let blocks = len.div_ceil(k);
    let padded_len = blocks * k;
    let mut row = vec![none; padded_len];
    let mut left = vec![none; padded_len];
    let mut right = vec![none; padded_len];
    for r in 0..rows {
        row.iter_mut().for_each(|x| *x = none);
        row[w..w + cols].copy_from_slice(&v[r * cols..(r + 1) * cols]);
        for b in 0..blocks {
            let s = b * k;
            left[s] = row[s];
            for i in s + 1..s + k {
                left[i] = pick(left[i - 1], row[i]);
            }
            right[s + k - 1] = row[s + k - 1];
            for i in (s..s + k - 1).rev() {
                right[i] = pick(right[i + 1], row[i]);
            }
        }
        for c in 0..cols {
            // The window over padded positions c .. c + 2w.
            let (a, b) = (c, c + 2 * w);
            out[r * cols + c] = pick(right[a], left[b]);
        }
    }
}

/// Erosion (`most` false: the least) or dilation (the most) of `v` by a disc of radius `r` cells.
pub fn morph(v: &[f64], cols: usize, rows: usize, r: usize, most: bool) -> Vec<f64> {
    let widths = disc(r);
    let mut by_width: Vec<Option<Vec<f64>>> = vec![None; r + 1];
    for &w in &widths {
        if by_width[w].is_none() {
            let mut out = Vec::new();
            rows_window(v, cols, rows, w, most, &mut out);
            by_width[w] = Some(out);
        }
    }
    let none = if most {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut out = vec![none; v.len()];
    for r0 in 0..rows {
        for (dy, &w) in widths.iter().enumerate() {
            let Some(rw) = &by_width[w] else {
                continue;
            };
            for rr in [r0 as i64 - dy as i64, r0 as i64 + dy as i64] {
                if rr < 0 || rr >= rows as i64 || (dy == 0 && rr != r0 as i64) {
                    continue;
                }
                let src = &rw[rr as usize * cols..(rr as usize + 1) * cols];
                let dst = &mut out[r0 * cols..(r0 + 1) * cols];
                for c in 0..cols {
                    dst[c] = if most {
                        dst[c].max(src[c])
                    } else {
                        dst[c].min(src[c])
                    };
                }
                if dy == 0 {
                    break;
                }
            }
        }
    }
    out
}

/// A progressive opening (docs/adr/0207 §7, SMRF's): for r = 1 … ⌈window / cell⌉
/// the surface opened by a disc of r cells; a cell whose surface stands above
/// the opened by more than `slope` · r · cell is flagged; the surface becomes
/// the opened. The flags.
pub fn progressive(
    v: &[f64],
    cols: usize,
    rows: usize,
    cell: f64,
    window: f64,
    slope: f64,
) -> Vec<bool> {
    let mut surface = v.to_vec();
    let mut flags = vec![false; v.len()];
    let steps = (window / cell).ceil();
    let steps = if steps.is_finite() && steps >= 1.0 {
        steps as usize
    } else {
        0
    };
    for r in 1..=steps {
        let eroded = morph(&surface, cols, rows, r, false);
        let opened = morph(&eroded, cols, rows, r, true);
        let limit = slope * r as f64 * cell;
        for k in 0..surface.len() {
            if surface[k] - opened[k] > limit {
                flags[k] = true;
            }
        }
        surface = opened;
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wave_fills_from_the_least_neighbour() {
        let n = f64::NAN;
        let mut v = vec![5.0, n, n, n, n, n, n, n, 1.0];
        fill(&mut v, 3, 3);
        // Wave 1: (1,0) and (0,1) from 5, (1,1), (2,1) and (1,2) from 1; wave 2: (2,0) and (0,2) from 1.
        assert_eq!(v, vec![5.0, 5.0, 1.0, 5.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
        let mut empty = vec![n; 4];
        fill(&mut empty, 2, 2);
        assert!(empty.iter().all(|x| x.is_nan()));
    }

    #[test]
    fn a_disc_erodes_and_dilates() {
        assert_eq!(disc(1), vec![1, 0]);
        assert_eq!(disc(2), vec![2, 1, 0]);
        let mut v = vec![0.0; 25];
        v[12] = 9.0;
        let e = morph(&v, 5, 5, 1, false);
        assert!(e.iter().all(|&x| x == 0.0));
        let d = morph(&v, 5, 5, 1, true);
        assert_eq!(d.iter().filter(|&&x| x == 9.0).count(), 5);
        // A spike stands out of its opening.
        let flags = progressive(&v, 5, 5, 1.0, 2.0, 0.5);
        assert!(flags[12] && flags.iter().filter(|&&f| f).count() == 1);
    }

    #[test]
    fn bilinear_and_slope() {
        let mut g = Grid::over([0.0, 0.0, 2.0, 1.0], 1.0).unwrap();
        // 3 × 2 cells, a plane rising 1 a metre eastwards.
        g.v = vec![0.5, 1.5, 2.5, 0.5, 1.5, 2.5];
        assert_eq!(g.bilinear(1.0, 0.7), 1.0);
        assert_eq!(g.bilinear(-5.0, 0.0), 0.5);
        assert_eq!(g.slope(1.2, 0.2), 1.0);
    }
}
