//! A chart's axes and numbers (docs/adr/0227 §5): the ticks of an axis in
//! exact decimals (steps 1, 2, 2.5 and 5 times a power of ten, the first in
//! that order whose ticks covering the range are no more than asked), and a
//! double with four significant digits (the trend line's words).

use crate::ops::statistics::Dec;

/// An axis: its ends (multiples of its step), its step and its ticks with their texts.
#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub lo: Dec,
    pub hi: Dec,
    pub step: Dec,
    pub ticks: Vec<Dec>,
    pub labels: Vec<String>,
}

fn pow10(k: u32) -> i128 {
    10i128.pow(k)
}

/// `x` as an integer at `scale` (scale ≥ x's).
fn int_at(x: Dec, scale: u32) -> Option<i128> {
    x.at(scale).map(|d| d.m)
}

fn floor_div(a: i128, b: i128) -> i128 {
    let q = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

fn ceil_div(a: i128, b: i128) -> i128 {
    -floor_div(-a, b)
}

/// A decimal with no trailing zeros in its scale.
pub fn trimmed(d: Dec) -> Dec {
    let (mut m, mut s) = (d.m, d.scale);
    while s > 0 && m % 10 == 0 {
        m /= 10;
        s -= 1;
    }
    Dec { m, scale: s }
}

/// The power of ten of a non-zero decimal's first significant digit.
fn magnitude(d: Dec) -> i32 {
    let digits = d.m.unsigned_abs().to_string().len() as i32;
    digits - 1 - d.scale as i32
}

/// A step `d`·10^`e` (d one of 10, 20, 25, 50 tenths) as a decimal.
fn step_of(tenths: i128, e: i32) -> Dec {
    // tenths · 10^(e − 1)
    let p = e - 1;
    if p >= 0 {
        Dec {
            m: tenths * pow10(p as u32),
            scale: 0,
        }
    } else {
        trimmed(Dec {
            m: tenths,
            scale: (-p) as u32,
        })
    }
}

/// The axis over `lo`…`hi` with at most `max` ticks (at least 2); `integer`
/// asks whole steps (counts), `zero` takes 0 into the range (bars).
pub fn nice(lo: Dec, hi: Dec, max: u32, integer: bool, zero: bool) -> Option<Axis> {
    let max = max.max(2) as i128;
    let zero_dec = Dec { m: 0, scale: 0 };
    let (mut lo, mut hi) = (lo, hi);
    if zero {
        if lo.m > 0 {
            lo = zero_dec;
        }
        if hi.m < 0 {
            hi = zero_dec;
        }
    }
    let s = lo.scale.max(hi.scale);
    if int_at(lo, s)? == int_at(hi, s)? {
        if lo.m == 0 {
            hi = Dec { m: 1, scale: 0 };
        } else {
            // One digit below the value's first: 101 takes 91…111.
            let e = magnitude(lo) - 1;
            let unit = if e >= 0 {
                Dec {
                    m: pow10(e as u32),
                    scale: 0,
                }
            } else {
                Dec {
                    m: 1,
                    scale: (-e) as u32,
                }
            };
            let t = lo.scale.max(unit.scale);
            let (v, u) = (int_at(lo, t)?, int_at(unit, t)?);
            lo = Dec { m: v - u, scale: t };
            hi = Dec { m: v + u, scale: t };
        }
    }
    // The first exponent to try: a step a hundredth of the range over the ticks is too small.
    let s = lo.scale.max(hi.scale);
    let span = Dec {
        m: int_at(hi, s)? - int_at(lo, s)?,
        scale: s,
    };
    let start = magnitude(span) - (max.to_string().len() as i32) - 1;
    for e in start..start + 64 {
        for tenths in [10i128, 20, 25, 50] {
            let step = step_of(tenths, e);
            if integer && step.scale > 0 {
                continue;
            }
            let t = s.max(step.scale);
            let (l, h, st) = (int_at(lo, t)?, int_at(hi, t)?, int_at(step, t)?);
            let (a, b) = (floor_div(l, st), ceil_div(h, st));
            // b − a + 1 ticks.
            if b - a < max {
                let ticks: Vec<Dec> = (a..=b)
                    .map(|k| {
                        trimmed(Dec {
                            m: k * st,
                            scale: t,
                        })
                    })
                    .collect();
                // A tick is a multiple of the step: exact at the step's digits.
                let digits = step.scale;
                let labels = (a..=b)
                    .map(|k| {
                        Dec {
                            m: k * st / pow10(t - digits),
                            scale: digits,
                        }
                        .text()
                    })
                    .collect();
                return Some(Axis {
                    lo: trimmed(Dec {
                        m: a * st,
                        scale: t,
                    }),
                    hi: trimmed(Dec {
                        m: b * st,
                        scale: t,
                    }),
                    step,
                    ticks,
                    labels,
                });
            }
        }
    }
    None
}

/// A double with four significant digits, without an exponent: the exact
/// binary value in scientific notation rounded half to even at three
/// fraction digits (`5.234e2` → `523.4`); zero, of either sign, is `0`.
pub fn sig4(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return if x.is_nan() {
            "—".to_owned()
        } else if x.is_infinite() {
            if x > 0.0 {
                "∞".to_owned()
            } else {
                "-∞".to_owned()
            }
        } else {
            "0".to_owned()
        };
    }
    let text = format!("{x:.3e}");
    let (mantissa, exp) = text.split_once('e').unwrap_or((&text, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (neg, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => (true, m),
        None => (false, mantissa),
    };
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    // The point sits after the first digit: move it by the exponent.
    let point = 1 + exp;
    let body = if point <= 0 {
        format!("0.{}{}", "0".repeat((-point) as usize), digits)
    } else if point as usize >= digits.len() {
        format!("{}{}", digits, "0".repeat(point as usize - digits.len()))
    } else {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    };
    if neg { format!("-{body}") } else { body }
}
