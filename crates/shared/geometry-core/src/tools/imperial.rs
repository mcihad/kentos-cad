//! Feet and inches (docs/adr/0262): how a length typed with feet (`'`) or
//! inch (`"`) marks reads, and how a length is written in a project's
//! length style (decimal, architectural, engineering, fractional).
//!
//! A length typed with a mark is in inches here, whatever the drawing's
//! unit: `5'6-1/2"` is 66.5 inches, `3/4"` 0.75, `5'` 60. The grammar, as
//! AutoCAD takes architectural input:
//!
//! ```text
//! [-+] [FEET '] [-] [INCHES] [[-] WHOLE/DENOM] ["]
//! ```
//!
//! - at least one mark: feet need `'`; inches without feet need `"`;
//! - FEET and INCHES are `\d+(\.\d+)?`; a fraction is `\d+/\d+` with a
//!   denominator above 0, after the inches with `-` (`6-1/2"`) or alone
//!   (`1/2"`);
//! - no white space inside: a space separates two coordinates.
//!
//! Exact: 1 inch is 0.0254 m, 1 foot 0.3048 m (the international foot).

use crate::display::fixed;
use crate::jsmath::js_round;

/// Metres in an inch, exactly.
pub const METRES_PER_INCH: f64 = 0.0254;
/// Inches in a foot.
pub const INCHES_PER_FOOT: f64 = 12.0;

/// `\d+(\.\d+)?` at the start of `s`: its value and the rest; `None` for no digit.
fn decimal(s: &str) -> Option<(f64, &str)> {
    let b = s.as_bytes();
    let whole = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if whole == 0 {
        return None;
    }
    let mut end = whole;
    if b.get(end) == Some(&b'.') {
        let frac = b[end + 1..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if frac > 0 {
            end += 1 + frac;
        }
    }
    Some((s[..end].parse().ok()?, &s[end..]))
}

/// `\d+/\d+` at the start of `s`, the denominator above 0.
fn fraction(s: &str) -> Option<(f64, &str)> {
    let b = s.as_bytes();
    let n = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 || b.get(n) != Some(&b'/') {
        return None;
    }
    let d = b[n + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
    if d == 0 {
        return None;
    }
    let num: f64 = s[..n].parse().ok()?;
    let den: f64 = s[n + 1..n + 1 + d].parse().ok()?;
    (den > 0.0).then_some((num / den, &s[n + 1 + d..]))
}

/// A feet-and-inches length at the start of `s`, in inches, and the rest;
/// `None` when it is not one (no mark: a plain number is not taken here).
pub fn feet_inches(s: &str) -> Option<(f64, &str)> {
    let (sign, mut rest) = match s.as_bytes().first() {
        Some(b'-') => (-1.0, &s[1..]),
        Some(b'+') => (1.0, &s[1..]),
        _ => (1.0, s),
    };
    let mut inches = 0.0;
    let mut marked = false;
    let mut any = false;
    // Feet: a number and `'`.
    if let Some((v, after)) = decimal(rest)
        && let Some(after) = after.strip_prefix('\'')
    {
        inches += v * INCHES_PER_FOOT;
        rest = after.strip_prefix('-').unwrap_or(after);
        marked = true;
        any = true;
    }
    // Inches: a whole or a fraction, or a whole, `-` and a fraction.
    if let Some((f, after)) = fraction(rest) {
        inches += f;
        rest = after;
        any = true;
    } else if let Some((v, after)) = decimal(rest) {
        inches += v;
        rest = after;
        any = true;
        if let Some(more) = rest.strip_prefix('-')
            && let Some((f, after)) = fraction(more)
        {
            inches += f;
            rest = after;
        }
    }
    if let Some(after) = rest.strip_prefix('"') {
        rest = after;
        marked = true;
    }
    (marked && any).then_some((sign * inches, rest))
}

/// How lengths are written (docs/adr/0262 §2): decimal in the drawing's
/// unit, or feet and inches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    /// `12.5417` in the drawing's unit.
    #[default]
    Decimal,
    /// `12'-6 1/2"`: feet, inches and a fraction of an inch.
    Architectural,
    /// `12'-6.50"`: feet and decimal inches.
    Engineering,
    /// `150 1/2"`: inches and a fraction.
    Fractional,
}

/// `inches` rounded to the nearest 1/`den` of an inch: the whole and the
/// numerator, in lowest terms with its denominator.
fn split_fraction(inches: f64, den: u32) -> (f64, u32, u32) {
    let den = den.max(1);
    let steps = js_round(inches * f64::from(den));
    let whole = (steps / f64::from(den)).floor();
    let mut num = (steps - whole * f64::from(den)) as u32;
    let mut d = den;
    while num > 0 && num.is_multiple_of(2) && d.is_multiple_of(2) {
        num /= 2;
        d /= 2;
    }
    (whole, num, d)
}

fn inches_text(whole: f64, num: u32, den: u32) -> String {
    match (whole == 0.0, num) {
        (_, 0) => format!("{whole}"),
        (true, _) => format!("{num}/{den}"),
        (false, _) => format!("{whole} {num}/{den}"),
    }
}

/// A length of `inches` in `style`: `decimals` are the decimal styles'
/// places, the fractional ones' denominator `2^decimals` (0 whole inches, 4
/// sixteenths), as AutoCAD's precision.
pub fn format_inches(inches: f64, style: Style, decimals: u32) -> String {
    let neg = inches < 0.0;
    let a = inches.abs();
    let den = 1u32 << decimals.min(8);
    let body = match style {
        Style::Decimal => fixed(a, decimals as usize),
        Style::Fractional => {
            let (w, n, d) = split_fraction(a, den);
            format!("{}\"", inches_text(w, n, d))
        }
        Style::Architectural => {
            let (w, n, d) = split_fraction(a, den);
            // Whole inches past a foot carry into feet.
            let feet = (w / INCHES_PER_FOOT).floor();
            let rest = w - feet * INCHES_PER_FOOT;
            // AutoCAD's way: the inches always written, `0 3/8` when only a fraction.
            let inches = if n == 0 {
                format!("{rest}")
            } else {
                format!("{rest} {n}/{d}")
            };
            format!("{feet}'-{inches}\"")
        }
        Style::Engineering => {
            let scale = (0..decimals.min(8)).fold(1.0, |s, _| s * 10.0);
            let rounded = js_round(a * scale) / scale;
            let feet = (rounded / INCHES_PER_FOOT).floor();
            let rest = rounded - feet * INCHES_PER_FOOT;
            format!("{feet}'-{}\"", fixed(rest, decimals as usize))
        }
    };
    if neg && body.chars().any(|c| c.is_ascii_digit() && c != '0') {
        format!("-{body}")
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feet_and_inches_read_in_inches() {
        let read = |s: &str| feet_inches(s).map(|(v, rest)| (v, rest.to_owned()));
        assert_eq!(read("5'6-1/2\""), Some((66.5, String::new())));
        assert_eq!(read("5'-6\""), Some((66.0, String::new())));
        assert_eq!(read("5'"), Some((60.0, String::new())));
        assert_eq!(read("6\""), Some((6.0, String::new())));
        assert_eq!(read("3/4\""), Some((0.75, String::new())));
        assert_eq!(read("-2'3\""), Some((-27.0, String::new())));
        assert_eq!(read("5'6.5\",3'"), Some((66.5, ",3'".to_owned())));
        // No mark: not taken here.
        assert_eq!(read("66"), None);
        assert_eq!(read("1/2"), None);
        assert_eq!(read("x'"), None);
    }

    #[test]
    fn lengths_are_written_in_their_style() {
        assert_eq!(format_inches(150.5, Style::Architectural, 4), "12'-6 1/2\"");
        assert_eq!(format_inches(150.5, Style::Engineering, 2), "12'-6.50\"");
        assert_eq!(format_inches(150.5, Style::Fractional, 4), "150 1/2\"");
        assert_eq!(format_inches(144.0, Style::Architectural, 4), "12'-0\"");
        // 11 31/32 rounds to a whole foot at sixteenths.
        assert_eq!(
            format_inches(11.0 + 31.0 / 32.0, Style::Architectural, 4),
            "1'-0\""
        );
        assert_eq!(format_inches(-0.25, Style::Fractional, 4), "-1/4\"");
        assert_eq!(format_inches(0.0, Style::Architectural, 4), "0'-0\"");
        assert_eq!(format_inches(12.375, Style::Architectural, 4), "1'-0 3/8\"");
    }
}
