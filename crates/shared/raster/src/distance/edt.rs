//! Uzaklık yüzeyi's exact distance transform (docs/adr/0236 §3; Felzenszwalb
//! and Huttenlocher 2012): every cell's nearest source cell among the cell
//! centres, by (Δi·sx)² + (Δj·sy)², ties to the smaller column and then the
//! smaller row. First each column's nearest source row for every row (two
//! sweeps), then each row's lower envelope of the columns' parabolas; on
//! square cells the envelope's breakpoints are compared as exact fractions,
//! on rectangular ones with float64 weights.

use libm::sqrt;

use crate::par;

/// No source (a column's, or the whole grid's).
pub const NONE: u32 = u32::MAX;

/// Each cell's nearest source cell (`NONE` when the grid has none), sources
/// where `source` is true; `sx` and `sy` the column and row steps' lengths.
pub fn nearest(
    width: u32,
    height: u32,
    source: &[bool],
    (sx, sy): (f64, f64),
    threads: usize,
) -> Vec<u32> {
    let (w, h) = (width as usize, height as usize);
    let column = column_rows(w, h, source, threads);
    let square = sx == sy;
    let (wx, wy) = (sx * sx, sy * sy);
    let mut out = vec![NONE; w * h];
    par::rows(threads, &mut out, w, &|first, chunk: &mut [u32]| {
        let mut env = Envelope::default();
        for (r, row) in chunk.chunks_mut(w).enumerate() {
            let j = first + r;
            if square {
                env.square(&column, w, j, row);
            } else {
                env.weighted(&column, w, j, (wx, wy), row);
            }
        }
    });
    out
}

/// The distance of cell (`i`, `j`) to cell `to` of a grid `width` wide:
/// √(x·x + y·y), x = Δi·sx, y = Δj·sy.
#[inline]
pub fn distance(i: usize, j: usize, to: u32, width: usize, (sx, sy): (f64, f64)) -> f64 {
    let (ti, tj) = (to as usize % width, to as usize / width);
    let x = (i as f64 - ti as f64) * sx;
    let y = (j as f64 - tj as f64) * sy;
    sqrt(x * x + y * y)
}

/// Every cell's nearest source row in its column (`NONE`: the column has
/// none), ties to the smaller row; columns in pieces on the threads, each
/// piece's rows kept together.
struct Columns {
    /// The pieces' first columns and widths.
    pieces: Vec<(usize, usize)>,
    /// A piece's rows, `width` × height.
    rows: Vec<Vec<u32>>,
}

impl Columns {
    #[inline]
    fn at(&self, i: usize, j: usize, per: usize) -> u32 {
        let p = i / per;
        let (c0, cw) = self.pieces[p];
        self.rows[p][j * cw + (i - c0)]
    }
}

fn column_rows(w: usize, h: usize, source: &[bool], threads: usize) -> (Columns, usize) {
    let n = threads.clamp(1, w.max(1));
    let per = w.div_ceil(n).max(1);
    let pieces: Vec<(usize, usize)> = (0..w)
        .step_by(per)
        .map(|c0| (c0, per.min(w - c0)))
        .collect();
    let mut rows: Vec<Vec<u32>> = pieces.iter().map(|&(_, cw)| vec![NONE; cw * h]).collect();
    let mut work: Vec<(usize, usize, &mut Vec<u32>)> = pieces
        .iter()
        .zip(rows.iter_mut())
        .map(|(&(c0, cw), r)| (c0, cw, r))
        .collect();
    par::each_mut(threads, &mut work, &|_, (c0, cw, out)| {
        let (c0, cw) = (*c0, *cw);
        // Down: the last source at or above; up: the nearer of it and the next at or below.
        let mut last = vec![NONE; cw];
        for j in 0..h {
            let row = &source[j * w + c0..j * w + c0 + cw];
            let o = &mut out[j * cw..(j + 1) * cw];
            for x in 0..cw {
                if row[x] {
                    last[x] = j as u32;
                }
                o[x] = last[x];
            }
        }
        let mut next = vec![NONE; cw];
        for j in (0..h).rev() {
            let row = &source[j * w + c0..j * w + c0 + cw];
            let o = &mut out[j * cw..(j + 1) * cw];
            for x in 0..cw {
                if row[x] {
                    next[x] = j as u32;
                }
                let (up, down) = (o[x], next[x]);
                o[x] = match (up, down) {
                    (NONE, d) => d,
                    (u, NONE) => u,
                    (u, d) => {
                        if j as u32 - u <= d - j as u32 {
                            u
                        } else {
                            d
                        }
                    }
                };
            }
        }
    });
    (Columns { pieces, rows }, per)
}

/// A row's lower envelope: its parabolas' columns and where each begins.
#[derive(Default)]
struct Envelope {
    v: Vec<usize>,
    /// Square cells: each breakpoint as a fraction (numerator, denominator > 0).
    zq: Vec<(i64, i64)>,
    /// Rectangular cells: each breakpoint.
    zf: Vec<f64>,
}

impl Envelope {
    /// Row `j` on square cells: f(i′) = (j − r(i′))², value at q f(i′) + (q − i′)².
    fn square(&mut self, (cols, per): &(Columns, usize), w: usize, j: usize, out: &mut [u32]) {
        self.v.clear();
        self.zq.clear();
        let f = |r: u32| -> i64 {
            let g = j as i64 - i64::from(r);
            g * g
        };
        // The breakpoint between parabolas p < q: ((f_q + q²) − (f_p + p²)) / (2(q − p)).
        let cut = |p: usize, fp: i64, q: usize, fq: i64| -> (i64, i64) {
            let (p, q) = (p as i64, q as i64);
            ((fq + q * q) - (fp + p * p), 2 * (q - p))
        };
        // a ≤ b for fractions with positive denominators.
        let le = |a: (i64, i64), b: (i64, i64)| {
            i128::from(a.0) * i128::from(b.1) <= i128::from(b.0) * i128::from(a.1)
        };
        let mut fs: Vec<i64> = Vec::new();
        for q in 0..w {
            let r = cols.at(q, j, *per);
            if r == NONE {
                continue;
            }
            let fq = f(r);
            while let Some(&p) = self.v.last() {
                let k = self.v.len() - 1;
                let s = cut(p, fs[k], q, fq);
                // The first parabola reaches back to −∞.
                if k > 0 && le(s, self.zq[k]) {
                    self.v.pop();
                    self.zq.pop();
                    fs.pop();
                } else {
                    self.zq.push(s);
                    break;
                }
            }
            if self.v.is_empty() {
                self.zq.push((0, 1));
            }
            self.v.push(q);
            fs.push(fq);
        }
        if self.v.is_empty() {
            out.fill(NONE);
            return;
        }
        let mut k = 0;
        for (q, o) in out.iter_mut().enumerate() {
            // The next breakpoint below q (ties stay with the smaller column).
            while k + 1 < self.v.len()
                && i128::from(self.zq[k + 1].0) < q as i128 * i128::from(self.zq[k + 1].1)
            {
                k += 1;
            }
            let p = self.v[k];
            *o = (cols.at(p, j, *per) as usize * w + p) as u32;
        }
    }

    /// Row `j` on rectangular cells: f(i′) = wy·(j − r(i′))², value at q f(i′) + wx·(q − i′)².
    fn weighted(
        &mut self,
        (cols, per): &(Columns, usize),
        w: usize,
        j: usize,
        (wx, wy): (f64, f64),
        out: &mut [u32],
    ) {
        self.v.clear();
        self.zf.clear();
        let f = |r: u32| -> f64 {
            let g = j as f64 - f64::from(r);
            wy * (g * g)
        };
        let cut = |p: usize, fp: f64, q: usize, fq: f64| -> f64 {
            let (p, q) = (p as f64, q as f64);
            ((fq + wx * (q * q)) - (fp + wx * (p * p))) / (2.0 * wx * (q - p))
        };
        let mut fs: Vec<f64> = Vec::new();
        for q in 0..w {
            let r = cols.at(q, j, *per);
            if r == NONE {
                continue;
            }
            let fq = f(r);
            while let Some(&p) = self.v.last() {
                let k = self.v.len() - 1;
                let s = cut(p, fs[k], q, fq);
                if k > 0 && s <= self.zf[k] {
                    self.v.pop();
                    self.zf.pop();
                    fs.pop();
                } else {
                    self.zf.push(s);
                    break;
                }
            }
            if self.v.is_empty() {
                self.zf.push(f64::NEG_INFINITY);
            }
            self.v.push(q);
            fs.push(fq);
        }
        if self.v.is_empty() {
            out.fill(NONE);
            return;
        }
        let mut k = 0;
        for (q, o) in out.iter_mut().enumerate() {
            while k + 1 < self.v.len() && self.zf[k + 1] < q as f64 {
                k += 1;
            }
            let p = self.v[k];
            *o = (cols.at(p, j, *per) as usize * w + p) as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The nearest by brute force: least (d², column, row), d² as an exact sum on whole-number steps.
    fn brute(w: usize, h: usize, source: &[bool], (sx2, sy2): (i64, i64)) -> Vec<u32> {
        let sources: Vec<(usize, usize)> = (0..w * h)
            .filter(|&k| source[k])
            .map(|k| (k % w, k / w))
            .collect();
        (0..w * h)
            .map(|k| {
                let (i, j) = (k % w, k / w);
                sources
                    .iter()
                    .map(|&(a, b)| {
                        let (di, dj) = (i as i64 - a as i64, j as i64 - b as i64);
                        (sx2 * di * di + sy2 * dj * dj, a, b)
                    })
                    .min()
                    .map_or(NONE, |(_, a, b)| (b * w + a) as u32)
            })
            .collect()
    }

    fn sources(w: usize, h: usize, seed: u32, every: u32) -> Vec<bool> {
        (0..w * h)
            .map(|k| {
                let mut x = (k as u32).wrapping_mul(0x9e37_79b9) ^ seed.wrapping_mul(0x85eb_ca6b);
                x ^= x >> 15;
                x = x.wrapping_mul(0x2c1b_3c6d);
                x ^= x >> 12;
                x.is_multiple_of(every)
            })
            .collect()
    }

    #[test]
    fn the_nearest_is_the_brute_forces_ties_included() {
        for (w, h, seed, every) in [
            (41, 37, 1, 60),
            (64, 9, 2, 30),
            (7, 53, 3, 25),
            (33, 33, 4, 400),
        ] {
            let s = sources(w, h, seed, every);
            for threads in [1, 3] {
                assert_eq!(
                    nearest(w as u32, h as u32, &s, (10.0, 10.0), threads),
                    brute(w, h, &s, (1, 1))
                );
                assert_eq!(
                    nearest(w as u32, h as u32, &s, (10.0, 6.0), threads),
                    brute(w, h, &s, (100, 36))
                );
            }
        }
        // Symmetric sources: every tie the smaller column, then the smaller row.
        let mut s = vec![false; 11 * 11];
        for k in [0, 10, 110, 120, 60] {
            s[k] = true;
        }
        assert_eq!(
            nearest(11, 11, &s, (2.0, 2.0), 2),
            brute(11, 11, &s, (1, 1))
        );
    }

    #[test]
    fn no_source_is_none_everywhere() {
        let s = vec![false; 12];
        assert!(nearest(4, 3, &s, (1.0, 1.0), 2).iter().all(|&n| n == NONE));
    }
}
