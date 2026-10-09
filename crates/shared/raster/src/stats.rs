//! The statistics of cell values (docs/adr/0233 §9, `kentos.rasterstats/1`):
//! Bölgesel istatistik's, Komşuluk istatistiği's and Hücre istatistiği's
//! numbers. Count, least and largest are exact; sum and mean are summed in
//! double-double and rounded once; the sample standard deviation (n − 1)
//! comes from the double-double sums of the values and their squares; the
//! median, majority, minority and variety from the values themselves.

use crate::dd::Dd;

/// The rule's version, written beside the results that depend on it.
pub const POLICY: &str = "kentos.rasterstats/1";

/// A statistic of a set of values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stat {
    Count,
    Sum,
    Mean,
    Min,
    Max,
    Range,
    Std,
    Median,
    Majority,
    Minority,
    Variety,
    /// The area of the cells with a value (zonal only: count × a cell's area).
    Area,
}

impl Stat {
    pub fn from_key(key: &str) -> Option<Stat> {
        Some(match key {
            "count" => Stat::Count,
            "sum" => Stat::Sum,
            "mean" => Stat::Mean,
            "min" => Stat::Min,
            "max" => Stat::Max,
            "range" => Stat::Range,
            "std" => Stat::Std,
            "median" => Stat::Median,
            "majority" => Stat::Majority,
            "minority" => Stat::Minority,
            "variety" => Stat::Variety,
            "area" => Stat::Area,
            _ => return None,
        })
    }

    pub fn key(self) -> &'static str {
        match self {
            Stat::Count => "count",
            Stat::Sum => "sum",
            Stat::Mean => "mean",
            Stat::Min => "min",
            Stat::Max => "max",
            Stat::Range => "range",
            Stat::Std => "std",
            Stat::Median => "median",
            Stat::Majority => "majority",
            Stat::Minority => "minority",
            Stat::Variety => "variety",
            Stat::Area => "area",
        }
    }

    /// Its name as the tables and summaries write it.
    pub fn label(self) -> &'static str {
        match self {
            Stat::Count => "Sayı",
            Stat::Sum => "Toplam",
            Stat::Mean => "Ortalama",
            Stat::Min => "En küçük",
            Stat::Max => "En büyük",
            Stat::Range => "Aralık",
            Stat::Std => "Standart sapma",
            Stat::Median => "Ortanca",
            Stat::Majority => "Çoğunluk",
            Stat::Minority => "Azınlık",
            Stat::Variety => "Çeşit",
            Stat::Area => "Alan",
        }
    }

    /// Worked out from the values themselves, not from their sums.
    pub fn orders(self) -> bool {
        matches!(
            self,
            Stat::Median | Stat::Majority | Stat::Minority | Stat::Variety
        )
    }

    /// A whole number (a count).
    pub fn whole(self) -> bool {
        matches!(self, Stat::Count | Stat::Variety)
    }
}

/// The sums a set of values is summed into: their count, the double-double
/// sums of the values and of their squares, the least and the largest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moments {
    pub n: u64,
    pub s1: Dd,
    pub s2: Dd,
    pub min: f64,
    pub max: f64,
}

impl Default for Moments {
    fn default() -> Moments {
        Moments {
            n: 0,
            s1: Dd::ZERO,
            s2: Dd::ZERO,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }
}

impl Moments {
    /// One more value (never NaN).
    #[inline]
    pub fn push(&mut self, x: f64) {
        self.n += 1;
        self.s1 = self.s1.add_f64(x);
        self.s2 = self.s2 + Dd::square(x);
        if x < self.min {
            self.min = x;
        }
        if x > self.max {
            self.max = x;
        }
    }

    /// Another set's sums joined to these (the caller keeps the order fixed).
    pub fn merge(&mut self, o: &Moments) {
        self.n += o.n;
        self.s1 = self.s1 + o.s1;
        self.s2 = self.s2 + o.s2;
        if o.min < self.min {
            self.min = o.min;
        }
        if o.max > self.max {
            self.max = o.max;
        }
    }

    pub fn sum(&self) -> Option<f64> {
        (self.n > 0).then(|| self.s1.value() + 0.0)
    }

    pub fn mean(&self) -> Option<f64> {
        (self.n > 0).then(|| self.s1.div_f64(self.n as f64).value() + 0.0)
    }

    pub fn min(&self) -> Option<f64> {
        (self.n > 0).then_some(self.min + 0.0)
    }

    pub fn max(&self) -> Option<f64> {
        (self.n > 0).then_some(self.max + 0.0)
    }

    pub fn range(&self) -> Option<f64> {
        (self.n > 0).then_some(self.max - self.min)
    }

    /// The sample standard deviation: √((S₂ − S₁²/n)/(n − 1)), the
    /// difference in double-double; none below two values.
    pub fn std(&self) -> Option<f64> {
        std_of(self.n, self.s1, self.s2)
    }

    /// A statistic these sums give (none for the order statistics and the area).
    pub fn stat(&self, s: Stat) -> Option<f64> {
        match s {
            Stat::Count => Some(self.n as f64),
            Stat::Sum => self.sum(),
            Stat::Mean => self.mean(),
            Stat::Min => self.min(),
            Stat::Max => self.max(),
            Stat::Range => self.range(),
            Stat::Std => self.std(),
            _ => None,
        }
    }
}

/// The sample standard deviation of `n` values from their sums.
pub fn std_of(n: u64, s1: Dd, s2: Dd) -> Option<f64> {
    if n < 2 {
        return None;
    }
    let nf = n as f64;
    let d = s2 - (s1 * s1).div_f64(nf);
    let var = d.div_f64(nf - 1.0).value();
    Some(if var > 0.0 { var.sqrt() } else { 0.0 })
}

/// The order statistics of `values` (sorted here, in place; none empty):
/// the median (the middle one, or the mean of the middle two), the most
/// and the least frequent value (ties: the smaller) and how many differ.
pub fn order_stat(values: &mut [f64], s: Stat) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let n = values.len();
    if s == Stat::Median {
        let (_, mid, _) = values.select_nth_unstable_by(n / 2, f64::total_cmp);
        let upper = *mid;
        if n % 2 == 1 {
            return Some(upper + 0.0);
        }
        let lower = values[..n / 2]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, |m, v| if v > m { v } else { m });
        // One rounding (the sum's; halving is exact), halves first where the sum overflows.
        let mean = (lower + upper) / 2.0;
        return Some(if mean.is_finite() {
            mean + 0.0
        } else {
            lower / 2.0 + upper / 2.0
        });
    }
    values.sort_unstable_by(f64::total_cmp);
    // Runs of equal values (−0 and 0 are one value).
    let mut variety = 0u64;
    let (mut best, mut best_n) = (f64::NAN, 0usize);
    let (mut least, mut least_n) = (f64::NAN, usize::MAX);
    let mut k = 0;
    while k < n {
        let v = values[k];
        let mut e = k + 1;
        while e < n && values[e] == v {
            e += 1;
        }
        let run = e - k;
        variety += 1;
        if run > best_n {
            best_n = run;
            best = v;
        }
        if run < least_n {
            least_n = run;
            least = v;
        }
        k = e;
    }
    Some(match s {
        Stat::Majority => best + 0.0,
        Stat::Minority => least + 0.0,
        Stat::Variety => variety as f64,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moments(values: &[f64]) -> Moments {
        let mut m = Moments::default();
        for &v in values {
            m.push(v);
        }
        m
    }

    #[test]
    fn the_sums_give_the_exact_figures() {
        let m = moments(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert_eq!(m.sum(), Some(40.0));
        assert_eq!(m.mean(), Some(5.0));
        // Σ(x − 5)² = 32, sample variance 32/7.
        assert_eq!(m.std(), Some((32.0f64 / 7.0).sqrt()));
        assert_eq!(
            (m.min(), m.max(), m.range()),
            (Some(2.0), Some(9.0), Some(7.0))
        );
        // A large mean and a small spread: float64's naive formula would cancel.
        let m = moments(&[1e9 + 1.0, 1e9 + 2.0, 1e9 + 3.0]);
        assert_eq!(m.std(), Some(1.0));
        assert_eq!(moments(&[3.0]).std(), None);
        assert_eq!(Moments::default().mean(), None);
    }

    #[test]
    fn order_statistics() {
        let v = [5.0, 1.0, 3.0, 3.0, 9.0, 1.0, 3.0];
        let of = |s| order_stat(&mut v.clone(), s);
        assert_eq!(of(Stat::Median), Some(3.0));
        assert_eq!(of(Stat::Majority), Some(3.0));
        // 5 and 9 once each: the smaller.
        assert_eq!(of(Stat::Minority), Some(5.0));
        assert_eq!(of(Stat::Variety), Some(4.0));
        assert_eq!(
            order_stat(&mut [4.0, 1.0, 3.0, 2.0], Stat::Median),
            Some(2.5)
        );
        // Ties for the most frequent: the smaller.
        assert_eq!(
            order_stat(&mut [2.0, 7.0, 7.0, 2.0], Stat::Majority),
            Some(2.0)
        );
        assert_eq!(order_stat(&mut [-0.0, 0.0], Stat::Variety), Some(1.0));
        assert_eq!(order_stat(&mut [], Stat::Median), None);
    }
}
