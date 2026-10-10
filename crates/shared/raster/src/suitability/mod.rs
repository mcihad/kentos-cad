//! Uygunluk analizi (docs/adr/0237): the cell rules of Bulanık üyelik,
//! Bulanık çakıştırma, Ağırlıklı toplam and Ağırlıklı çakıştırma, the
//! common grid of several inputs, İkili karşılaştırma's weights
//! ([`pairwise`]) and ROC ile doğrulama's counts ([`roc`]). The operation
//! job (`ops`) reads the inputs onto the grid a strip at a time and asks
//! these for each cell.

pub mod pairwise;
pub mod roc;

use std::collections::BTreeMap;

use crate::grid::Grid;
use crate::inputs::{Input, place_in, point_of};
use crate::reclass::{self, Bounds, Rule};

/// The largest spread, steepness or exponent (§3).
const MOST_PARAMETER: f64 = 1e6;
/// The largest weight of Ağırlıklı toplam (§5).
const MOST_WEIGHT: f64 = 1e9;
/// The scale's widest ends (§6).
const MOST_SCALE: f64 = 1e6;
/// An influence's units: per cent with four decimals (§6).
const INFLUENCE_UNITS: f64 = 1e4;
/// The influences' sum in those units: 100 %.
const INFLUENCE_WHOLE: i64 = 1_000_000;
/// A rule's new value `kısıt` (restricted), kept as +∞ among the numbers.
pub const RESTRICTED: f64 = f64::INFINITY;

/// Bulanık üyelik's function (§3), its settings checked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Membership {
    Linear { a: f64, b: f64 },
    Power { a: f64, b: f64, e: f64 },
    Gaussian { m: f64, s: f64 },
    Large { m: f64, s: f64 },
    Small { m: f64, s: f64 },
    Near { m: f64, s: f64 },
}

fn finite(v: f64, what: &str) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{what} bir sayı olmalı."))
    }
}

fn positive(v: f64, what: &str) -> Result<f64, String> {
    if v > 0.0 && v < MOST_PARAMETER {
        Ok(v)
    } else {
        Err(format!("{what} 0'dan büyük, 10⁶'dan küçük olmalı."))
    }
}

impl Membership {
    /// The function `name` with its settings, checked (§3).
    pub fn new(
        name: &str,
        (low, high, exponent): (f64, f64, f64),
        (midpoint, spread, steep): (f64, f64, f64),
    ) -> Result<Membership, String> {
        let ends = || -> Result<(f64, f64), String> {
            let (a, b) = (finite(low, "Alt değer")?, finite(high, "Üst değer")?);
            if a == b {
                return Err("Alt ve üst değer aynı olamaz.".into());
            }
            Ok((a, b))
        };
        let m = || finite(midpoint, "Orta nokta");
        let above = |m: f64| {
            if m > 0.0 {
                Ok(m)
            } else {
                Err("Büyük ve Küçük'te orta nokta 0'dan büyük olmalı.".to_owned())
            }
        };
        Ok(match name {
            "linear" => {
                let (a, b) = ends()?;
                Membership::Linear { a, b }
            }
            "power" => {
                let (a, b) = ends()?;
                Membership::Power {
                    a,
                    b,
                    e: positive(exponent, "Üs")?,
                }
            }
            "gaussian" => Membership::Gaussian {
                m: m()?,
                s: positive(spread, "Yayılım")?,
            },
            "large" => Membership::Large {
                m: above(m()?)?,
                s: positive(steep, "Diklik")?,
            },
            "small" => Membership::Small {
                m: above(m()?)?,
                s: positive(steep, "Diklik")?,
            },
            "near" => Membership::Near {
                m: m()?,
                s: positive(spread, "Yayılım")?,
            },
            other => return Err(format!("Üyelik işlevi bilinmiyor: {other}.")),
        })
    }

    /// The membership of a value (not NaN).
    #[inline]
    pub fn of(&self, x: f64) -> f64 {
        match *self {
            Membership::Linear { a, b } => linear(a, b, x),
            Membership::Power { a, b, e } => libm::pow(linear(a, b, x), e),
            Membership::Gaussian { m, s } => {
                let d = x - m;
                libm::exp(-(s * (d * d)))
            }
            Membership::Large { m, s } => {
                if x <= 0.0 {
                    0.0
                } else {
                    1.0 / (1.0 + libm::pow(x / m, -s))
                }
            }
            Membership::Small { m, s } => {
                if x <= 0.0 {
                    1.0
                } else {
                    1.0 / (1.0 + libm::pow(x / m, s))
                }
            }
            Membership::Near { m, s } => {
                let d = x - m;
                1.0 / (1.0 + s * (d * d))
            }
        }
    }
}

/// Doğrusal's membership: rising from a to b, or falling when a > b.
#[inline]
fn linear(a: f64, b: f64, x: f64) -> f64 {
    if a < b {
        if x <= a {
            0.0
        } else if x >= b {
            1.0
        } else {
            (x - a) / (b - a)
        }
    } else if x <= b {
        1.0
    } else if x >= a {
        0.0
    } else {
        (a - x) / (a - b)
    }
}

/// Bulanık çakıştırma's operator (§4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FuzzyOp {
    And,
    Or,
    Product,
    Sum,
    Gamma(f64),
}

impl FuzzyOp {
    pub fn new(name: &str, gamma: f64) -> Result<FuzzyOp, String> {
        Ok(match name {
            "and" => FuzzyOp::And,
            "or" => FuzzyOp::Or,
            "product" => FuzzyOp::Product,
            "sum" => FuzzyOp::Sum,
            "gamma" => {
                if !(0.0..=1.0).contains(&gamma) {
                    return Err("Gamma 0 ile 1 arasında olmalı.".into());
                }
                FuzzyOp::Gamma(gamma)
            }
            other => return Err(format!("Bulanık işleç bilinmiyor: {other}.")),
        })
    }

    /// The memberships (each 0–1, in the inputs' order) combined.
    #[inline]
    pub fn combine(&self, mu: &[f64]) -> f64 {
        match *self {
            FuzzyOp::And => mu
                .iter()
                .copied()
                .fold(f64::INFINITY, |m, v| if v < m { v } else { m }),
            FuzzyOp::Or => mu
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, |m, v| if v > m { v } else { m }),
            FuzzyOp::Product => mu.iter().fold(1.0, |p, v| p * v),
            FuzzyOp::Sum => 1.0 - mu.iter().fold(1.0, |q, v| q * (1.0 - v)),
            FuzzyOp::Gamma(g) => {
                let p = mu.iter().fold(1.0, |p, v| p * v);
                let q = mu.iter().fold(1.0, |q, v| q * (1.0 - v));
                libm::pow(1.0 - q, g) * libm::pow(p, 1.0 - g)
            }
        }
    }
}

/// Ağırlıklı toplam's weights in the inputs' order (§5): a raster not named weighs 1.
pub fn sum_weights(names: &[String], weights: &BTreeMap<String, f64>) -> Result<Vec<f64>, String> {
    names
        .iter()
        .map(|n| match weights.get(n) {
            None => Ok(1.0),
            Some(&w) if w.is_finite() && w.abs() <= MOST_WEIGHT => Ok(w),
            Some(_) => Err(format!(
                "{n}: ağırlık −10⁹ ile 10⁹ arasında bir sayı olmalı."
            )),
        })
        .collect()
}

/// A cell's scale value for one input (§6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scaled {
    Value(i64),
    Restricted,
    /// No value: the input's own, or `boş`.
    Empty,
    /// The table has no rule that holds.
    Unmatched,
    /// Not a whole number on the scale.
    Outside,
}

/// Ağırlıklı çakıştırma's settings, checked (§6).
#[derive(Clone, Debug, PartialEq)]
pub struct Overlay {
    pub lo: i64,
    pub hi: i64,
    /// The influences in units of 10⁻⁴ %.
    pub weights: Vec<i64>,
    pub tables: Vec<Option<Vec<Rule>>>,
    pub bounds: Bounds,
}

/// The influences' sum written for a message: per cent, at most four decimals (a sum is never below 0).
fn percent_text(units: i64) -> String {
    let (whole, part) = (units / 10_000, units % 10_000);
    if part == 0 {
        format!("{whole}")
    } else {
        format!("{whole}.{}", format!("{part:04}").trim_end_matches('0'))
    }
}

impl Overlay {
    pub fn new(
        names: &[String],
        (lo, hi): (f64, f64),
        influence: &BTreeMap<String, f64>,
        classes: &BTreeMap<String, String>,
        bounds: Bounds,
    ) -> Result<Overlay, String> {
        let whole = |v: f64, what: &str| {
            if v.is_finite() && v.fract() == 0.0 && v.abs() <= MOST_SCALE {
                Ok(v as i64)
            } else {
                Err(format!(
                    "Ölçeğin {what} −10⁶ ile 10⁶ arasında bir tam sayı olmalı."
                ))
            }
        };
        let (lo, hi) = (whole(lo, "alt ucu")?, whole(hi, "üst ucu")?);
        if lo >= hi {
            return Err("Ölçeğin alt ucu üst ucundan küçük olmalı.".into());
        }
        let mut weights = Vec::with_capacity(names.len());
        for n in names {
            let w = *influence
                .get(n)
                .ok_or_else(|| format!("{n}: etkisi yazılmadı."))?;
            if !(w.is_finite() && (0.0..=100.0).contains(&w)) {
                return Err(format!("{n}: etki 0 ile 100 arasında olmalı."));
            }
            let units = w * INFLUENCE_UNITS;
            let rounded = units.round();
            if (units - rounded).abs() > 1e-6 {
                return Err(format!("{n}: etki en çok dört ondalıklı olmalı."));
            }
            weights.push(rounded as i64);
        }
        let total: i64 = weights.iter().sum();
        if total != INFLUENCE_WHOLE {
            return Err(format!(
                "Etkilerin toplamı 100 olmalı; şimdi {}.",
                percent_text(total)
            ));
        }
        let mut tables = Vec::with_capacity(names.len());
        for n in names {
            let text = classes.get(n).map(|t| t.trim()).unwrap_or("");
            if text.is_empty() {
                tables.push(None);
                continue;
            }
            let rules = reclass::parse_with(text, &[("kısıt", RESTRICTED), ("kisit", RESTRICTED)])
                .map_err(|e| format!("{n}: {e}"))?;
            for (k, rule) in rules.iter().enumerate() {
                let new = match *rule {
                    Rule::Range { new, .. } | Rule::Value { new, .. } | Rule::Empty { new } => new,
                };
                if let Some(v) = new
                    && v != RESTRICTED
                    && (v.fract() != 0.0 || v < lo as f64 || v > hi as f64)
                {
                    return Err(format!(
                        "{n}: {}. kuralda yeni değer {} ölçeğin dışında ({lo}–{hi}).",
                        k + 1,
                        number_text(v)
                    ));
                }
            }
            tables.push(Some(rules));
        }
        Ok(Overlay {
            lo,
            hi,
            weights,
            tables,
            bounds,
        })
    }

    /// Input `k`'s scale value of a cell whose value is `x` (NaN: none).
    #[inline]
    pub fn scaled(&self, k: usize, x: f64) -> Scaled {
        match &self.tables[k] {
            Some(rules) => match reclass::apply(rules, self.bounds, x) {
                None => Scaled::Unmatched,
                Some(None) => Scaled::Empty,
                Some(Some(v)) if v == RESTRICTED => Scaled::Restricted,
                Some(Some(v)) => Scaled::Value(v as i64),
            },
            None if x.is_nan() => Scaled::Empty,
            None if x.fract() != 0.0 || x < self.lo as f64 || x > self.hi as f64 => Scaled::Outside,
            None => Scaled::Value(x as i64),
        }
    }

    /// The weighted scale values' sum, rounded half away from zero (the units' 10⁶).
    #[inline]
    pub fn class_of(&self, sum: i64) -> i64 {
        if sum >= 0 {
            (sum + INFLUENCE_WHOLE / 2) / INFLUENCE_WHOLE
        } else {
            -((-sum + INFLUENCE_WHOLE / 2) / INFLUENCE_WHOLE)
        }
    }
}

/// A number as a message writes it: whole numbers without decimals.
pub fn number_text(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// The finest input's lattice over the inputs' common area (§2): the cells
/// whose centres fall inside every input's box.
pub fn common_grid(inputs: &[Input]) -> Result<Grid, String> {
    let area = |k: usize| {
        let [_, a, b, _, c, d] = inputs[k].affine;
        (a * d - b * c).abs()
    };
    let mut base = 0;
    for k in 1..inputs.len() {
        if area(k) < area(base) {
            base = k;
        }
    }
    let g = inputs[base].grid();
    let (mut u0, mut u1, mut v0, mut v1) = (
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
    );
    for input in inputs {
        let (w, h) = (f64::from(input.width), f64::from(input.height));
        let (mut umin, mut umax, mut vmin, mut vmax) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for (i, j) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
            let (x, y) = point_of(&input.affine, i, j);
            let (u, v) = place_in(&g.affine, x, y);
            umin = umin.min(u);
            umax = umax.max(u);
            vmin = vmin.min(v);
            vmax = vmax.max(v);
        }
        u0 = u0.max(umin);
        u1 = u1.min(umax);
        v0 = v0.max(vmin);
        v1 = v1.min(vmax);
    }
    let (i0, i1) = ((u0 - 0.5 - 1e-9).ceil(), (u1 - 0.5 + 1e-9).floor());
    let (j0, j1) = ((v0 - 0.5 - 1e-9).ceil(), (v1 - 0.5 + 1e-9).floor());
    if !(i1 >= i0 && j1 >= j0) {
        return Err("Rasterler örtüşmüyor: ortak alanlarında hiç hücre merkezi yok.".into());
    }
    if i1 - i0 + 1.0 > f64::from(u32::MAX) || j1 - j0 + 1.0 > f64::from(u32::MAX) {
        return Err("Girdilerin ortak ızgarası kurulamadı.".into());
    }
    let (x0, y0) = point_of(&g.affine, i0, j0);
    let [_, a, b, _, c, d] = g.affine;
    Grid::of(
        [x0, a, b, y0, c, d],
        (i1 - i0 + 1.0) as u32,
        (j1 - j0 + 1.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memberships_follow_their_formulas() {
        let lin = Membership::new("linear", (0.0, 10.0, 2.0), (1.0, 0.1, 5.0)).unwrap();
        assert_eq!(lin.of(-1.0), 0.0);
        assert_eq!(lin.of(2.5), 0.25);
        assert_eq!(lin.of(10.0), 1.0);
        let down = Membership::new("linear", (10.0, 0.0, 2.0), (1.0, 0.1, 5.0)).unwrap();
        assert_eq!(down.of(2.5), 0.75);
        assert_eq!(down.of(0.0), 1.0);
        let pow = Membership::new("power", (0.0, 10.0, 2.0), (1.0, 0.1, 5.0)).unwrap();
        assert_eq!(pow.of(5.0), 0.25);
        let g = Membership::new("gaussian", (0.0, 1.0, 1.0), (3.0, 0.5, 5.0)).unwrap();
        assert_eq!(g.of(3.0), 1.0);
        let large = Membership::new("large", (0.0, 1.0, 1.0), (4.0, 0.1, 5.0)).unwrap();
        assert_eq!(large.of(4.0), 0.5);
        assert_eq!(large.of(0.0), 0.0);
        assert_eq!(large.of(-3.0), 0.0);
        let small = Membership::new("small", (0.0, 1.0, 1.0), (4.0, 0.1, 5.0)).unwrap();
        assert_eq!(small.of(4.0), 0.5);
        assert_eq!(small.of(-3.0), 1.0);
        let near = Membership::new("near", (0.0, 1.0, 1.0), (2.0, 1.0, 5.0)).unwrap();
        assert_eq!(near.of(3.0), 0.5);
        assert!(Membership::new("linear", (1.0, 1.0, 2.0), (1.0, 0.1, 5.0)).is_err());
        assert!(Membership::new("large", (0.0, 1.0, 1.0), (0.0, 0.1, 5.0)).is_err());
        assert!(Membership::new("gaussian", (0.0, 1.0, 1.0), (0.0, 0.0, 5.0)).is_err());
    }

    #[test]
    fn fuzzy_operators_combine_in_order() {
        let mu = [0.5, 0.25];
        assert_eq!(FuzzyOp::And.combine(&mu), 0.25);
        assert_eq!(FuzzyOp::Or.combine(&mu), 0.5);
        assert_eq!(FuzzyOp::Product.combine(&mu), 0.125);
        assert_eq!(FuzzyOp::Sum.combine(&mu), 1.0 - 0.5 * 0.75);
        assert_eq!(FuzzyOp::Gamma(1.0).combine(&mu), 0.625);
        assert_eq!(FuzzyOp::Gamma(0.0).combine(&mu), 0.125);
        assert!(FuzzyOp::new("gamma", 1.5).is_err());
    }

    #[test]
    fn overlay_checks_and_rounds() {
        let names = vec!["A".to_owned(), "B".to_owned()];
        let mut inf = BTreeMap::new();
        inf.insert("A".to_owned(), 33.3333);
        inf.insert("B".to_owned(), 66.6667);
        let mut classes = BTreeMap::new();
        classes.insert("A".to_owned(), "* 5 9; 5 * kısıt".to_owned());
        let o = Overlay::new(&names, (1.0, 9.0), &inf, &classes, Bounds::UpperClosed).unwrap();
        assert_eq!(o.weights, vec![333_333, 666_667]);
        assert_eq!(o.scaled(0, 3.0), Scaled::Value(9));
        assert_eq!(o.scaled(0, 7.0), Scaled::Restricted);
        assert_eq!(o.scaled(1, 4.0), Scaled::Value(4));
        assert_eq!(o.scaled(1, 4.5), Scaled::Outside);
        assert_eq!(o.scaled(1, 10.0), Scaled::Outside);
        assert_eq!(o.scaled(1, f64::NAN), Scaled::Empty);
        assert_eq!(o.class_of(6_500_000), 7);
        assert_eq!(o.class_of(6_499_999), 6);
        assert_eq!(o.class_of(-1_500_000), -2);
        inf.insert("B".to_owned(), 56.0);
        let e = Overlay::new(&names, (1.0, 9.0), &inf, &classes, Bounds::UpperClosed).unwrap_err();
        assert_eq!(e, "Etkilerin toplamı 100 olmalı; şimdi 89.3333.");
        inf.remove("B");
        assert_eq!(
            Overlay::new(&names, (1.0, 9.0), &inf, &classes, Bounds::UpperClosed).unwrap_err(),
            "B: etkisi yazılmadı."
        );
        inf.insert("B".to_owned(), 66.66667);
        assert!(
            Overlay::new(&names, (1.0, 9.0), &inf, &classes, Bounds::UpperClosed)
                .unwrap_err()
                .contains("dört ondalık")
        );
        inf.insert("B".to_owned(), 66.6667);
        classes.insert("B".to_owned(), "1 12".to_owned());
        assert_eq!(
            Overlay::new(&names, (1.0, 9.0), &inf, &classes, Bounds::UpperClosed).unwrap_err(),
            "B: 1. kuralda yeni değer 12 ölçeğin dışında (1–9)."
        );
    }
}
