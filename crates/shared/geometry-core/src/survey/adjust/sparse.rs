//! The normal equations of a large GNSS network (docs/adr/4560). The stations are ordered by reverse Cuthill–McKee
//! (Cuthill and McKee 1969, as SciPy's `reverse_cuthill_mckee` orders a sparse matrix), so that the matrix kept by rows
//! within its envelope (each row from its first non-zero column to the diagonal) stays narrow; its Cholesky factor fills
//! nothing outside the envelope and keeps ADR 0203's pivot rule. The inverse is not formed: its entries within the
//! envelope come from the Takahashi equations (Takahashi, Fagan and Chin 1973; the sparse inverse subset on the
//! factor's pattern, as SuiteSparse's sparseinv computes it), and they are every entry the statistics, a station's
//! covariance and an adjusted baseline's read (an observation joins two stations, whose rows reach each other).

use super::linalg::{Inverse, PIVOT};

/// Reverse Cuthill–McKee over a graph given by each node's neighbours (no node its own): the nodes in their new order.
/// Component by component (each from its lowest-numbered node), the walk starts at a pseudo-peripheral node (George and
/// Liu's search: from the component's node of least degree, the last level's node of least degree while the levels
/// grow deeper) and takes each node's unvisited neighbours by increasing degree, ties by number; the whole is reversed.
pub(crate) fn reverse_cuthill_mckee(neighbours: &[Vec<usize>]) -> Vec<usize> {
    let n = neighbours.len();
    let degree = |v: usize| neighbours[v].len();
    let mut seen = vec![false; n];
    let mut order = Vec::with_capacity(n);
    // The levels from `root` within its component, by breadth first search (`mark` holds the visit's stamp).
    let mut mark = vec![usize::MAX; n];
    let mut stamp = 0usize;
    let mut levels = |root: usize, mark: &mut Vec<usize>| -> Vec<Vec<usize>> {
        stamp += 1;
        mark[root] = stamp;
        let mut out = vec![vec![root]];
        loop {
            let mut next = Vec::new();
            for &v in out.last().map_or(&[][..], |l| l.as_slice()) {
                for &w in &neighbours[v] {
                    if mark[w] != stamp {
                        mark[w] = stamp;
                        next.push(w);
                    }
                }
            }
            if next.is_empty() {
                return out;
            }
            out.push(next);
        }
    };
    for first in 0..n {
        if seen[first] {
            continue;
        }
        // The component's node of least degree, ties by number.
        let component: Vec<usize> = levels(first, &mut mark).into_iter().flatten().collect();
        let mut root = component.iter().copied().min_by_key(|&v| (degree(v), v)).unwrap_or(first);
        let mut depth = levels(root, &mut mark).len();
        loop {
            let last = levels(root, &mut mark).pop().unwrap_or_default();
            let Some(far) = last.iter().copied().min_by_key(|&v| (degree(v), v)) else { break };
            let deeper = levels(far, &mut mark).len();
            if deeper > depth {
                root = far;
                depth = deeper;
            } else {
                break;
            }
        }
        // Cuthill–McKee from the root.
        seen[root] = true;
        let start = order.len();
        order.push(root);
        let mut k = start;
        while k < order.len() {
            let v = order[k];
            let mut next: Vec<usize> = neighbours[v].iter().copied().filter(|&w| !seen[w]).collect();
            next.sort_by_key(|&w| (degree(w), w));
            for w in next {
                seen[w] = true;
                order.push(w);
            }
            k += 1;
        }
    }
    order.reverse();
    order
}

/// A symmetric matrix kept by rows within its envelope: row `i` holds columns `first[i]..=i`.
#[derive(Clone, Debug)]
pub(crate) struct Envelope {
    first: Vec<usize>,
    start: Vec<usize>,
    a: Vec<f64>,
}

impl Envelope {
    /// The entries an envelope of rows starting at `first` holds (each `first[i] ≤ i`).
    pub(crate) fn entries(first: &[usize]) -> usize {
        first.iter().enumerate().map(|(i, &f)| i + 1 - f.min(i)).sum()
    }

    pub(crate) fn new(mut first: Vec<usize>) -> Envelope {
        let mut start = Vec::with_capacity(first.len() + 1);
        let mut at = 0usize;
        for (i, f) in first.iter_mut().enumerate() {
            *f = (*f).min(i);
            start.push(at);
            at += i + 1 - *f;
        }
        start.push(at);
        Envelope { first, start, a: vec![0.0; at] }
    }

    pub(crate) fn size(&self) -> usize {
        self.first.len()
    }

    fn at(&self, i: usize, j: usize) -> Option<usize> {
        let (r, c) = if i >= j { (i, j) } else { (j, i) };
        (c >= self.first[r]).then(|| self.start[r] + c - self.first[r])
    }

    /// Adds `v` at (i, j) and (j, i); an entry outside the envelope is a caller's slip and is not kept.
    pub(crate) fn add(&mut self, i: usize, j: usize, v: f64) {
        debug_assert!(self.at(i, j).is_some(), "({i}, {j}) outside the envelope");
        if let Some(k) = self.at(i, j) {
            self.a[k] += v;
        }
    }

    /// The row's stored part: columns `first[i]..=i`.
    fn row(&self, i: usize) -> &[f64] {
        &self.a[self.start[i]..self.start[i + 1]]
    }

    /// Its Cholesky factor in the same envelope, worked in place (a row's diagonal is read before it is replaced), or the
    /// first unknown (in this order) whose pivot is not above [`PIVOT`] times its own diagonal.
    pub(crate) fn cholesky(mut self) -> Result<EnvelopeFactor, usize> {
        let n = self.size();
        for i in 0..n {
            let fi = self.first[i];
            let ri = self.start[i];
            for j in fi..i {
                let fj = self.first[j];
                let rj = self.start[j];
                let mut s = self.a[ri + j - fi];
                for k in fi.max(fj)..j {
                    s -= self.a[ri + k - fi] * self.a[rj + k - fj];
                }
                self.a[ri + j - fi] = s / self.a[rj + j - fj];
            }
            let diagonal = self.a[ri + i - fi];
            let mut s = diagonal;
            for k in fi..i {
                let v = self.a[ri + k - fi];
                s -= v * v;
            }
            if !(s > PIVOT * diagonal) {
                return Err(i);
            }
            self.a[ri + i - fi] = s.sqrt();
        }
        Ok(EnvelopeFactor { l: self })
    }
}

/// A lower triangular Cholesky factor L of N = L·Lᵀ within N's envelope.
#[derive(Clone, Debug)]
pub(crate) struct EnvelopeFactor {
    l: Envelope,
}

impl EnvelopeFactor {
    fn diagonal(&self, i: usize) -> f64 {
        self.l.a[self.l.start[i + 1] - 1]
    }

    /// x with N·x = b.
    pub(crate) fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.l.size();
        let mut y = b.to_vec();
        for i in 0..n {
            let fi = self.l.first[i];
            let row = self.l.row(i);
            let mut t = y[i];
            for k in fi..i {
                t -= row[k - fi] * y[k];
            }
            y[i] = t / row[i - fi];
        }
        for i in (0..n).rev() {
            let fi = self.l.first[i];
            let row = self.l.row(i);
            y[i] /= row[i - fi];
            let yi = y[i];
            for k in fi..i {
                y[k] -= row[k - fi] * yi;
            }
        }
        y
    }

    /// N⁻¹'s entries within the envelope by the Takahashi equations: from the last column back, for i > j in column
    /// j's pattern Zᵢⱼ = −Σₖ Lₖⱼ·Zᵢₖ / Lⱼⱼ and Zⱼⱼ = (1/Lⱼⱼ − Σₖ Lₖⱼ·Zₖⱼ) / Lⱼⱼ, k over the rows below j whose envelope
    /// reaches column j (every Zᵢₖ they read lies within the envelope and is already known).
    pub(crate) fn inverse(&self) -> EnvelopeInverse {
        let n = self.l.size();
        // Column j's rows below the diagonal: i > j with first[i] ≤ j, ascending.
        let mut count = vec![0usize; n + 1];
        for i in 0..n {
            for j in self.l.first[i]..i {
                count[j + 1] += 1;
            }
        }
        for j in 0..n {
            count[j + 1] += count[j];
        }
        let mut rows = vec![0u32; count[n]];
        let mut fill = count.clone();
        for i in 0..n {
            for j in self.l.first[i]..i {
                rows[fill[j]] = u32::try_from(i).unwrap_or(u32::MAX);
                fill[j] += 1;
            }
        }
        let mut z = Envelope { first: self.l.first.clone(), start: self.l.start.clone(), a: vec![0.0; self.l.a.len()] };
        let entry = |e: &Envelope, i: usize, j: usize| e.at(i, j).map_or(0.0, |k| e.a[k]);
        for j in (0..n).rev() {
            let below = &rows[count[j]..count[j + 1]];
            let ljj = self.diagonal(j);
            let lkj = |k: usize| self.l.a[self.l.start[k] + j - self.l.first[k]];
            for &i in below {
                let i = i as usize;
                let mut s = 0.0;
                for &k in below {
                    let k = k as usize;
                    s += lkj(k) * entry(&z, i, k);
                }
                let at = z.start[i] + j - z.first[i];
                z.a[at] = -s / ljj;
            }
            let mut s = 0.0;
            for &k in below {
                let k = k as usize;
                s += lkj(k) * z.a[z.start[k] + j - z.first[k]];
            }
            let at = z.start[j + 1] - 1;
            z.a[at] = (1.0 / ljj - s) / ljj;
        }
        EnvelopeInverse { z }
    }
}

/// The entries of N⁻¹ within N's envelope.
#[derive(Clone, Debug)]
pub(crate) struct EnvelopeInverse {
    z: Envelope,
}

impl Inverse for EnvelopeInverse {
    fn size(&self) -> usize {
        self.z.size()
    }

    /// An entry within the envelope; one outside it is not held (NaN, so that a slip shows).
    fn get(&self, i: usize, j: usize) -> f64 {
        self.z.at(i, j).map_or(f64::NAN, |k| self.z.a[k])
    }
}

#[cfg(test)]
mod tests {
    use super::super::linalg::Normal;
    use super::*;

    /// A ring of `n` nodes with chords every `step`: neighbours sorted.
    fn ring(n: usize, step: usize) -> Vec<Vec<usize>> {
        let mut g = vec![Vec::new(); n];
        for i in 0..n {
            for d in [1, step] {
                let j = (i + d) % n;
                if j != i && !g[i].contains(&j) {
                    g[i].push(j);
                    g[j].push(i);
                }
            }
        }
        for v in &mut g {
            v.sort_unstable();
        }
        g
    }

    /// A positive definite matrix on the graph: each edge a 2×2 Laplacian of weight 1 + (i·j mod 7)/10, the diagonal
    /// raised by 0.5 at every third node.
    fn matrix(g: &[Vec<usize>]) -> Vec<Vec<f64>> {
        let n = g.len();
        let mut m = vec![vec![0.0; n]; n];
        for (i, list) in g.iter().enumerate() {
            for &j in list {
                if j > i {
                    let w = 1.0 + ((i * j) % 7) as f64 / 10.0;
                    m[i][i] += w;
                    m[j][j] += w;
                    m[i][j] -= w;
                    m[j][i] -= w;
                }
            }
            if i % 3 == 0 {
                m[i][i] += 0.5;
            }
        }
        m
    }

    #[test]
    fn the_order_is_a_permutation_and_narrows_a_ring() {
        let g = ring(40, 2);
        let order = reverse_cuthill_mckee(&g);
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..40).collect::<Vec<_>>());
        let mut at = vec![0usize; 40];
        for (p, &v) in order.iter().enumerate() {
            at[v] = p;
        }
        let band = |pos: &dyn Fn(usize) -> usize| {
            (0..40).flat_map(|i| g[i].iter().map(move |&j| (i, j))).map(|(i, j)| pos(i).abs_diff(pos(j))).max().unwrap_or(0)
        };
        // The natural numbering joins 0 to 39 and 38 (bandwidth 39); the new order walks the ring's two arms side by
        // side, four nodes a level (two each side), so a neighbour is at most two levels' worth away.
        assert_eq!(band(&|v| v), 39);
        assert!(band(&|v| at[v]) <= 7, "{}", band(&|v| at[v]));
    }

    #[test]
    fn two_components_and_a_lone_node_are_all_ordered() {
        let mut g = ring(6, 1);
        g.extend(vec![vec![]]);
        let mut h = ring(5, 1);
        for list in &mut h {
            for v in list.iter_mut() {
                *v += 7;
            }
        }
        g.extend(h);
        let order = reverse_cuthill_mckee(&g);
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn the_envelope_solve_and_inverse_are_the_dense_ones() {
        let g = ring(30, 3);
        let order = reverse_cuthill_mckee(&g);
        let mut at = vec![0usize; 30];
        for (p, &v) in order.iter().enumerate() {
            at[v] = p;
        }
        let m = matrix(&g);
        // Permuted: P·M·Pᵀ in the envelope, M itself dense.
        let mut first: Vec<usize> = (0..30).collect();
        for i in 0..30 {
            for &j in &g[i] {
                let (p, q) = (at[i], at[j]);
                if q < p {
                    first[p] = first[p].min(q);
                }
            }
        }
        let mut env = Envelope::new(first);
        let mut dense = Normal::new(30);
        for i in 0..30 {
            for j in 0..30 {
                if m[i][j] != 0.0 {
                    dense.add(i, j, m[i][j]);
                    if at[i] >= at[j] {
                        env.add(at[i], at[j], m[i][j]);
                    }
                }
            }
        }
        let b: Vec<f64> = (0..30).map(|i| (i as f64 * 0.37).sin()).collect();
        let pb: Vec<f64> = (0..30).map(|p| b[order[p]]).collect();
        let factor = env.cholesky().expect("positive definite");
        let x = factor.solve(&pb);
        let dense_factor = dense.cholesky().expect("positive definite");
        let want = dense_factor.solve(&b);
        for i in 0..30 {
            assert!((x[at[i]] - want[i]).abs() < 1e-12, "x[{i}]");
        }
        let z = factor.inverse();
        let q = dense_factor.inverse();
        for i in 0..30 {
            assert!((z.get(at[i], at[i]) - Inverse::get(&q, i, i)).abs() < 1e-12, "Z[{i}][{i}]");
            for &j in &g[i] {
                assert!((z.get(at[i], at[j]) - Inverse::get(&q, i, j)).abs() < 1e-12, "Z[{i}][{j}]");
            }
        }
    }

    #[test]
    fn a_floating_part_fails_at_its_pivot() {
        // Two nodes joined, neither held: singular (the second pivot is 0).
        let mut env = Envelope::new(vec![0, 0]);
        env.add(0, 0, 1.0);
        env.add(1, 0, -1.0);
        env.add(1, 1, 1.0);
        assert_eq!(env.cholesky().err(), Some(1));
    }
}
