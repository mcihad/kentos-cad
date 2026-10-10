//! İkili karşılaştırma (docs/adr/0237 §7): Saaty's analytic hierarchy
//! process. The criteria are compared two by two on the 1–9 scale; the
//! weights are the comparison matrix's principal eigenvector (power
//! iteration), its consistency the ratio of the consistency index to
//! Saaty's random index.

/// Saaty's random index for 1 … 15 criteria (1980).
const RANDOM_INDEX: [f64; 15] = [
    0.0, 0.0, 0.58, 0.90, 1.12, 1.24, 1.32, 1.41, 1.45, 1.49, 1.51, 1.48, 1.56, 1.57, 1.59,
];
/// The most criteria.
pub const MOST_CRITERIA: usize = 15;
/// The power iteration stops when no weight moves more than this…
const SETTLED: f64 = 1e-15;
/// …or after this many steps.
const MOST_STEPS: u32 = 10_000;
/// A consistency ratio above this is inconsistent.
pub const CONSISTENT: f64 = 0.10;

/// The weights and how consistent the comparisons were.
#[derive(Clone, Debug, PartialEq)]
pub struct Pairwise {
    /// In the criteria's order; their sum is 1.
    pub weights: Vec<f64>,
    /// The principal eigenvalue λ.
    pub lambda: f64,
    pub ci: f64,
    /// Saaty's random index (0 for two criteria).
    pub ri: f64,
    pub cr: f64,
    pub steps: u32,
}

/// A comparison's value checked: 9 … 2 (the first over the second), 1, −2 … −9.
fn checked(a: &str, b: &str, v: f64) -> Result<i32, String> {
    let ok = v.fract() == 0.0 && (v == 1.0 || (2.0..=9.0).contains(&v.abs()));
    if ok {
        Ok(v as i32)
    } else {
        Err(format!(
            "{a} — {b}: karşılaştırma 1, 2 … 9 ya da −2 … −9 olmalı."
        ))
    }
}

/// The comparison matrix of `names` (row by row): pair (a, b, v) gives the
/// first named over the second v times (v ≥ 1), or the second over the
/// first |v| times (v ≤ −2); (b, a, v) is (a, b, −v); a pair given twice
/// counts once, the first; a pair not given is 1.
pub fn matrix(names: &[String], pairs: &[(String, String, f64)]) -> Result<Vec<f64>, String> {
    let n = names.len();
    if !(2..=MOST_CRITERIA).contains(&n) {
        return Err(format!(
            "İkili karşılaştırma 2 ile {MOST_CRITERIA} arasında ölçüt (raster) ister; {n} raster seçili."
        ));
    }
    let mut a = vec![1.0; n * n];
    let mut set = vec![false; n * n];
    for (x, y, v) in pairs {
        let (Some(i), Some(j)) = (
            names.iter().position(|m| m == x),
            names.iter().position(|m| m == y),
        ) else {
            continue;
        };
        if i == j {
            continue;
        }
        let v = checked(x, y, *v)?;
        // Kept as the pair in the criteria's order: i < j.
        let (i, j, v) = if i < j {
            (i, j, v)
        } else {
            (j, i, if v == 1 { 1 } else { -v })
        };
        if set[i * n + j] {
            continue;
        }
        set[i * n + j] = true;
        // The whole number on one side, its reciprocal on the other.
        let (aij, aji) = if v >= 1 {
            (f64::from(v), 1.0 / f64::from(v))
        } else {
            (1.0 / f64::from(-v), f64::from(-v))
        };
        a[i * n + j] = aij;
        a[j * n + i] = aji;
    }
    Ok(a)
}

/// The weights of the criteria `names` and their consistency (§7).
pub fn pairwise(names: &[String], pairs: &[(String, String, f64)]) -> Result<Pairwise, String> {
    let a = matrix(names, pairs)?;
    let n = names.len();
    let mut w = vec![1.0 / n as f64; n];
    let mut y = vec![0.0; n];
    let mut steps = 0;
    while steps < MOST_STEPS {
        steps += 1;
        for (i, yi) in y.iter_mut().enumerate() {
            let row = &a[i * n..(i + 1) * n];
            *yi = row.iter().zip(&w).fold(0.0, |s, (aij, wj)| s + aij * wj);
        }
        let s = y.iter().fold(0.0, |s, v| s + v);
        let mut moved: f64 = 0.0;
        for (wi, yi) in w.iter_mut().zip(&y) {
            let next = yi / s;
            let d = (next - *wi).abs();
            if d > moved {
                moved = d;
            }
            *wi = next;
        }
        if moved <= SETTLED {
            break;
        }
    }
    let lambda = (0..n)
        .map(|i| {
            let row = &a[i * n..(i + 1) * n];
            row.iter().zip(&w).fold(0.0, |s, (aij, wj)| s + aij * wj)
        })
        .fold(0.0, |s, v| s + v);
    let (ci, ri, cr) = if n <= 2 {
        (0.0, 0.0, 0.0)
    } else {
        let ci = (lambda - n as f64) / (n as f64 - 1.0);
        let ri = RANDOM_INDEX[n - 1];
        (ci, ri, ci / ri)
    };
    Ok(Pairwise {
        weights: w,
        lambda,
        ci,
        ri,
        cr,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_consistent_matrix_gives_its_ratios() {
        // A = 2B = 4C: weights 4/7, 2/7, 1/7, λ = 3, CR = 0.
        let p = pairwise(
            &names(&["A", "B", "C"]),
            &[
                ("A".into(), "B".into(), 2.0),
                ("A".into(), "C".into(), 4.0),
                ("C".into(), "B".into(), -2.0),
            ],
        )
        .unwrap();
        for (w, want) in p.weights.iter().zip([4.0 / 7.0, 2.0 / 7.0, 1.0 / 7.0]) {
            assert!((w - want).abs() < 1e-15, "{w} {want}");
        }
        assert!((p.lambda - 3.0).abs() < 1e-12);
        assert!(p.cr.abs() < 1e-12);
    }

    #[test]
    fn pairs_read_both_ways_and_first_wins() {
        let n = names(&["A", "B"]);
        let a = matrix(
            &n,
            &[("B".into(), "A".into(), 3.0), ("A".into(), "B".into(), 5.0)],
        )
        .unwrap();
        assert_eq!(a, vec![1.0, 1.0 / 3.0, 3.0, 1.0]);
        let p = pairwise(&n, &[]).unwrap();
        assert_eq!(p.weights, vec![0.5, 0.5]);
        assert_eq!((p.ci, p.cr), (0.0, 0.0));
        assert!(matrix(&n, &[("A".into(), "B".into(), 1.5)]).is_err());
        assert!(matrix(&n, &[("A".into(), "B".into(), -1.0)]).is_err());
        assert!(matrix(&names(&["A"]), &[]).is_err());
    }
}
