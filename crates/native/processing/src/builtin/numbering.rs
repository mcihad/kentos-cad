//! Numbering corner points: the number format and the names (the web's
//! `builtin/numbering.ts`). The order a ring is walked in, one number per
//! place across many shapes (neighbouring parcels share their common
//! corners) and the outward direction at a corner come from the geometry
//! core (`processing::numbering`, docs/adr/0008 S4).

use kentos_contracts::Vec2;

use crate::geometry::CoreCorner;
use crate::text::{js_number, utf16_len};

/// How numbers are written: "P" + "00017".
#[derive(Clone, Debug, PartialEq)]
pub struct NumberFormat {
    /// Text before the number ("P").
    pub prefix: String,
    /// Total length with the prefix ("P00001" → 6).
    pub length: f64,
    /// Fills the gap between prefix and digits; empty: no padding.
    pub pad: String,
}

/// "P" + 17 at length 6 with "0" → "P00017". A number too long for the
/// width is written in full.
pub fn format_number(n: f64, f: &NumberFormat) -> String {
    let digits = js_number(n);
    let width = (f.length - utf16_len(&f.prefix) as f64).max(0.0);
    let Some(pad) = f.pad.chars().next() else {
        return format!("{}{digits}", f.prefix);
    };
    let missing = (width - utf16_len(&digits) as f64).max(0.0) as usize;
    let mut out = f.prefix.clone();
    out.extend(std::iter::repeat_n(pad, missing));
    out.push_str(&digits);
    out
}

/// The number in a name written in this format ("P00017" → 17).
pub fn parse_number(name: &str, f: &NumberFormat) -> Option<f64> {
    let rest = name.strip_prefix(f.prefix.as_str())?;
    let digits = if !f.pad.is_empty() && f.pad != "0" {
        rest.trim_start_matches(|c: char| f.pad.contains(c))
    } else {
        rest
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<f64>().ok()
}

/// A numbered point already on the target layer.
#[derive(Clone, Debug, PartialEq)]
pub struct Named {
    pub p: Vec2,
    pub name: String,
}

/// A corner with its name.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberedCorner {
    pub p: Vec2,
    pub name: String,
    /// Unit vector away from the shape (for text placement).
    pub out: Vec2,
    /// False when the corner reused an existing point or an earlier shape's number.
    pub created: bool,
}

/// Points already numbered: a point without a name is not one.
pub fn numbered_points(existing: &[Named]) -> Vec<Named> {
    existing
        .iter()
        .filter(|e| !e.name.is_empty())
        .cloned()
        .collect()
}

/// Names the core's corners (in numbering order): the counter starts at
/// `first`, or after the highest number written in this format among
/// `named` (the points given to the core), and a new number is made where
/// it first appears.
pub fn name_corners(
    corners: &[CoreCorner],
    named: &[Named],
    format: &NumberFormat,
    first: f64,
    step: f64,
) -> Vec<NumberedCorner> {
    let mut next = first;
    for e in named {
        if let Some(n) = parse_number(&e.name, format)
            && n + step > next
        {
            next = n + step;
        }
    }
    let mut made: Vec<String> = Vec::new();
    corners
        .iter()
        .map(|c| {
            let (name, created) = if c.refers < 0 {
                let j = (-1 - c.refers) as usize;
                (
                    named.get(j).map(|n| n.name.clone()).unwrap_or_default(),
                    false,
                )
            } else if (c.refers as usize) < made.len() {
                (made[c.refers as usize].clone(), false)
            } else {
                let name = format_number(next, format);
                next += step;
                made.push(name.clone());
                (name, true)
            };
            NumberedCorner {
                p: c.p,
                name,
                out: c.out,
                created,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(prefix: &str, length: f64, pad: &str) -> NumberFormat {
        NumberFormat {
            prefix: prefix.into(),
            length,
            pad: pad.into(),
        }
    }

    #[test]
    fn formats_and_reads_numbers_as_the_web_does() {
        assert_eq!(format_number(17.0, &f("P", 6.0, "0")), "P00017");
        assert_eq!(format_number(1.0, &f("K", 4.0, "")), "K1");
        assert_eq!(format_number(1234567.0, &f("P", 6.0, "0")), "P1234567");
        assert_eq!(format_number(3.0, &f("", 3.0, "-")), "--3");
        assert_eq!(parse_number("P00017", &f("P", 6.0, "0")), Some(17.0));
        assert_eq!(parse_number("P--17", &f("P", 6.0, "-")), Some(17.0));
        assert_eq!(parse_number("K17a", &f("K", 6.0, "0")), None);
        assert_eq!(parse_number("X1", &f("P", 6.0, "0")), None);
        assert_eq!(parse_number("P", &f("P", 6.0, "0")), None);
    }
}
