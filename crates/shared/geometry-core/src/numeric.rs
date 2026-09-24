//! The numeric policy of cadastral values (CLAUDE.md §23): exact decimals,
//! rounding only by an explicit, versioned rule, exact rational shares, and
//! remainder distribution that never hands a leftover out silently.
//!
//! Values cross boundaries as decimal text (never as a float): "748.5151".
//! Accepted text is plain: an optional minus, digits without leading zeros,
//! an optional fraction; no plus sign, exponent, NaN or infinity. At most 28
//! significant digits (rust_decimal's range); anything larger is refused
//! rather than rounded. Library default rounding is never used.

use std::str::FromStr;

use rust_decimal::{Decimal, RoundingStrategy};

/// How a value is rounded to its scale. Names say the direction plainly:
/// "half_…" modes only differ at an exact half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    HalfEven,
    HalfAwayFromZero,
    HalfTowardZero,
    AwayFromZero,
    TowardZero,
    TowardPositive,
    TowardNegative,
}

impl RoundingMode {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "half_even" => Self::HalfEven,
            "half_away_from_zero" => Self::HalfAwayFromZero,
            "half_toward_zero" => Self::HalfTowardZero,
            "away_from_zero" => Self::AwayFromZero,
            "toward_zero" => Self::TowardZero,
            "toward_positive" => Self::TowardPositive,
            "toward_negative" => Self::TowardNegative,
            _ => return None,
        })
    }

    fn strategy(self) -> RoundingStrategy {
        match self {
            Self::HalfEven => RoundingStrategy::MidpointNearestEven,
            Self::HalfAwayFromZero => RoundingStrategy::MidpointAwayFromZero,
            Self::HalfTowardZero => RoundingStrategy::MidpointTowardZero,
            Self::AwayFromZero => RoundingStrategy::AwayFromZero,
            Self::TowardZero => RoundingStrategy::ToZero,
            Self::TowardPositive => RoundingStrategy::ToPositiveInfinity,
            Self::TowardNegative => RoundingStrategy::ToNegativeInfinity,
        }
    }
}

/// What to do with the leftover when rounded parts do not add up to the total.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemainderRule {
    /// No approved rule: the operation stops with `NeedsRule`.
    Reject,
    /// One unit at a time to the parts that fell furthest short (ties: input order).
    LargestRemainder,
}

impl RemainderRule {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "reject" => Some(Self::Reject),
            "largest_remainder" => Some(Self::LargestRemainder),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumericError {
    InvalidDecimal,
    TooManyDigits,
    ScaleOutOfRange,
    Overflow,
    InvalidShare,
    InvalidWeights,
    /// The parts do not add up and no remainder rule is approved; carries the remainder.
    NeedsRule(String),
    ScaleTooFineForTotal,
}

impl NumericError {
    /// Stable code, as the fixtures and the API name it.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidDecimal => "invalid_decimal",
            Self::TooManyDigits => "too_many_digits",
            Self::ScaleOutOfRange => "scale_out_of_range",
            Self::Overflow => "overflow",
            Self::InvalidShare => "invalid_share",
            Self::InvalidWeights => "invalid_weights",
            Self::NeedsRule(_) => "needs_rule",
            Self::ScaleTooFineForTotal => "scale_too_fine_for_total",
        }
    }
}

const MAX_DIGITS: usize = 28;
const MAX_SCALE: u32 = 28;

/// Parses plain decimal text exactly (see the module rules).
pub fn parse_decimal(text: &str) -> Result<Decimal, NumericError> {
    let body = text.strip_prefix('-').unwrap_or(text);
    let (int, frac) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };
    let digits_ok = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits_ok(int)
        || (int.len() > 1 && int.starts_with('0'))
        || frac.is_some_and(|f| !digits_ok(f))
    {
        return Err(NumericError::InvalidDecimal);
    }
    // Significant digits as a decimal counts them: leading zeros do not count, trailing ones do.
    let all: String = format!("{int}{}", frac.unwrap_or(""));
    let significant = all.trim_start_matches('0').len().max(1);
    if significant > MAX_DIGITS {
        return Err(NumericError::TooManyDigits);
    }
    Decimal::from_str_exact(text)
        .or_else(|_| Decimal::from_str(text))
        .map_err(|_| NumericError::Overflow)
}

/// Decimal text of a value, never "-0".
pub fn format_decimal(mut d: Decimal) -> String {
    if d.is_zero() {
        d.set_sign_positive(true);
    }
    d.to_string()
}

/// Rounds decimal text to `scale` places by `mode`, padding with zeros when it has fewer.
pub fn round_decimal(text: &str, scale: u32, mode: RoundingMode) -> Result<String, NumericError> {
    if scale > MAX_SCALE {
        return Err(NumericError::ScaleOutOfRange);
    }
    let d = parse_decimal(text)?;
    let mut r = d.round_dp_with_strategy(scale, mode.strategy());
    // Rounding left at most `scale` places: rescale only pads, it never rounds again.
    r.rescale(scale);
    Ok(format_decimal(r))
}

// ── Exact rationals ────────────────────────────────────────────────────

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// A share of a whole (hisse) as an exact fraction: 1/3 stays 1/3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Share {
    pub numerator: u128,
    pub denominator: u128,
}

impl Share {
    /// Reads "n/d" and reduces it; a zero denominator is refused.
    pub fn parse(text: &str) -> Result<Self, NumericError> {
        let (n, d) = text.split_once('/').ok_or(NumericError::InvalidShare)?;
        let n: u128 = n.parse().map_err(|_| NumericError::InvalidShare)?;
        let d: u128 = d.parse().map_err(|_| NumericError::InvalidShare)?;
        if d == 0 {
            return Err(NumericError::InvalidShare);
        }
        let g = gcd(n, d).max(1);
        Ok(Self {
            numerator: n / g,
            denominator: d / g,
        })
    }

    pub fn checked_add(self, other: Share) -> Result<Share, NumericError> {
        let g = gcd(self.denominator, other.denominator);
        let den = (self.denominator / g)
            .checked_mul(other.denominator)
            .ok_or(NumericError::Overflow)?;
        let num = self
            .numerator
            .checked_mul(den / self.denominator)
            .and_then(|a| {
                other
                    .numerator
                    .checked_mul(den / other.denominator)
                    .and_then(|b| a.checked_add(b))
            })
            .ok_or(NumericError::Overflow)?;
        let g = gcd(num, den).max(1);
        Ok(Share {
            numerator: num / g,
            denominator: den / g,
        })
    }

    pub fn is_whole(self) -> bool {
        self.numerator == self.denominator
    }
}

impl std::fmt::Display for Share {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

/// Exact sum of shares (it must be exactly 1 for a complete ownership record).
pub fn sum_shares(shares: &[Share]) -> Result<Share, NumericError> {
    shares.iter().try_fold(
        Share {
            numerator: 0,
            denominator: 1,
        },
        |acc, s| acc.checked_add(*s),
    )
}

/// A signed exact fraction with checked arithmetic (overflow is an error, never a wrong answer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ratio {
    num: i128,
    den: i128,
}

impl Ratio {
    fn of_decimal(d: Decimal) -> Result<Self, NumericError> {
        let den = 10i128
            .checked_pow(d.scale())
            .ok_or(NumericError::Overflow)?;
        Ok(Self {
            num: d.mantissa(),
            den,
        }
        .reduced())
    }

    fn reduced(self) -> Self {
        let g = gcd(self.num.unsigned_abs(), self.den.unsigned_abs()).max(1) as i128;
        let s = if self.den < 0 { -1 } else { 1 };
        Self {
            num: s * self.num / g,
            den: s * self.den / g,
        }
    }

    fn mul(self, o: Ratio) -> Result<Self, NumericError> {
        let a = Ratio {
            num: self.num,
            den: o.den,
        }
        .reduced();
        let b = Ratio {
            num: o.num,
            den: self.den,
        }
        .reduced();
        Ok(Self {
            num: a.num.checked_mul(b.num).ok_or(NumericError::Overflow)?,
            den: a.den.checked_mul(b.den).ok_or(NumericError::Overflow)?,
        }
        .reduced())
    }

    fn div(self, o: Ratio) -> Result<Self, NumericError> {
        if o.num == 0 {
            return Err(NumericError::InvalidWeights);
        }
        self.mul(
            Ratio {
                num: o.den,
                den: o.num,
            }
            .reduced(),
        )
    }

    fn add(self, o: Ratio) -> Result<Self, NumericError> {
        let n = self
            .num
            .checked_mul(o.den)
            .and_then(|a| o.num.checked_mul(self.den).and_then(|b| a.checked_add(b)))
            .ok_or(NumericError::Overflow)?;
        let d = self.den.checked_mul(o.den).ok_or(NumericError::Overflow)?;
        Ok(Self { num: n, den: d }.reduced())
    }

    fn sub(self, o: Ratio) -> Result<Self, NumericError> {
        self.add(Ratio {
            num: -o.num,
            den: o.den,
        })
    }

    /// Rounds to `scale` places by `mode` (exact: integer division and remainder).
    fn round(self, scale: u32, mode: RoundingMode) -> Result<Decimal, NumericError> {
        let p = 10i128.checked_pow(scale).ok_or(NumericError::Overflow)?;
        let n = self.num.checked_mul(p).ok_or(NumericError::Overflow)?;
        let (q, r) = (n / self.den, n % self.den);
        let q = if r == 0 {
            q
        } else {
            let away = if n < 0 { q - 1 } else { q + 1 };
            let twice = r.unsigned_abs() * 2;
            let half = twice.cmp(&self.den.unsigned_abs());
            let up = match mode {
                RoundingMode::AwayFromZero => true,
                RoundingMode::TowardZero => false,
                RoundingMode::TowardPositive => n > 0,
                RoundingMode::TowardNegative => n < 0,
                RoundingMode::HalfAwayFromZero => half != std::cmp::Ordering::Less,
                RoundingMode::HalfTowardZero => half == std::cmp::Ordering::Greater,
                RoundingMode::HalfEven => {
                    half == std::cmp::Ordering::Greater
                        || (half == std::cmp::Ordering::Equal && q % 2 != 0)
                }
            };
            if up { away } else { q }
        };
        Decimal::try_from_i128_with_scale(q, scale).map_err(|_| NumericError::Overflow)
    }
}

/// Parts of a distribution and the remainder that was handed out.
#[derive(Clone, Debug, PartialEq)]
pub struct Distribution {
    pub parts: Vec<String>,
    pub remainder: String,
}

/// Splits `total` by `weights`, each part rounded to `scale` by `mode`. When the
/// rounded parts do not add up, the rule decides: `Reject` stops with
/// `NeedsRule` (nothing is given out silently), `LargestRemainder` gives one
/// unit at a time to the parts that fell furthest short, ties in input order.
pub fn distribute(
    total: &str,
    weights: &[&str],
    scale: u32,
    mode: RoundingMode,
    rule: RemainderRule,
) -> Result<Distribution, NumericError> {
    if scale > MAX_SCALE {
        return Err(NumericError::ScaleOutOfRange);
    }
    let t = Ratio::of_decimal(parse_decimal(total)?)?;
    let ws = weights
        .iter()
        .map(|w| parse_decimal(w).and_then(Ratio::of_decimal))
        .collect::<Result<Vec<_>, _>>()?;
    let wsum = ws
        .iter()
        .try_fold(Ratio { num: 0, den: 1 }, |a, w| a.add(*w))?;
    if wsum.num <= 0 || ws.iter().any(|w| w.num < 0) {
        return Err(NumericError::InvalidWeights);
    }
    let exact = ws
        .iter()
        .map(|w| t.mul(*w)?.div(wsum))
        .collect::<Result<Vec<_>, _>>()?;
    let mut parts = exact
        .iter()
        .map(|e| e.round(scale, mode))
        .collect::<Result<Vec<_>, _>>()?;
    let rounded_sum = parts.iter().try_fold(Ratio { num: 0, den: 1 }, |a, p| {
        a.add(Ratio::of_decimal(*p)?)
    })?;
    let diff = t.sub(rounded_sum)?;
    let unit = Ratio {
        num: 1,
        den: 10i128.checked_pow(scale).ok_or(NumericError::Overflow)?,
    };
    let steps = diff.div(unit).map_err(|_| NumericError::Overflow)?;
    if steps.den != 1 {
        return Err(NumericError::ScaleTooFineForTotal);
    }
    // A whole number of units, so exact at `scale`; shown without trailing zeros.
    let remainder = format_decimal(diff.round(scale, RoundingMode::HalfEven)?.normalize());
    if steps.num == 0 {
        return Ok(Distribution {
            parts: parts.into_iter().map(format_decimal).collect(),
            remainder: "0".into(),
        });
    }
    if rule == RemainderRule::Reject {
        return Err(NumericError::NeedsRule(remainder));
    }
    let sign: i128 = if steps.num > 0 { 1 } else { -1 };
    let mut order: Vec<(Ratio, usize)> = exact
        .iter()
        .zip(&parts)
        .enumerate()
        .map(|(i, (e, p))| {
            Ok((
                Ratio { num: sign, den: 1 }.mul(e.sub(Ratio::of_decimal(*p)?)?)?,
                i,
            ))
        })
        .collect::<Result<_, NumericError>>()?;
    // Furthest short first; equal shortfalls keep input order.
    order.sort_by(|(a, i), (b, j)| {
        let lhs = a.num.checked_mul(b.den);
        let rhs = b.num.checked_mul(a.den);
        match (lhs, rhs) {
            (Some(l), Some(r)) => r.cmp(&l).then(i.cmp(j)),
            _ => i.cmp(j),
        }
    });
    let step =
        Decimal::try_from_i128_with_scale(sign, scale).map_err(|_| NumericError::Overflow)?;
    for k in 0..steps.num.unsigned_abs() as usize {
        let i = order[k % order.len()].1;
        parts[i] += step;
    }
    Ok(Distribution {
        parts: parts.into_iter().map(format_decimal).collect(),
        remainder,
    })
}
