//! Akış birikimi (docs/adr/0235 §5): A(c) = w(c) + Σ pay(n → c)·A(n), the
//! neighbours in their order, a cell once all its donors are known (Kahn's
//! order, level by level: a large level's cells on the threads, each piece
//! counting down its receivers' donors). The sum's order is the neighbours'
//! whatever the level's order, so every thread count writes the same values.
//!
//! - D8: all of a cell to its direction.
//! - Çoklu yön: L·tᵖ / Σ L·tᵖ over the lower neighbours (Quinn's contour
//!   lengths, a fixed exponent or Qin's adaptive 8.9·min(e, 1) + 1.1); a
//!   cell without a lower neighbour all to its direction.
//! - D∞: Tarboton's facets, r/α of the share to the diagonal neighbour.

use std::sync::atomic::{AtomicU8, Ordering};

use libm::{atan2, exp, log, sqrt};

use super::surface::{NOFLOW, Surface, back};
use crate::par;

/// Quinn et al.'s (1991) effective contour lengths, square and diagonal.
const CONTOUR: [f64; 8] = [0.5, 0.354, 0.5, 0.354, 0.5, 0.354, 0.5, 0.354];
/// D∞'s facets: the square neighbour e₁ and the diagonal one e₂, from east–north-east counter-clockwise.
const FACETS: [(usize, usize); 8] = [
    (0, 7),
    (6, 7),
    (6, 5),
    (4, 5),
    (4, 3),
    (2, 3),
    (2, 1),
    (0, 1),
];
/// The square step between each facet's two neighbours.
const BETWEEN: [usize; 8] = [6, 0, 4, 6, 2, 4, 0, 2];
/// A level this large is worked out on the threads.
const PARALLEL_LEVEL: usize = 4096;
/// Cells a step takes at most.
const STEP_CELLS: usize = 1 << 21;

/// How flow leaves a cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Method {
    D8,
    /// The exponent; 0: Qin's adaptive one.
    Mfd(f64),
    Dinf,
}

/// The routing worked out ahead: Çoklu yön's sums and steepest neighbours, D∞'s facets and shares.
pub struct Routing {
    method: Method,
    /// Çoklu yön: Σ L·tᵖ of each cell's lower neighbours; NaN without one.
    total: Vec<f64>,
    /// Çoklu yön (adaptive): each cell's steepest lower neighbour (its slope is e).
    steep: Vec<u8>,
    /// D∞: each cell's facet (8: none) and the diagonal neighbour's share r/α.
    facet: Vec<u8>,
    ratio: Vec<f64>,
}

/// tᵖ as exp(p·ln t) (t itself for p = 1): within a few units of the 16th digit of libm's pow, at half its time.
#[inline(always)]
fn power(t: f64, p: f64) -> f64 {
    if p == 1.0 { t } else { exp(p * log(t)) }
}

/// The slope from cell `k` (row `j`) to its neighbour `m` at `q`.
#[inline(always)]
fn slope(s: &Surface, j: usize, k: usize, m: usize, q: usize) -> f64 {
    (s.z[k] - s.z[m]) / s.row(j).step[q]
}

impl Routing {
    pub fn new(s: &Surface, method: Method, threads: usize) -> Routing {
        let w = s.width as usize;
        let n = s.len();
        let mut r = Routing {
            method,
            total: Vec::new(),
            steep: Vec::new(),
            facet: Vec::new(),
            ratio: Vec::new(),
        };
        let rows: Vec<usize> = (0..s.height as usize).collect();
        match method {
            Method::D8 => {}
            Method::Mfd(p) => {
                let got = par::map(threads, &rows, &|&j| {
                    let mut out = Vec::with_capacity(w);
                    for i in 0..w {
                        let k = j * w + i;
                        out.push(if s.valid(k) {
                            mfd_cell(s, i, j, k, p)
                        } else {
                            (f64::NAN, NOFLOW)
                        });
                    }
                    out
                });
                r.total = Vec::with_capacity(n);
                if p == 0.0 {
                    r.steep = Vec::with_capacity(n);
                }
                for row in got {
                    for (t, q) in row {
                        r.total.push(t);
                        if p == 0.0 {
                            r.steep.push(q);
                        }
                    }
                }
            }
            Method::Dinf => {
                let got = par::map(threads, &rows, &|&j| {
                    (0..w)
                        .map(|i| {
                            let k = j * w + i;
                            if s.valid(k) {
                                dinf_facet(s, i, j, k)
                            } else {
                                (NOFLOW, 0.0)
                            }
                        })
                        .collect::<Vec<_>>()
                });
                r.facet = Vec::with_capacity(n);
                r.ratio = Vec::with_capacity(n);
                for row in got {
                    for (f, q) in row {
                        r.facet.push(f);
                        r.ratio.push(q);
                    }
                }
            }
        }
        r
    }

    /// Çoklu yön's exponent at cell `k` (at `i`, `j`).
    #[inline(always)]
    fn exponent(&self, s: &Surface, (i, j, k): (usize, usize, usize), p: f64) -> f64 {
        if p > 0.0 {
            return p;
        }
        let q = self.steep[k] as usize;
        let e = match s.at(i, j, k, q.min(7)) {
            Some(m) if q < 8 => slope(s, j, k, m, q),
            _ => 0.0,
        };
        8.9 * (if e < 1.0 { e } else { 1.0 }) + 1.1
    }

    /// The share cell `n` (at `ni`, `nj`) gives its neighbour `c` at `q`, when it gives any.
    #[inline(always)]
    fn give(
        &self,
        s: &Surface,
        dirs: &[u8],
        (ni, nj, n): (usize, usize, usize),
        c: usize,
        q: usize,
    ) -> Option<f64> {
        let d8 = || (dirs[n] as usize == q).then_some(1.0);
        match self.method {
            Method::D8 => d8(),
            Method::Mfd(p) => {
                let total = self.total[n];
                if total.is_nan() {
                    return d8();
                }
                let t = slope(s, nj, n, c, q);
                (t > 0.0).then(|| CONTOUR[q] * power(t, self.exponent(s, (ni, nj, n), p)) / total)
            }
            Method::Dinf => {
                let f = self.facet[n];
                if f == NOFLOW {
                    return d8();
                }
                let (e1, e2) = FACETS[f as usize];
                let r = self.ratio[n];
                let share = if q == e1 {
                    1.0 - r
                } else if q == e2 {
                    r
                } else {
                    0.0
                };
                (share > 0.0).then_some(share)
            }
        }
    }

    /// Whether cell `n` (row `nj`) sends anything to its neighbour `c` at `q`.
    #[inline(always)]
    fn sends(&self, s: &Surface, dirs: &[u8], nj: usize, n: usize, c: usize, q: usize) -> bool {
        match self.method {
            Method::D8 => dirs[n] as usize == q,
            Method::Mfd(_) => {
                if self.total[n].is_nan() {
                    dirs[n] as usize == q
                } else {
                    slope(s, nj, n, c, q) > 0.0
                }
            }
            Method::Dinf => {
                let f = self.facet[n];
                if f == NOFLOW {
                    return dirs[n] as usize == q;
                }
                let (e1, e2) = FACETS[f as usize];
                let r = self.ratio[n];
                (q == e1 && 1.0 - r > 0.0) || (q == e2 && r > 0.0)
            }
        }
    }
}

/// Çoklu yön at a cell: Σ L·tᵖ of its lower neighbours (NaN: none) and its steepest one.
fn mfd_cell(s: &Surface, i: usize, j: usize, k: usize, p: f64) -> (f64, u8) {
    let mut lower = [0.0f64; 8];
    let (mut e, mut steep) = (0.0, NOFLOW);
    s.each(i, j, k, |q, m| {
        let t = slope(s, j, k, m, q);
        if t > 0.0 {
            lower[q] = t;
            if t > e {
                e = t;
                steep = q as u8;
            }
        }
    });
    if steep == NOFLOW {
        return (f64::NAN, NOFLOW);
    }
    let p = if p > 0.0 {
        p
    } else {
        8.9 * (if e < 1.0 { e } else { 1.0 }) + 1.1
    };
    let mut total = 0.0;
    for (q, &t) in lower.iter().enumerate() {
        if t > 0.0 {
            total += CONTOUR[q] * power(t, p);
        }
    }
    (total, steep)
}

/// D∞'s facet of cell `k` at (`i`, `j`) and the diagonal neighbour's share (8 and 0: no facet goes down).
fn dinf_facet(s: &Surface, i: usize, j: usize, k: usize) -> (u8, f64) {
    let step = &s.row(j).step;
    let (mut best, mut chosen) = (0.0, (NOFLOW, 0.0));
    for (x, &(e1, e2)) in FACETS.iter().enumerate() {
        let (Some(m1), Some(m2)) = (s.at(i, j, k, e1), s.at(i, j, k, e2)) else {
            continue;
        };
        let (f0, f1, f2) = (s.z[k], s.z[m1], s.z[m2]);
        if f1.is_nan() || f2.is_nan() {
            continue;
        }
        let d1 = step[e1];
        let d2 = step[BETWEEN[x]];
        let s1 = (f0 - f1) / d1;
        let s2 = (f1 - f2) / d2;
        let mut r = atan2(s2, s1);
        let mut down = sqrt(s1 * s1 + s2 * s2);
        let alpha = atan2(d2, d1);
        if r < 0.0 {
            r = 0.0;
            down = s1;
        } else if r > alpha {
            r = alpha;
            down = (f0 - f2) / sqrt(d1 * d1 + d2 * d2);
        }
        if down > best {
            best = down;
            chosen = (x as u8, r / alpha);
        }
    }
    chosen
}

/// What a cell holds of itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Own {
    Cells,
    Area,
}

/// An accumulation under way.
pub struct Accumulating {
    own: Own,
    indeg: Vec<AtomicU8>,
    front: Vec<u32>,
    pub acc: Vec<f64>,
    done: usize,
    total: usize,
}

impl Accumulating {
    pub fn new(s: &Surface, dirs: &[u8], r: &Routing, own: Own, threads: usize) -> Accumulating {
        let w = s.width as usize;
        let n = s.len();
        let mut counts = vec![0u8; n];
        par::rows(threads, &mut counts, w, &|first, chunk: &mut [u8]| {
            for (r0, row) in chunk.chunks_mut(w).enumerate() {
                let j = first + r0;
                for (i, d) in row.iter_mut().enumerate() {
                    let k = j * w + i;
                    if !s.valid(k) {
                        continue;
                    }
                    let mut c = 0u8;
                    s.each(i, j, k, |q, m| {
                        if r.sends(s, dirs, m / w, m, k, back(q)) {
                            c += 1;
                        }
                    });
                    *d = c;
                }
            }
        });
        let front: Vec<u32> = (0..n)
            .filter(|&k| s.valid(k) && counts[k] == 0)
            .map(|k| k as u32)
            .collect();
        Accumulating {
            own,
            indeg: counts.into_iter().map(AtomicU8::new).collect(),
            front,
            acc: vec![f64::NAN; n],
            done: 0,
            total: s.z.iter().filter(|v| !v.is_nan()).count(),
        }
    }

    pub fn share(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.done as f64 / self.total as f64
        }
    }

    /// A cell's sum: its own and its donors' shares, the neighbours in their order.
    #[inline]
    fn pull(s: &Surface, dirs: &[u8], r: &Routing, own: Own, acc: &[f64], c: usize) -> f64 {
        let w = s.width as usize;
        let (i, j) = (c % w, c / w);
        let mut a = match own {
            Own::Cells => 1.0,
            Own::Area => s.row(j).area,
        };
        s.each(i, j, c, |q, m| {
            if let Some(share) = r.give(s, dirs, (m % w, m / w, m), c, back(q)) {
                a += share * acc[m];
            }
        });
        a
    }

    /// Counts down the donors of cell `c`'s receivers; those left with none go to `next`.
    #[inline]
    fn release(
        s: &Surface,
        dirs: &[u8],
        r: &Routing,
        indeg: &[AtomicU8],
        c: usize,
        next: &mut Vec<u32>,
    ) {
        let w = s.width as usize;
        let (i, j) = (c % w, c / w);
        s.each(i, j, c, |q, m| {
            if r.sends(s, dirs, j, c, m, q) && indeg[m].fetch_sub(1, Ordering::Relaxed) == 1 {
                next.push(m as u32);
            }
        });
    }

    /// Works out levels until [`STEP_CELLS`] cells are done; whether all are.
    pub fn step(&mut self, s: &Surface, dirs: &[u8], r: &Routing, threads: usize) -> bool {
        let mut budget = STEP_CELLS;
        while !self.front.is_empty() && budget > 0 {
            let front = std::mem::take(&mut self.front);
            let own = self.own;
            let next = if threads > 1 && front.len() >= PARALLEL_LEVEL {
                let pieces: Vec<&[u32]> = front.chunks(front.len().div_ceil(threads)).collect();
                let (acc, indeg) = (&self.acc, &self.indeg);
                let got = par::map(threads, &pieces, &|piece: &&[u32]| {
                    let vals: Vec<f64> = piece
                        .iter()
                        .map(|&c| Self::pull(s, dirs, r, own, acc, c as usize))
                        .collect();
                    let mut next = Vec::new();
                    for &c in piece.iter() {
                        Self::release(s, dirs, r, indeg, c as usize, &mut next);
                    }
                    (vals, next)
                });
                let mut next = Vec::new();
                for (piece, (vals, more)) in pieces.iter().zip(got) {
                    for (&c, v) in piece.iter().zip(vals) {
                        self.acc[c as usize] = v;
                    }
                    next.extend(more);
                }
                next
            } else {
                for &c in &front {
                    let v = Self::pull(s, dirs, r, own, &self.acc, c as usize);
                    self.acc[c as usize] = v;
                }
                let mut next = Vec::new();
                for &c in &front {
                    Self::release(s, dirs, r, &self.indeg, c as usize, &mut next);
                }
                next
            };
            self.done += front.len();
            budget = budget.saturating_sub(front.len());
            self.front = next;
        }
        self.front.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::surface::N;
    use super::*;

    #[test]
    fn the_facets_between_steps_are_square() {
        for (x, &(e1, e2)) in FACETS.iter().enumerate() {
            let d = (N[e2].0 - N[e1].0, N[e2].1 - N[e1].1);
            assert_eq!(N[BETWEEN[x]], d, "facet {x}");
        }
    }
}
