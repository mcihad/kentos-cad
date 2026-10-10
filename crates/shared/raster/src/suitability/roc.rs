//! ROC ile doğrulama (docs/adr/0237 §8): how well a raster's values pick
//! out the cells of known events. The samples are cells: a point's cell, an
//! area's cells (their centres inside); the background is every cell with a
//! value (the success rate) or the absence objects' cells. The first pass
//! reads the samples' values; with every cell as the background a second
//! pass counts each cell against the sorted presence values, so the memory
//! is the presence's alone. AUC is Mann–Whitney's, counted in integers.

use kentos_geometry_core::entity::Shape;

use crate::areas::{Areas, Span, union};
use crate::grid::Grid;
use crate::inputs::place_in;

/// The curve's most rows.
pub const MOST_ROWS: usize = 200;

/// A set of sample cells: the areas' and the points' (§8).
pub struct Cells {
    areas: Option<Areas>,
    /// The points' cells, (row, column), sorted, each once.
    points: Vec<(u32, u32)>,
    /// Points off the raster.
    pub outside: u64,
}

impl Cells {
    /// The cells of `shapes` on `grid`: a point's (multi-points' each), an area's.
    pub fn new(grid: &Grid, shapes: &[Shape]) -> Cells {
        let mut points = Vec::new();
        let mut outside = 0u64;
        let mut areas = Vec::new();
        let mut at = |x: f64, y: f64| {
            let (u, v) = place_in(&grid.affine, x, y);
            let (i, j) = (u.floor(), v.floor());
            if i >= 0.0 && j >= 0.0 && i < f64::from(grid.width) && j < f64::from(grid.height) {
                points.push((j as u32, i as u32));
            } else {
                outside += 1;
            }
        };
        for s in shapes {
            match s {
                Shape::Point { p, parts, .. } => {
                    at(p.x, p.y);
                    for q in parts.iter().flatten() {
                        at(q.p.x, q.p.y);
                    }
                }
                _ => areas.push(s.clone()),
            }
        }
        points.sort_unstable();
        points.dedup();
        Cells {
            areas: (!areas.is_empty()).then(|| Areas::new(grid, &areas)),
            points,
            outside,
        }
    }

    /// The area pieces near rows `j0..j1`.
    fn strip(&self, j0: u32, j1: u32) -> Vec<u32> {
        self.areas
            .as_ref()
            .map_or_else(Vec::new, |a| a.strip(j0, j1))
    }

    /// Row `j`'s sample columns within `c0..c1`, sorted, each once.
    fn row(
        &self,
        j: u32,
        (c0, c1): (u32, u32),
        near: &[u32],
        scratch: &mut Scratch,
        out: &mut Vec<u32>,
    ) {
        out.clear();
        if let Some(a) = &self.areas {
            a.row(j, near, &mut scratch.spans, &mut scratch.cuts);
            union(&scratch.spans, &mut scratch.merged);
            for &(a, z) in &scratch.merged {
                out.extend(a.max(c0)..z.min(c1));
            }
        }
        let from = self.points.partition_point(|&(pj, pi)| (pj, pi) < (j, c0));
        let before = out.len();
        out.extend(
            self.points[from..]
                .iter()
                .take_while(|&&(pj, pi)| pj == j && pi < c1)
                .map(|&(_, i)| i),
        );
        if before > 0 && out.len() > before {
            out.sort_unstable();
            out.dedup();
        }
    }
}

/// A row's working lists.
#[derive(Default)]
struct Scratch {
    spans: Vec<Span>,
    cuts: Vec<(u32, i64)>,
    merged: Vec<(u32, u32)>,
}

/// A row of the curve: the threshold and the samples at or above it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RocRow {
    pub threshold: f64,
    pub tp: u64,
    pub fp: u64,
}

/// The validation (§8).
#[derive(Clone, Debug, PartialEq)]
pub struct Roc {
    /// Presence cells P and background cells N.
    pub presence: u64,
    pub background: u64,
    /// The background is every cell with a value (the success rate).
    pub all_cells: bool,
    pub auc: f64,
    pub rows: Vec<RocRow>,
    /// Youden's best row.
    pub best: Option<usize>,
    /// Sample cells without a value; points off the raster.
    pub skipped: u64,
    pub outside: u64,
    /// Cells both presence and absence.
    pub both: u64,
}

/// What a strip's row gave the first pass.
struct FirstRow {
    presence: Vec<f64>,
    absence: Vec<f64>,
    skipped: u64,
    both: u64,
}

/// What a row gave the second pass.
struct SecondRow {
    greater: u128,
    equal: u128,
    n: u64,
    hist: Vec<u64>,
}

/// The run's state.
pub struct RocWork {
    pub band: usize,
    higher: bool,
    presence: Cells,
    absence: Option<Cells>,
    pres: Vec<f64>,
    abs: Vec<f64>,
    skipped: u64,
    both: u64,
    /// The second pass (every cell against the presence) is under way.
    pub second: bool,
    pub done: bool,
    /// The presence's values sorted, and the curve's thresholds rising.
    sorted: Vec<f64>,
    rising: Vec<f64>,
    greater: u128,
    equal: u128,
    n: u64,
    hist: Vec<u64>,
}

impl RocWork {
    /// The objects before `first` are the presence, the rest the absence
    /// (with the absence as the background; else every cell).
    pub fn new(
        grid: &Grid,
        band: usize,
        shapes: &[Shape],
        first: usize,
        absence: bool,
        higher: bool,
    ) -> Result<RocWork, String> {
        let first = first.min(shapes.len());
        if first == 0 {
            return Err("Varlık nesnelerini seçin: noktalar ya da alanlar.".into());
        }
        if absence && first == shapes.len() {
            return Err("Yokluk nesnelerini seçin: noktalar ya da alanlar.".into());
        }
        Ok(RocWork {
            band,
            higher,
            presence: Cells::new(grid, &shapes[..first]),
            absence: absence.then(|| Cells::new(grid, &shapes[first..])),
            pres: Vec::new(),
            abs: Vec::new(),
            skipped: 0,
            both: 0,
            second: false,
            done: false,
            sorted: Vec::new(),
            rising: Vec::new(),
            greater: 0,
            equal: 0,
            n: 0,
            hist: Vec::new(),
        })
    }

    /// Whether every cell is the background (two passes).
    pub fn all_cells(&self) -> bool {
        self.absence.is_none()
    }

    /// A strip's rows `rows` over columns `c0..c1`; `line(j, out)` reads row j's values there.
    pub fn block(
        &mut self,
        (c0, c1): (u32, u32),
        rows: &[u32],
        threads: usize,
        line: &(dyn Fn(u32, &mut [f64]) + Sync),
    ) {
        let bw = (c1 - c0) as usize;
        let (j0, j1) = (
            rows.first().copied().unwrap_or(0),
            rows.last().map_or(0, |j| j + 1),
        );
        if self.second {
            let (sorted, rising) = (&self.sorted, &self.rising);
            let higher = self.higher;
            let made: Vec<SecondRow> = crate::par::map(threads, rows, &|&j| {
                let mut values = vec![0.0; bw];
                line(j, &mut values);
                let mut out = SecondRow {
                    greater: 0,
                    equal: 0,
                    n: 0,
                    hist: vec![0; rising.len() + 1],
                };
                for &v in &values {
                    if v.is_nan() {
                        continue;
                    }
                    let s = if higher { v } else { -v };
                    count(sorted, rising, s, &mut out);
                }
                out
            });
            for r in made {
                self.greater += r.greater;
                self.equal += r.equal;
                self.n += r.n;
                for (h, c) in self.hist.iter_mut().zip(&r.hist) {
                    *h += c;
                }
            }
            return;
        }
        let near_p = self.presence.strip(j0, j1);
        let near_a = self
            .absence
            .as_ref()
            .map(|a| a.strip(j0, j1))
            .unwrap_or_default();
        let (presence, absence) = (&self.presence, self.absence.as_ref());
        let higher = self.higher;
        let made: Vec<FirstRow> = crate::par::map(threads, rows, &|&j| {
            let mut scratch = Scratch::default();
            let mut values = vec![0.0; bw];
            line(j, &mut values);
            let mut out = FirstRow {
                presence: Vec::new(),
                absence: Vec::new(),
                skipped: 0,
                both: 0,
            };
            let mut cols = Vec::new();
            presence.row(j, (c0, c1), &near_p, &mut scratch, &mut cols);
            let take = |cols: &[u32], into: &mut Vec<f64>, skipped: &mut u64| {
                for &i in cols {
                    let v = values[(i - c0) as usize];
                    if v.is_nan() {
                        *skipped += 1;
                    } else {
                        into.push(if higher { v } else { -v });
                    }
                }
            };
            take(&cols, &mut out.presence, &mut out.skipped);
            if let Some(a) = absence {
                let mut other = Vec::new();
                a.row(j, (c0, c1), &near_a, &mut scratch, &mut other);
                take(&other, &mut out.absence, &mut out.skipped);
                out.both = both_in(&cols, &other);
            }
            out
        });
        for r in made {
            self.pres.extend(r.presence);
            self.abs.extend(r.absence);
            self.skipped += r.skipped;
            self.both += r.both;
        }
    }

    /// The first pass is read: the presence sorted and the thresholds kept;
    /// whether a second pass (every cell) follows.
    pub fn first_done(&mut self) -> Result<bool, String> {
        if self.pres.is_empty() {
            return Err(
                "Rasterin değerli hücrelerine düşen varlık yok: varlık nesneleri rasterin üstünde olmalı.".into(),
            );
        }
        self.pres.sort_unstable_by(f64::total_cmp);
        let sorted = std::mem::take(&mut self.pres);
        let mut distinct = sorted.clone();
        distinct.dedup();
        distinct.reverse();
        let m = distinct.len();
        let kept: Vec<f64> = if m <= MOST_ROWS {
            distinct
        } else {
            (1..=MOST_ROWS)
                .map(|k| distinct[(k * m).div_ceil(MOST_ROWS) - 1])
                .collect()
        };
        self.rising = kept.into_iter().rev().collect();
        self.sorted = sorted;
        self.hist = vec![0; self.rising.len() + 1];
        if self.absence.is_some() {
            let abs = std::mem::take(&mut self.abs);
            let mut out = SecondRow {
                greater: 0,
                equal: 0,
                n: 0,
                hist: std::mem::take(&mut self.hist),
            };
            for &s in &abs {
                count(&self.sorted, &self.rising, s, &mut out);
            }
            self.greater = out.greater;
            self.equal = out.equal;
            self.n = out.n;
            self.hist = out.hist;
            self.done = true;
            return Ok(false);
        }
        self.second = true;
        Ok(true)
    }

    /// The second pass is read.
    pub fn second_done(&mut self) {
        self.done = true;
    }

    /// The validation's figures.
    pub fn finish(self) -> Result<Roc, String> {
        let p = self.sorted.len() as u64;
        let n = self.n;
        if n == 0 {
            return Err(if self.absence.is_some() {
                "Rasterin değerli hücrelerine düşen yokluk yok: yokluk nesneleri rasterin üstünde olmalı.".into()
            } else {
                "Rasterin değerli hücresi yok.".into()
            });
        }
        let num = 2 * self.greater + self.equal;
        let den = 2 * u128::from(p) * u128::from(n);
        let auc = num as f64 / den as f64;
        // FP of the k-th rising threshold: the cells counted at or past it.
        let mut fp_rising = vec![0u64; self.rising.len()];
        let mut acc = 0u64;
        for k in (0..self.rising.len()).rev() {
            acc += self.hist[k + 1];
            fp_rising[k] = acc;
        }
        let rows: Vec<RocRow> = (0..self.rising.len())
            .rev()
            .map(|k| {
                let t = self.rising[k];
                let tp = p - self.sorted.partition_point(|&v| v < t) as u64;
                RocRow {
                    threshold: if self.higher { t } else { -t + 0.0 },
                    tp,
                    fp: fp_rising[k],
                }
            })
            .collect();
        let mut best: Option<(usize, f64)> = None;
        for (k, r) in rows.iter().enumerate() {
            let j = r.tp as f64 / p as f64 - r.fp as f64 / n as f64;
            if best.is_none_or(|(_, b)| j > b) {
                best = Some((k, j));
            }
        }
        Ok(Roc {
            presence: p,
            background: n,
            all_cells: self.absence.is_none(),
            auc,
            rows,
            best: best.map(|(k, _)| k),
            skipped: self.skipped,
            outside: self.presence.outside + self.absence.as_ref().map_or(0, |a| a.outside),
            both: self.both,
        })
    }

    /// The share of the work done, `f` the pass's.
    pub fn share(&self, f: f64) -> f64 {
        match (self.all_cells(), self.second) {
            (false, _) => f,
            (true, false) => f / 2.0,
            (true, true) => 0.5 + f / 2.0,
        }
    }
}

/// A background value `s` against the sorted presence: the pairs it makes
/// (presence greater, equal) and the thresholds at or below it.
#[inline]
fn count(sorted: &[f64], rising: &[f64], s: f64, out: &mut SecondRow) {
    let below = sorted.partition_point(|&v| v < s);
    let upto = sorted.partition_point(|&v| v <= s);
    out.greater += (sorted.len() - upto) as u128;
    out.equal += (upto - below) as u128;
    out.n += 1;
    out.hist[rising.partition_point(|&t| t <= s)] += 1;
}

/// The columns in both sorted lists.
fn both_in(a: &[u32], b: &[u32]) -> u64 {
    let (mut i, mut j, mut n) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                n += 1;
                i += 1;
                j += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn second(sorted: &[f64], rising: &[f64], values: &[f64]) -> SecondRow {
        let mut out = SecondRow {
            greater: 0,
            equal: 0,
            n: 0,
            hist: vec![0; rising.len() + 1],
        };
        for &v in values {
            count(sorted, rising, v, &mut out);
        }
        out
    }

    #[test]
    fn pairs_are_counted_with_ties_half() {
        // Presence 1, 3, 3; background 0, 3, 5.
        let r = second(&[1.0, 3.0, 3.0], &[1.0, 3.0], &[0.0, 3.0, 5.0]);
        // 0: all three greater; 3: one below (1), two equal; 5: none.
        assert_eq!((r.greater, r.equal, r.n), (3, 2, 3));
        // Thresholds 1 and 3: 0 is below both, 3 and 5 at or past both.
        assert_eq!(r.hist, vec![1, 0, 2]);
        assert_eq!(both_in(&[1, 3, 5, 9], &[0, 3, 9]), 2);
    }
}
