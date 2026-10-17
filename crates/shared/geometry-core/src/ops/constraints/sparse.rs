//! The rows' factor (docs/adr/0334 §4): a sparse LDLᵀ of the Gram matrix `G = J·Jᵀ` built row by row in the
//! constraints' order. A row's pivot is the square of its distance from the span of the rows kept before it, so a
//! row whose pivot is nought next to its own length repeats them and is not kept: the same test as Gram–Schmidt in
//! the same order, at the cost of the factor's fill rather than of a dense basis. Each row has at most a constraint's
//! few unknowns, so `G` is as sparse as the drawing's connections; a row's factor is found by a sparse forward solve
//! over the columns it reaches.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// A row of the Jacobian: (column, value), columns ascending.
pub(super) type Row = Vec<(usize, f64)>;

/// How small a pivot is next to its row's own square length for the row to repeat the ones before: a distance of
/// a ten-millionth of the row's length.
pub(super) const DEPENDENT: f64 = 1e-14;

fn dot(a: &Row, b: &Row) -> f64 {
    let (mut i, mut j, mut s) = (0, 0, 0.0);
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                s += a[i].1 * b[j].1;
                i += 1;
                j += 1;
            }
        }
    }
    s
}

/// The factor of the kept rows, and which rows were kept, dropped (a pivot of nought) or had no length.
pub(super) struct Factor {
    /// Per kept row (by position in `kept`): its column of `L` below the diagonal, (position, value) ascending.
    below: Vec<Vec<(usize, f64)>>,
    pivots: Vec<f64>,
    pub kept: Vec<usize>,
    pub dropped: Vec<usize>,
    pub zero: Vec<usize>,
}

impl Factor {
    /// Factors `rows` in order.
    pub fn new(rows: &[Row], columns: usize) -> Factor {
        let mut f = Factor {
            below: Vec::new(),
            pivots: Vec::new(),
            kept: Vec::new(),
            dropped: Vec::new(),
            zero: Vec::new(),
        };
        // Which kept rows touch each column.
        let mut touching: Vec<Vec<usize>> = vec![Vec::new(); columns];
        // The forward solve's scratch: values by kept position, whether set, and the positions to visit.
        let mut z: Vec<f64> = Vec::new();
        let mut set: Vec<bool> = Vec::new();
        let mut heap: BinaryHeap<Reverse<usize>> = BinaryHeap::new();
        let mut seen: Vec<usize> = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let own = dot(row, row);
            if !own.is_finite() || own == 0.0 {
                f.zero.push(i);
                continue;
            }
            // g: the row's products with the kept rows it shares a column with.
            for &(c, _) in row {
                for &k in &touching[c] {
                    if !set[k] {
                        set[k] = true;
                        z[k] = dot(row, &rows[f.kept[k]]);
                        heap.push(Reverse(k));
                        seen.push(k);
                    }
                }
            }
            // L·z = g, column by column from the smallest; `z[k] / d_k` is the row's entry of `L`.
            let mut left = own;
            let mut entries: Vec<(usize, f64)> = Vec::new();
            while let Some(Reverse(k)) = heap.pop() {
                let zk = z[k];
                if zk == 0.0 {
                    continue;
                }
                let l = zk / f.pivots[k];
                left -= zk * l;
                entries.push((k, l));
                for &(j, ljk) in &f.below[k] {
                    if !set[j] {
                        set[j] = true;
                        z[j] = 0.0;
                        heap.push(Reverse(j));
                        seen.push(j);
                    }
                    z[j] -= ljk * zk;
                }
            }
            for &k in &seen {
                set[k] = false;
            }
            seen.clear();
            if !(left > DEPENDENT * own) {
                f.dropped.push(i);
                continue;
            }
            let at = f.kept.len();
            for &(k, l) in &entries {
                f.below[k].push((at, l));
            }
            f.below.push(Vec::new());
            f.pivots.push(left);
            f.kept.push(i);
            z.push(0.0);
            set.push(false);
            for &(c, _) in row {
                touching[c].push(at);
            }
        }
        f
    }

    /// The rank: how many rows were kept.
    pub fn rank(&self) -> usize {
        self.kept.len()
    }

    /// Solves `G·y = b` over the kept rows (`b` by kept position).
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let mut y = b.to_vec();
        for k in 0..y.len() {
            let yk = y[k];
            if yk != 0.0 {
                for &(j, l) in &self.below[k] {
                    y[j] -= l * yk;
                }
            }
        }
        for (v, d) in y.iter_mut().zip(&self.pivots) {
            *v /= d;
        }
        for k in (0..y.len()).rev() {
            let s: f64 = self.below[k].iter().map(|&(j, l)| l * y[j]).sum();
            y[k] -= s;
        }
        y
    }

    /// The least-norm `x` with `J_kept·x = b`: `x = J_keptᵀ·G⁻¹·b`.
    pub fn least_norm(&self, rows: &[Row], b: &[f64], columns: usize) -> Vec<f64> {
        let y = self.solve(b);
        let mut x = vec![0.0; columns];
        for (k, &i) in self.kept.iter().enumerate() {
            for &(c, v) in &rows[i] {
                x[c] += y[k] * v;
            }
        }
        x
    }

    /// `v` less its part in the kept rows' span.
    pub fn null_part(&self, rows: &[Row], v: &[f64]) -> Vec<f64> {
        let b: Vec<f64> = self
            .kept
            .iter()
            .map(|&i| rows[i].iter().map(|&(c, x)| x * v[c]).sum())
            .collect();
        let along = self.least_norm(rows, &b, v.len());
        v.iter().zip(along).map(|(a, b)| a - b).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_in_the_span_are_dropped() {
        let rows: Vec<Row> = vec![
            vec![(0, 1.0), (1, 1.0)],
            vec![(1, 2.0), (2, 1.0)],
            vec![(0, 2.0), (1, 4.0), (2, 1.0)], // the first plus the second
            vec![],
            vec![(2, 3.0)],
        ];
        let f = Factor::new(&rows, 3);
        assert_eq!(
            (f.kept.clone(), f.dropped.clone(), f.zero.clone()),
            (vec![0, 1, 4], vec![2], vec![3])
        );
        // Square and independent: the least-norm solution is the solution.
        let x = f.least_norm(&rows, &[3.0, 7.0, 6.0], 3);
        assert!(
            (x[0] - 0.5).abs() < 1e-12 && (x[1] - 2.5).abs() < 1e-12 && (x[2] - 2.0).abs() < 1e-12
        );
        let n = f.null_part(&rows, &[1.0, 2.0, 3.0]);
        assert!(n.iter().all(|v| v.abs() < 1e-12));
    }
}
