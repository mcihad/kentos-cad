//! A surface's vertical accuracy from check points, beyond one RMSE (docs/adr/0945, plan/road.md Y39; ROAD-SURVEY-05):
//! the RMSE with its 95 % interval, and the robust measures Höhle and Höhle recommend for elevation models whose errors
//! are not normal (ISPRS J. 64, 2009): the median, the NMAD and the 68.3 % and 95 % quantiles of the absolute
//! differences; the skewness and kurtosis that say how far from normal the differences are; the points far from the
//! rest; and, with the check points' own accuracy, the two together as ASPRS's second edition asks.
//!
//! - RMSE = √(Σd²/n). With the differences normal and unbiased, n·RMSE²/σ² is χ² with n degrees of freedom: σ lies in
//!   [√(n·RMSE²/χ²₀.₉₇₅,ₙ), √(n·RMSE²/χ²₀.₀₂₅,ₙ)] at 95 %.
//! - NMAD = 1.4826 · median(|d − median(d)|): for normal differences, their standard deviation, but blind to the
//!   outliers that pull the RMSE.
//! - Quantiles by linear interpolation between order statistics (Hyndman and Fan's seventh, numpy's and R's default).
//! - Skewness g₁ = m₃/m₂^{3/2} and excess kurtosis g₂ = m₄/m₂² − 3 with the central moments mₖ = Σ(d − d̄)ᵏ/n; both 0 for
//!   normal differences.
//! - A point is far when |d − median| > 3 · NMAD. It is flagged, not left out.
//! - With the check points' own RMSE s, the two together √(RMSE² + s²).

/// The NMAD's factor: 1/Φ⁻¹(3/4), the normal's standard deviation from its median absolute deviation.
pub const NMAD: f64 = 1.4826;

/// How many NMADs from the median a point is far.
pub const FAR: f64 = 3.0;

/// A set of differences' accuracy.
#[derive(Clone, Debug, PartialEq)]
pub struct Accuracy {
    pub count: usize,
    pub mean: f64,
    pub rmse: f64,
    /// The RMSE's 95 % interval (the differences normal and unbiased).
    pub rmse_low: f64,
    pub rmse_high: f64,
    pub median: f64,
    pub nmad: f64,
    /// The 68.3 % and 95 % quantiles of |d|.
    pub q68: f64,
    pub q95: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    /// The indices of the far points.
    pub far: Vec<usize>,
    /// With the check points' own RMSE, the two together.
    pub combined: Option<f64>,
}

/// The `p` quantile of sorted values by linear interpolation between order statistics (type 7).
pub fn quantile(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    let h = (n - 1) as f64 * p.clamp(0.0, 1.0);
    let (lo, frac) = (h.floor() as usize, h - h.floor());
    let hi = (lo + 1).min(n - 1);
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}

fn sorted(v: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut out: Vec<f64> = v.collect();
    out.sort_by(f64::total_cmp);
    out
}

/// ln Γ(x) for x > 0 (Lanczos, g = 7, n = 9; about 15 digits).
fn ln_gamma(x: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let t = x + 7.5;
    let mut a = C[0];
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// The regularized lower incomplete gamma P(a, x): its series below a + 1, its continued fraction (Lentz) above.
fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let lead = a * x.ln() - x - ln_gamma(a);
    if x < a + 1.0 {
        let (mut sum, mut term, mut ap) = (1.0 / a, 1.0 / a, a);
        for _ in 0..10_000 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() < sum.abs() * 1e-16 {
                break;
            }
        }
        (sum.ln() + lead).exp()
    } else {
        let tiny = 1e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..10_000 {
            let an = -(i as f64) * (i as f64 - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-16 {
                break;
            }
        }
        1.0 - (h.ln() + lead).exp()
    }
}

/// The `p` quantile of χ² with `k` degrees of freedom: P(k/2, x/2) = p, by bisection to the last bits.
pub fn chi2_quantile(p: f64, k: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, k.max(1.0));
    while gamma_p(0.5 * k, 0.5 * hi) < p {
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if gamma_p(0.5 * k, 0.5 * mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo <= 1e-15 * hi {
            break;
        }
    }
    0.5 * (lo + hi)
}

/// The accuracy of `differences` (check point less surface, m), with the check points' own RMSE `survey` (m); none for
/// fewer than two.
pub fn accuracy(differences: &[f64], survey: Option<f64>) -> Option<Accuracy> {
    let n = differences.len();
    if n < 2 {
        return None;
    }
    let nf = n as f64;
    let mean = differences.iter().sum::<f64>() / nf;
    let rmse = (differences.iter().map(|d| d * d).sum::<f64>() / nf).sqrt();
    let ss = nf * rmse * rmse;
    let (rmse_low, rmse_high) = ((ss / chi2_quantile(0.975, nf)).sqrt(), (ss / chi2_quantile(0.025, nf)).sqrt());
    let by = sorted(differences.iter().copied());
    let median = quantile(&by, 0.5);
    let nmad = NMAD * quantile(&sorted(differences.iter().map(|d| (d - median).abs())), 0.5);
    let abs = sorted(differences.iter().map(|d| d.abs()));
    let moment = |k: i32| differences.iter().map(|d| (d - mean).powi(k)).sum::<f64>() / nf;
    let m2 = moment(2);
    // Equal differences have no spread: their mean's rounding leaves m₂ ~1e-36, not 0; within 1e-12 of RMSE² it is 0.
    let spread = m2 > 1e-12 * rmse * rmse;
    let (skewness, kurtosis) = if spread { (moment(3) / m2.powf(1.5), moment(4) / (m2 * m2) - 3.0) } else { (0.0, 0.0) };
    let far = differences
        .iter()
        .enumerate()
        .filter(|(_, d)| nmad > 0.0 && (*d - median).abs() > FAR * nmad)
        .map(|(i, _)| i)
        .collect();
    Some(Accuracy {
        count: n,
        mean,
        rmse,
        rmse_low,
        rmse_high,
        median,
        nmad,
        q68: quantile(&abs, 0.683),
        q95: quantile(&abs, 0.95),
        skewness,
        kurtosis,
        far,
        combined: survey.map(|s| (rmse * rmse + s * s).sqrt()),
    })
}
