//! Çukur doldur (docs/adr/0235 §3): the least surface f ≥ z whose every
//! inner cell has a neighbour n with f(n) + ε·d ≤ f(c).
//!
//! With ε = 0 a priority flood from the outlets (Barnes et al. 2014): the
//! lowest frontier cell sets the level; a cell is final when first reached,
//! f = max(z, level). Cells under the level flood through a FIFO queue. A
//! slope cell above the level (f = z: its path through the cell that reached
//! it peaks at itself) goes into the heap only when an unreached neighbour
//! is not higher than it, the neighbour it might yet raise; otherwise its
//! unreached neighbours, all higher, are reached at once and traced in turn
//! (Zhou et al. 2016's idea): reaching a higher neighbour from a cell whose
//! f is its z gives it its own z whenever it happens. The surface is the same
//! in any order; the heap only keeps what the order matters for.
//!
//! With ε > 0 the cells settle as in Dijkstra's search (Knuth's monotone
//! form): a candidate max(z, f + ε·d) never falls below the cell it came from.
//!
//! Both work in budgets of cells per step, so a host can show the share and
//! stop the run between steps.

use std::collections::VecDeque;

use super::heap::{RadixHeap, key_of, value_of};
use super::surface::{N, Surface};

/// Cells a step settles at most.
pub const STEP_CELLS: usize = 1 << 21;

/// A fill under way.
#[derive(Debug)]
pub struct Filling {
    eps: f64,
    heap: RadixHeap,
    pit: VecDeque<u32>,
    trace: Vec<u32>,
    /// 1 once a cell is reached (ε = 0) or settled (ε > 0).
    closed: Vec<u8>,
    /// ε > 0: each cell's best candidate so far.
    best: Vec<f64>,
    settled: usize,
    total: usize,
    /// Cells raised above their own height.
    pub raised: u64,
}

impl Filling {
    /// The outlets in the heap (row by row); ε the least slope (a ratio).
    pub fn new(s: &Surface, eps: f64) -> Filling {
        let n = s.len();
        let w = s.width as usize;
        let mut f = Filling {
            eps,
            heap: RadixHeap::new(),
            pit: VecDeque::new(),
            trace: Vec::new(),
            closed: vec![0; n],
            best: if eps > 0.0 {
                vec![f64::INFINITY; n]
            } else {
                Vec::new()
            },
            settled: 0,
            total: s.z.iter().filter(|v| !v.is_nan()).count(),
            raised: 0,
        };
        for k in 0..n {
            if !s.valid(k) || !s.outlet(k % w, k / w) {
                continue;
            }
            if eps > 0.0 {
                f.best[k] = s.z[k];
            } else {
                f.closed[k] = 1;
            }
            f.heap.push(key_of(s.z[k]), k as u32);
        }
        f
    }

    /// The share of the cells settled.
    pub fn share(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.settled as f64 / self.total as f64
        }
    }

    /// Settles up to [`STEP_CELLS`] cells of `s`'s surface (its heights
    /// rise in place); whether the fill is done.
    pub fn step(&mut self, s: &mut Surface) -> bool {
        if self.eps > 0.0 {
            self.step_slope(s)
        } else {
            self.step_flat(s)
        }
    }

    fn step_flat(&mut self, s: &mut Surface) -> bool {
        let w = s.width as usize;
        let mut budget = STEP_CELLS;
        while budget > 0 {
            // A depression's cells first, then the traced slope, then the next level.
            let c = if let Some(c) = self.pit.pop_front() {
                c
            } else if let Some(c) = self.trace.pop() {
                c
            } else if let Some((_, c)) = self.heap.pop() {
                c
            } else {
                return true;
            };
            budget -= 1;
            self.settled += 1;
            let c = c as usize;
            let level = s.z[c];
            let (i, j) = (c % w, c / w);
            let (mut nb, mut count) = ([0usize; 8], 0);
            s.each(i, j, c, |_, m| {
                nb[count] = m;
                count += 1;
            });
            for &m in &nb[..count] {
                if self.closed[m] != 0 {
                    continue;
                }
                self.closed[m] = 1;
                if s.z[m] <= level {
                    if s.z[m] < level {
                        self.raised += 1;
                    }
                    s.z[m] = level;
                    self.pit.push_back(m as u32);
                } else {
                    self.slope(s, m);
                }
            }
        }
        false
    }

    /// A slope cell just reached: into the heap while an unreached neighbour is not higher, else traced.
    #[inline]
    fn slope(&mut self, s: &Surface, m: usize) {
        let w = s.width as usize;
        let (i, j) = (m % w, m / w);
        let zm = s.z[m];
        let mut spills = false;
        s.each(i, j, m, |_, n| {
            if self.closed[n] == 0 && s.z[n] <= zm {
                spills = true;
            }
        });
        if spills {
            self.heap.push(key_of(zm), m as u32);
        } else {
            self.trace.push(m as u32);
        }
    }

    fn step_slope(&mut self, s: &mut Surface) -> bool {
        let w = s.width as usize;
        let eps = self.eps;
        let mut budget = STEP_CELLS;
        while budget > 0 {
            let Some((key, c)) = self.heap.pop() else {
                // Every cell settled: the surface is the best candidates.
                for (k, z) in s.z.iter_mut().enumerate() {
                    if !z.is_nan() {
                        if self.best[k] > *z {
                            self.raised += 1;
                        }
                        *z = self.best[k];
                    }
                }
                return true;
            };
            let c = c as usize;
            let v = value_of(key);
            if self.closed[c] != 0 || v != self.best[c] {
                continue;
            }
            budget -= 1;
            self.closed[c] = 1;
            self.settled += 1;
            let (i, j) = (c % w, c / w);
            for (q, &(_, dj)) in N.iter().enumerate() {
                let Some(m) = s.neighbour(i, j, q) else {
                    continue;
                };
                if self.closed[m] != 0 {
                    continue;
                }
                // The step from m (its row) back to c.
                let jm = (j as i64 + dj) as usize;
                let back = s.row(jm).step[super::surface::back(q)];
                let up = v + eps * back;
                let cand = if s.z[m] > up { s.z[m] } else { up };
                if cand < self.best[m] {
                    self.best[m] = cand;
                    self.heap.push(key_of(cand), m as u32);
                }
            }
        }
        false
    }
}
