//! The normal equations of an adjustment (docs/adr/0203 §4): a dense
//! symmetric matrix, its Cholesky factor with the ADR's pivot rule (a
//! diagonal not above 1e-10 of the column's own is a datum defect, named by
//! its unknown), solving and the inverse. Networks are small (at most
//! [`super::MAX_UNKNOWNS`] unknowns); a sparse solver is later work.

/// A pivot not above this share of its column's diagonal: the unknown is not determined.
const PIVOT: f64 = 1e-10;

/// A square symmetric matrix, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Normal {
    m: usize,
    a: Vec<f64>,
}

impl Normal {
    pub fn new(m: usize) -> Normal {
        Normal {
            m,
            a: vec![0.0; m * m],
        }
    }

    pub fn size(&self) -> usize {
        self.m
    }

    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.a[i * self.m + j]
    }

    pub fn add(&mut self, i: usize, j: usize, v: f64) {
        self.a[i * self.m + j] += v;
    }

    /// Its Cholesky factor, or the first unknown whose pivot is not above
    /// [`PIVOT`] times its own diagonal.
    pub fn cholesky(&self) -> Result<Factor, usize> {
        let m = self.m;
        let mut l = vec![0.0; m * m];
        for k in 0..m {
            let row_k = k * m;
            let mut s = self.a[row_k + k];
            for j in 0..k {
                s -= l[row_k + j] * l[row_k + j];
            }
            if !(s > PIVOT * self.a[row_k + k]) {
                return Err(k);
            }
            let d = s.sqrt();
            l[row_k + k] = d;
            for i in k + 1..m {
                let row_i = i * m;
                let mut t = self.a[row_i + k];
                for j in 0..k {
                    t -= l[row_i + j] * l[row_k + j];
                }
                l[row_i + k] = t / d;
            }
        }
        Ok(Factor { m, l })
    }
}

/// A lower triangular Cholesky factor L of N = L·Lᵀ, row by row.
#[derive(Clone, Debug)]
pub struct Factor {
    m: usize,
    l: Vec<f64>,
}

impl Factor {
    /// x with N·x = b.
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let m = self.m;
        let mut y = b.to_vec();
        for i in 0..m {
            let mut t = y[i];
            for j in 0..i {
                t -= self.l[i * m + j] * y[j];
            }
            y[i] = t / self.l[i * m + i];
        }
        for i in (0..m).rev() {
            let mut t = y[i];
            for j in i + 1..m {
                t -= self.l[j * m + i] * y[j];
            }
            y[i] = t / self.l[i * m + i];
        }
        y
    }

    /// N⁻¹ = L⁻ᵀ·L⁻¹.
    pub fn inverse(&self) -> Normal {
        let m = self.m;
        // L⁻¹, lower triangular, column by column.
        let mut inv = vec![0.0; m * m];
        for c in 0..m {
            inv[c * m + c] = 1.0 / self.l[c * m + c];
            for i in c + 1..m {
                let mut t = 0.0;
                for j in c..i {
                    t -= self.l[i * m + j] * inv[j * m + c];
                }
                inv[i * m + c] = t / self.l[i * m + i];
            }
        }
        let mut q = Normal::new(m);
        for i in 0..m {
            for j in 0..=i {
                let mut t = 0.0;
                for k in i..m {
                    t += inv[k * m + i] * inv[k * m + j];
                }
                q.a[i * m + j] = t;
                q.a[j * m + i] = t;
            }
        }
        q
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_system_solves_and_inverts() {
        let mut n = Normal::new(3);
        let rows = [[4.0, 2.0, 0.6], [2.0, 5.0, 1.0], [0.6, 1.0, 3.0]];
        for (i, r) in rows.iter().enumerate() {
            for (j, v) in r.iter().enumerate() {
                n.add(i, j, *v);
            }
        }
        let f = n.cholesky().expect("positive definite");
        let x = f.solve(&[1.0, 2.0, 3.0]);
        for (i, r) in rows.iter().enumerate() {
            let b: f64 = r.iter().zip(&x).map(|(a, x)| a * x).sum();
            assert!((b - [1.0, 2.0, 3.0][i]).abs() < 1e-14);
        }
        let q = f.inverse();
        for i in 0..3 {
            for j in 0..3 {
                let e: f64 = (0..3).map(|k| rows[i][k] * q.get(k, j)).sum();
                assert!((e - f64::from(u8::from(i == j))).abs() < 1e-14);
            }
        }
    }

    #[test]
    fn a_singular_system_names_its_first_free_unknown() {
        let mut n = Normal::new(2);
        for (i, j, v) in [(0, 0, 1.0), (0, 1, 1.0), (1, 0, 1.0), (1, 1, 1.0)] {
            n.add(i, j, v);
        }
        assert_eq!(n.cholesky().err(), Some(1));
    }
}
