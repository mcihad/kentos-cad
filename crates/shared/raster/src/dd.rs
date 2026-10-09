//! Double-double numbers (docs/adr/0233 §9): a value as the unevaluated sum
//! `hi + lo` of two float64s, |lo| ≤ ½ ulp(hi), about 106 bits. The
//! statistics add their values and squares in it, so that a sum, a mean
//! and a variance's cancellation lose nothing before the one rounding to
//! float64 at the end. Only float64's +, − and × (no fused multiply-add):
//! every target gives the same bits.

/// `hi + lo`, normalised.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dd {
    pub hi: f64,
    pub lo: f64,
}

/// a + b = s + e exactly (Knuth).
#[inline]
pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

/// a + b = s + e exactly, for |a| ≥ |b| (Dekker).
#[inline]
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    (s, b - (s - a))
}

/// Veltkamp's splitter for float64: 2²⁷ + 1.
const SPLIT: f64 = 134_217_729.0;

/// `a` as two halves of 26 bits each (Veltkamp).
#[inline]
fn split(a: f64) -> (f64, f64) {
    let t = SPLIT * a;
    let hi = t - (t - a);
    (hi, a - hi)
}

/// a · b = p + e exactly (Dekker; |a|, |b| below 2⁹⁹⁶).
#[inline]
pub fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    (p, ((ah * bh - p) + ah * bl + al * bh) + al * bl)
}

impl Dd {
    pub const ZERO: Dd = Dd { hi: 0.0, lo: 0.0 };

    #[inline]
    pub fn from(x: f64) -> Dd {
        Dd { hi: x, lo: 0.0 }
    }

    /// x², exactly.
    #[inline]
    pub fn square(x: f64) -> Dd {
        let (hi, lo) = two_prod(x, x);
        Dd { hi, lo }
    }

    /// The nearest float64 (hi, the sum normalised).
    #[inline]
    pub fn value(self) -> f64 {
        self.hi
    }

    #[inline]
    pub fn add_f64(self, b: f64) -> Dd {
        let (s, e) = two_sum(self.hi, b);
        let (hi, lo) = fast_two_sum(s, e + self.lo);
        Dd { hi, lo }
    }

    /// The quotient by a float64 (two correction steps).
    #[inline]
    pub fn div_f64(self, b: f64) -> Dd {
        let q1 = self.hi / b;
        let (p, e) = two_prod(q1, b);
        let r = self - Dd { hi: p, lo: e };
        let q2 = r.hi / b;
        let (p, e) = two_prod(q2, b);
        let r = r - Dd { hi: p, lo: e };
        let q3 = r.hi / b;
        let (hi, lo) = fast_two_sum(q1, q2);
        Dd { hi, lo }.add_f64(q3)
    }
}

/// The sum of two, to about 106 bits (the accurate addition: the low parts summed apart).
impl std::ops::Add for Dd {
    type Output = Dd;

    #[inline]
    fn add(self, b: Dd) -> Dd {
        let (s, e) = two_sum(self.hi, b.hi);
        let (t, f) = two_sum(self.lo, b.lo);
        let (s, e) = fast_two_sum(s, e + t);
        let (hi, lo) = fast_two_sum(s, e + f);
        Dd { hi, lo }
    }
}

impl std::ops::Neg for Dd {
    type Output = Dd;

    #[inline]
    fn neg(self) -> Dd {
        Dd {
            hi: -self.hi,
            lo: -self.lo,
        }
    }
}

impl std::ops::Sub for Dd {
    type Output = Dd;

    #[inline]
    fn sub(self, b: Dd) -> Dd {
        std::ops::Add::add(self, -b)
    }
}

impl std::ops::Mul for Dd {
    type Output = Dd;

    #[inline]
    fn mul(self, b: Dd) -> Dd {
        let (p, e) = two_prod(self.hi, b.hi);
        let (hi, lo) = fast_two_sum(p, e + (self.hi * b.lo + self.lo * b.hi));
        Dd { hi, lo }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_keep_what_float64_drops() {
        // 1 + 2⁻⁶⁰ − 1 is 2⁻⁶⁰, which float64 alone loses.
        let tiny = 1.0 / (1u64 << 60) as f64;
        let s = Dd::from(1.0).add_f64(tiny).add_f64(-1.0);
        assert_eq!(s.value(), tiny);
        // (1 + 2⁻³⁰)² − 1 − 2⁻²⁹ = 2⁻⁶⁰ exactly.
        let x = 1.0 + 1.0 / (1u64 << 30) as f64;
        let d = Dd::square(x)
            .add_f64(-1.0)
            .add_f64(-2.0 / (1u64 << 30) as f64);
        assert_eq!(d.value(), tiny);
        // 1/3 to 106 bits: 3 · (1/3) − 1 well below 2⁻¹⁰⁰.
        let third = Dd::from(1.0).div_f64(3.0);
        let back = (third * Dd::from(3.0)).add_f64(-1.0);
        assert!(back.value().abs() < 1e-30, "{back:?}");
    }
}
