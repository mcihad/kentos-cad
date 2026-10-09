//! Small dense systems (docs/adr/0232 §8, §9): LU with partial pivoting
//! for Spline's and Kriging's equations of a cell's neighbours. A pivot
//! below 10⁻¹³ of the matrix's largest entry is taken as no solution
//! (neighbours on a line): the cell is left empty.

/// The relative size below which a pivot counts as zero.
pub const PIVOT: f64 = 1e-13;

/// An n × n matrix factored: L and U in place, row-major, and the rows' order.
#[derive(Clone, Debug, Default)]
pub struct Lu {
    n: usize,
    lu: Vec<f64>,
    piv: Vec<usize>,
}

impl Lu {
    /// The factors of `a` (n × n, row-major), or none when it is singular by the pivot rule.
    pub fn factor(mut a: Vec<f64>, n: usize) -> Option<Lu> {
        if a.len() != n * n {
            return None;
        }
        let largest = a.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        if !(largest > 0.0) || !largest.is_finite() {
            return None;
        }
        let tiny = PIVOT * largest;
        let mut piv: Vec<usize> = (0..n).collect();
        for k in 0..n {
            // The largest entry of column k at or below the diagonal (the first of equals).
            let mut p = k;
            let mut best = a[k * n + k].abs();
            for r in k + 1..n {
                let v = a[r * n + k].abs();
                if v > best {
                    best = v;
                    p = r;
                }
            }
            if !(best > tiny) {
                return None;
            }
            if p != k {
                for c in 0..n {
                    a.swap(k * n + c, p * n + c);
                }
                piv.swap(k, p);
            }
            let d = a[k * n + k];
            for r in k + 1..n {
                let f = a[r * n + k] / d;
                a[r * n + k] = f;
                if f != 0.0 {
                    for c in k + 1..n {
                        a[r * n + c] -= f * a[k * n + c];
                    }
                }
            }
        }
        Some(Lu { n, lu: a, piv })
    }

    pub fn n(&self) -> usize {
        self.n
    }

    /// x of A·x = b, written over `b` (`tmp` as long, reused).
    pub fn solve(&self, b: &mut [f64], tmp: &mut Vec<f64>) {
        let n = self.n;
        tmp.clear();
        tmp.extend(self.piv.iter().map(|&p| b[p]));
        for r in 0..n {
            let row = &self.lu[r * n..r * n + r];
            let s = row
                .iter()
                .zip(&tmp[..r])
                .fold(tmp[r], |s, (l, t)| s - l * t);
            tmp[r] = s;
        }
        for r in (0..n).rev() {
            let row = &self.lu[r * n + r + 1..(r + 1) * n];
            let s = row
                .iter()
                .zip(&tmp[r + 1..n])
                .fold(tmp[r], |s, (u, t)| s - u * t);
            tmp[r] = s / self.lu[r * n + r];
        }
        b[..n].copy_from_slice(&tmp[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_and_refuses() {
        let a = vec![2.0, 1.0, 1.0, 1.0, 3.0, 2.0, 1.0, 0.0, 0.0];
        let lu = Lu::factor(a, 3).expect("regular");
        let mut b = vec![4.0, 5.0, 6.0];
        lu.solve(&mut b, &mut Vec::new());
        // 2x + y + z = 4, x + 3y + 2z = 5, x = 6.
        assert!((b[0] - 6.0).abs() < 1e-12);
        assert!((2.0 * b[0] + b[1] + b[2] - 4.0).abs() < 1e-12);
        assert!((b[0] + 3.0 * b[1] + 2.0 * b[2] - 5.0).abs() < 1e-12);
        assert!(Lu::factor(vec![1.0, 2.0, 2.0, 4.0], 2).is_none());
        assert!(Lu::factor(vec![0.0; 4], 2).is_none());
    }
}
