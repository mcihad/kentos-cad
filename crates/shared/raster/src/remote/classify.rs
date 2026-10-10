//! Denetimli sınıflandırma (docs/adr/0242 §6): each class's training cells
//! give its mean vector and sample covariance; a cell goes to the class of
//! the nearest mean (En yakın ortalama) or of the largest Gaussian
//! discriminant with equal priors, gᵢ(x) = −ln|Σᵢ| − (x − μᵢ)ᵀΣᵢ⁻¹(x − μᵢ)
//! (En büyük olabilirlik); on a tie the smaller value.
//!
//! The moments are Welford's over a row's cells, the rows' then joined in
//! the rows' order (Chan's formula): whatever the threads, the same sums.

// Vectors and matrices by their bands' indices read as the formulas do.
#![allow(clippy::needless_range_loop)]

use serde::Deserialize;

/// How a cell is assigned.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Method {
    /// En büyük olabilirlik.
    Likelihood,
    /// En yakın ortalama.
    Distance,
}

/// A class's training cells' moments: count, mean, co-moments (bands × bands).
#[derive(Clone, Debug, PartialEq)]
pub struct Moments {
    pub n: u64,
    pub mean: Vec<f64>,
    pub co: Vec<f64>,
}

impl Moments {
    pub fn new(bands: usize) -> Moments {
        Moments {
            n: 0,
            mean: vec![0.0; bands],
            co: vec![0.0; bands * bands],
        }
    }

    /// One more cell (Welford's update); `d` a scratch of the bands' length.
    pub fn push(&mut self, x: &[f64], d: &mut [f64]) {
        let b = self.mean.len();
        self.n += 1;
        let n = self.n as f64;
        for a in 0..b {
            d[a] = x[a] - self.mean[a];
            self.mean[a] += d[a] / n;
        }
        for a in 0..b {
            for c in 0..b {
                self.co[a * b + c] += d[a] * (x[c] - self.mean[c]);
            }
        }
    }

    /// Another set of cells joined after these (Chan's formula).
    pub fn join(&mut self, o: &Moments) {
        if o.n == 0 {
            return;
        }
        if self.n == 0 {
            *self = o.clone();
            return;
        }
        let b = self.mean.len();
        let (na, nb) = (self.n as f64, o.n as f64);
        let n = na + nb;
        let delta: Vec<f64> = (0..b).map(|a| o.mean[a] - self.mean[a]).collect();
        let f = na * nb / n;
        for a in 0..b {
            for c in 0..b {
                self.co[a * b + c] += o.co[a * b + c] + delta[a] * delta[c] * f;
            }
        }
        for a in 0..b {
            self.mean[a] += delta[a] * nb / n;
        }
        self.n += o.n;
    }

    /// The sample covariance (n − 1), symmetric from the lower half.
    pub fn covariance(&self) -> Vec<f64> {
        let b = self.mean.len();
        let div = (self.n.max(2) - 1) as f64;
        let mut s = vec![0.0; b * b];
        for a in 0..b {
            for c in 0..=a {
                let v = self.co[a * b + c] / div;
                s[a * b + c] = v;
                s[c * b + a] = v;
            }
        }
        s
    }
}

/// The lower Cholesky factor of `s` (bands × bands); none when a pivot's
/// rest is not above 10⁻¹² of its variance (the covariance is singular).
pub fn cholesky(s: &[f64], b: usize) -> Option<Vec<f64>> {
    let mut l = vec![0.0; b * b];
    for j in 0..b {
        let mut sum = 0.0;
        for k in 0..j {
            sum += l[j * b + k] * l[j * b + k];
        }
        let rest = s[j * b + j] - sum;
        if !(rest > 1e-12 * s[j * b + j]) {
            return None;
        }
        let d = rest.sqrt();
        l[j * b + j] = d;
        for i in j + 1..b {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i * b + k] * l[j * b + k];
            }
            l[i * b + j] = (s[i * b + j] - sum) / d;
        }
    }
    Some(l)
}

/// A class's model.
#[derive(Clone, Debug)]
struct Model {
    mean: Vec<f64>,
    /// The lower Cholesky factor of the covariance (En büyük olabilirlik).
    low: Vec<f64>,
    /// ln |Σ|.
    logdet: f64,
}

/// The classes' models.
#[derive(Clone, Debug)]
pub struct Classifier {
    method: Method,
    bands: usize,
    models: Vec<Model>,
}

impl Classifier {
    /// The models of the classes `names` from their training moments; why not, by the class's name.
    pub fn new(
        method: Method,
        names: &[String],
        moments: &[Moments],
    ) -> Result<Classifier, String> {
        let bands = moments.first().map_or(0, |m| m.mean.len());
        let mut models = Vec::with_capacity(names.len());
        for (name, m) in names.iter().zip(moments) {
            if method == Method::Likelihood && m.n < bands as u64 + 1 {
                return Err(format!(
                    "“{name}” sınıfının eğitim hücresi {}; en büyük olabilirlik en az {} ister.",
                    m.n,
                    bands + 1
                ));
            }
            if m.n == 0 {
                return Err(format!("“{name}” sınıfının eğitim hücresi yok."));
            }
            let (low, logdet) = if method == Method::Likelihood {
                let low = cholesky(&m.covariance(), bands).ok_or_else(|| {
                    format!(
                        "“{name}” sınıfının kovaryansı tekil: bantlarından biri sabit ya da bantları doğrusal bağımlı."
                    )
                })?;
                let mut logdet = 0.0;
                for j in 0..bands {
                    logdet += libm::log(low[j * bands + j]);
                }
                (low, 2.0 * logdet)
            } else {
                (Vec::new(), 0.0)
            };
            models.push(Model {
                mean: m.mean.clone(),
                low,
                logdet,
            });
        }
        Ok(Classifier {
            method,
            bands,
            models,
        })
    }

    /// The class (from 0) of the cell `x`; `z` a scratch of the bands' length.
    #[inline]
    pub fn assign(&self, x: &[f64], z: &mut [f64]) -> usize {
        let b = self.bands;
        let mut best = 0;
        let mut top = f64::NEG_INFINITY;
        for (c, m) in self.models.iter().enumerate() {
            let score = match self.method {
                Method::Distance => {
                    let mut s = 0.0;
                    for a in 0..b {
                        let e = x[a] - m.mean[a];
                        s += e * e;
                    }
                    -s
                }
                Method::Likelihood => {
                    // ‖L⁻¹(x − μ)‖² by forward substitution.
                    let mut q = 0.0;
                    for a in 0..b {
                        let mut v = x[a] - m.mean[a];
                        for k in 0..a {
                            v -= m.low[a * b + k] * z[k];
                        }
                        z[a] = v / m.low[a * b + a];
                        q += z[a] * z[a];
                    }
                    -m.logdet - q
                }
            };
            if c == 0 || score > top {
                best = c;
                top = score;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moments(cells: &[[f64; 2]]) -> Moments {
        let mut m = Moments::new(2);
        let mut d = [0.0; 2];
        for x in cells {
            m.push(x, &mut d);
        }
        m
    }

    #[test]
    fn welford_and_chan_agree_with_the_plain_sums() {
        let cells = [[1.0, 2.0], [3.0, 1.0], [2.0, 5.0], [4.0, 4.0], [0.0, 3.0]];
        let whole = moments(&cells);
        let mut joined = moments(&cells[..2]);
        joined.join(&moments(&cells[2..]));
        let s = whole.covariance();
        // Means 2 and 3; covariance [[2.5, 0.25], [0.25, 2.5]].
        assert!((whole.mean[0] - 2.0).abs() < 1e-15 && (whole.mean[1] - 3.0).abs() < 1e-15);
        for (got, want) in s.iter().zip([2.5, 0.25, 0.25, 2.5]) {
            assert!((got - want).abs() < 1e-14, "{got} vs {want}");
        }
        for (a, b) in joined.covariance().iter().zip(&s) {
            assert!((a - b).abs() < 1e-14);
        }
    }

    #[test]
    fn a_constant_band_is_singular() {
        let m = moments(&[[1.0, 7.0], [2.0, 7.0], [4.0, 7.0]]);
        assert!(cholesky(&m.covariance(), 2).is_none());
        let names = vec!["Düz".to_owned()];
        let e = Classifier::new(Method::Likelihood, &names, &[m]).unwrap_err();
        assert!(e.contains("tekil"), "{e}");
    }

    #[test]
    fn ties_go_to_the_smaller_value() {
        let a = moments(&[[0.0, 0.0], [2.0, 2.0]]);
        let b = moments(&[[4.0, 4.0], [6.0, 6.0]]);
        let names = vec!["A".to_owned(), "B".to_owned()];
        let c = Classifier::new(Method::Distance, &names, &[a, b]).unwrap();
        let mut z = [0.0; 2];
        assert_eq!(c.assign(&[3.0, 3.0], &mut z), 0);
        assert_eq!(c.assign(&[3.1, 3.0], &mut z), 1);
    }
}
