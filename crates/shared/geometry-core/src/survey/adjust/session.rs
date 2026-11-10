//! A GNSS network's baseline sessions (docs/adr/4920): baselines processed together are correlated, and GVX 1.0 gives
//! the cross-covariances between them. A session's 3n × 3n covariance (the baselines' own 3×3 blocks on the diagonal,
//! the cross blocks off it, all times the variance factor) is factored L·Lᵀ whole and its rows whitened by L⁻¹, so the
//! normal equations take the session's full weight P = Σ⁻¹. A whitened row mixes the session's baselines, so with
//! correlations a baseline is tested component by component on its own observations: Baarda's w-test for correlated
//! observations, w = |eᵢᵀPv| / √(eᵢᵀ·P·Qvv·P·eᵢ) with the a priori unit variance, its redundancy number rᵢ = (Qvv·P)ᵢᵢ
//! (Teunissen, Testing Theory, 2006; Baarda 1968). The independent reference is
//! `scripts/fixtures/map_survey_network_cases.py` (numpy's dense GLS).

/// A session: its name (for the words), its baselines (indices into the network's) and the cross-covariance between
/// each pair of them, 3×3 row by row (m², before the variance factor): the row baseline's X, Y, Z against the column
/// baseline's.
#[derive(Clone, Debug, PartialEq)]
pub struct GnssSession {
    pub name: String,
    pub baselines: Vec<usize>,
    pub cross: Vec<(usize, usize, [f64; 9])>,
}

/// A group of baselines whitened together: its baselines and L⁻¹ of its covariance, 3m × 3m row by row (lower).
#[derive(Clone, Debug)]
pub(crate) struct Group {
    pub baselines: Vec<usize>,
    pub w: Vec<f64>,
}

impl Group {
    pub fn size(&self) -> usize {
        3 * self.baselines.len()
    }

    pub fn at(&self, r: usize, c: usize) -> f64 {
        self.w[r * self.size() + c]
    }
}

/// The lower Cholesky factor of a dense symmetric `n` × `n` matrix (row by row), none when it is not positive definite.
pub(crate) fn cholesky(a: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut l = vec![0.0; n * n];
    for j in 0..n {
        let mut d = a[j * n + j];
        for k in 0..j {
            d -= l[j * n + k] * l[j * n + k];
        }
        if !(d > 0.0) {
            return None;
        }
        let d = d.sqrt();
        l[j * n + j] = d;
        for i in j + 1..n {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= l[i * n + k] * l[j * n + k];
            }
            l[i * n + j] = s / d;
        }
    }
    Some(l)
}

/// The inverse of a lower triangular `n` × `n` matrix (row by row), by forward substitution on the unit vectors.
pub(crate) fn lower_inverse(l: &[f64], n: usize) -> Vec<f64> {
    let mut w = vec![0.0; n * n];
    for c in 0..n {
        for r in c..n {
            let mut s = if r == c { 1.0 } else { 0.0 };
            for k in c..r {
                s -= l[r * n + k] * w[k * n + c];
            }
            w[r * n + c] = s / l[r * n + r];
        }
    }
    w
}

/// A group's L⁻¹ from its baselines' own covariances and its cross blocks, times `scale`; none when the whole is not
/// positive definite.
pub(crate) fn whitener(own: &[[f64; 6]], cross: &[(usize, usize, [f64; 9])], scale: f64) -> Option<Vec<f64>> {
    let n = 3 * own.len();
    let mut a = vec![0.0; n * n];
    for (k, c) in own.iter().enumerate() {
        let full = [[c[0], c[1], c[2]], [c[1], c[3], c[4]], [c[2], c[4], c[5]]];
        for r in 0..3 {
            for q in 0..3 {
                a[(3 * k + r) * n + 3 * k + q] = full[r][q] * scale;
            }
        }
    }
    for &(p, q, b) in cross {
        for r in 0..3 {
            for c in 0..3 {
                let v = b[3 * r + c] * scale;
                a[(3 * p + r) * n + 3 * q + c] = v;
                a[(3 * q + c) * n + 3 * p + r] = v;
            }
        }
    }
    cholesky(&a, n).map(|l| lower_inverse(&l, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_whitener_inverts_the_factor() {
        let own = [[4e-6, 1e-6, 0.0, 9e-6, 0.0, 16e-6], [4e-6, 0.0, 0.0, 4e-6, 0.0, 4e-6]];
        let cross = [(0usize, 1usize, [1e-6, 0.0, 0.0, 0.0, 2e-6, 0.0, 0.0, 0.0, 1e-6])];
        let w = whitener(&own, &cross, 2.0).expect("positive definite");
        // W·Σ·Wᵀ = I.
        let n = 6;
        let mut a = vec![0.0; n * n];
        for (k, c) in own.iter().enumerate() {
            let full = [[c[0], c[1], c[2]], [c[1], c[3], c[4]], [c[2], c[4], c[5]]];
            for r in 0..3 {
                for q in 0..3 {
                    a[(3 * k + r) * n + 3 * k + q] = 2.0 * full[r][q];
                }
            }
        }
        for r in 0..3 {
            for c in 0..3 {
                a[r * n + 3 + c] = 2.0 * cross[0].2[3 * r + c];
                a[(3 + c) * n + r] = 2.0 * cross[0].2[3 * r + c];
            }
        }
        for i in 0..n {
            for j in 0..n {
                let mut s = 0.0;
                for p in 0..n {
                    for q in 0..n {
                        s += w[i * n + p] * a[p * n + q] * w[j * n + q];
                    }
                }
                assert!((s - if i == j { 1.0 } else { 0.0 }).abs() < 1e-9, "{i} {j}: {s}");
            }
        }
        // Correlations too strong for the deviations: not positive definite.
        let bad = [(0usize, 1usize, [4e-6, 0.0, 0.0, 0.0, 6e-6, 0.0, 0.0, 0.0, 8e-6])];
        assert!(whitener(&own, &bad, 1.0).is_none());
    }
}
