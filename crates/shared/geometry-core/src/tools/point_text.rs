//! The grammar of typed point input, as the web reads it
//! (`apps/web/src/tools/coordinateInput.ts`): the value field beside the
//! cursor and the command line send the same text to a tool.
//!
//! ```text
//! 486512.34,4420118.9   absolute Y,X (also Y;X, or Y X)
//! @12.5,-3              relative to the last point: @dY,dX
//! @25<45                distance<angle, degrees counter-clockwise from east (the @ may be left out)
//! 18.4                  a distance along the tracking line, else towards the cursor
//! ```
//!
//! The web keeps its regular expressions; the desktop reads with this module
//! (docs/adr/0021). The shared cases in `fixtures/point-input/v1/cases.json`
//! hold both readers to the same answers, so they cannot drift. The grammar
//! itself (units, grads, expressions, the decimal comma) is TODOS.md UX-05;
//! this is today's, kept to the character:
//!
//! - `trim()` and `\s` are ECMAScript's white space and line terminators, not
//!   Rust's `char::is_whitespace`: U+FEFF is one, U+0085 is not;
//! - `\d` is an ASCII digit; a number is `[-+]?\d+(\.\d+)?`, nothing else
//!   (no exponent, no leading or trailing point);
//! - a number reads as JavaScript's `Number()` reads it: both round to the
//!   nearest double.

use crate::tools::point_input::{polar_offset, relative_point, toward_point};
use crate::vec2::Vec2;

/// What typed point text says, before it is placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointText {
    /// `Y,X`: an absolute point, east then north.
    Absolute(Vec2),
    /// `@dY,dX`: the last point moved by the differences.
    Relative { dx: f64, dy: f64 },
    /// `@distance<angle`: from the last point, the angle in degrees counter-clockwise from east.
    Polar { distance: f64, angle: f64 },
    /// A bare number: that far along the tracking line, else towards the cursor.
    Distance(f64),
}

/// Reads point text; `None` when it is none of the forms. The forms are tried
/// in the web's order: relative, polar, absolute, distance.
pub fn parse_point_text(text: &str) -> Option<PointText> {
    let t = js_trim(text);
    if let Some(rest) = t.strip_prefix('@')
        && let Some((dx, dy)) = pair(rest)
    {
        return Some(PointText::Relative { dx, dy });
    }
    if let Some((distance, angle)) = polar(t.strip_prefix('@').unwrap_or(t)) {
        return Some(PointText::Polar { distance, angle });
    }
    if let Some((x, y)) = pair(t) {
        return Some(PointText::Absolute(Vec2::new(x, y)));
    }
    whole(t).map(PointText::Distance)
}

/// The point typed text gives (the web's `parsePointInput`): `last` is the
/// tool's last point, `cursor` the effective cursor, `along` a point that far
/// along an active tracking line (object tracking; `None` when there is none).
/// Relative and polar input need a last point, a distance a last point and a
/// cursor apart from it; without them the text gives no point.
pub fn point_from_text(
    text: &str,
    last: Option<Vec2>,
    cursor: Option<Vec2>,
    along: impl FnOnce(f64) -> Option<Vec2>,
) -> Option<Vec2> {
    match parse_point_text(text)? {
        PointText::Relative { dx, dy } => last.map(|last| relative_point(last, dx, dy)),
        PointText::Polar { distance, angle } => {
            last.map(|last| polar_offset(last, distance, angle))
        }
        PointText::Absolute(p) => Some(p),
        PointText::Distance(d) => along(d).or_else(|| toward_point(last?, cursor?, d)),
    }
}

/// A single typed number (a length, a radius, an angle): a decimal comma is
/// taken for the point, once (the web's `parseNumber`).
pub fn parse_number(text: &str) -> Option<f64> {
    whole(&js_trim(text).replacen(',', ".", 1))
}

/// Whether text starts like a number or a coordinate: a digit, `.`, `@`, `+`
/// or `-` (the web's `looksLikeCoordinate`).
pub fn looks_like_coordinate(text: &str) -> bool {
    js_trim(text)
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || matches!(c, '@' | '.' | '+' | '-'))
}

/// ECMAScript's white space and line terminators: what `trim()` removes and
/// what `\s` matches.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000d}'
            | ' '
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

/// `text.trim()`.
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

/// `[-+]?\d+(?:\.\d+)?` at the start of `s`: the number and what follows it.
/// Greedy is exact here: what may follow a number in every form is never a
/// digit or a point, so a shorter number could not match either.
fn number(s: &str) -> Option<(f64, &str)> {
    let bytes = s.as_bytes();
    let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let whole = digits(end);
    if whole == 0 {
        return None;
    }
    end += whole;
    if bytes.get(end) == Some(&b'.') {
        let fraction = digits(end + 1);
        if fraction > 0 {
            end += 1 + fraction;
        }
    }
    // ASCII up to `end`, so the split is on a character boundary. Rust reads
    // the sign, the digits and the point as Number() does, correctly rounded.
    let value = s[..end].parse::<f64>().ok()?;
    Some((value, &s[end..]))
}

/// `^NUM$`.
fn whole(s: &str) -> Option<f64> {
    number(s).and_then(|(value, rest)| rest.is_empty().then_some(value))
}

/// `^NUM\s*[,; ]\s*NUM$`: between the numbers white space and exactly one
/// separator; a plain space can be that separator.
fn pair(s: &str) -> Option<(f64, f64)> {
    let (a, rest) = number(s)?;
    let between = rest.len()
        - rest
            .trim_start_matches(|c| is_js_space(c) || c == ',' || c == ';')
            .len();
    let (gap, rest) = rest.split_at(between);
    let marks = gap.chars().filter(|&c| c == ',' || c == ';').count();
    let separated = match marks {
        0 => gap.contains(' '),
        1 => true,
        _ => false,
    };
    if !separated {
        return None;
    }
    whole(rest).map(|b| (a, b))
}

/// `^NUM\s*<\s*NUM$`.
fn polar(s: &str) -> Option<(f64, f64)> {
    let (a, rest) = number(s)?;
    let between = rest.len()
        - rest
            .trim_start_matches(|c| is_js_space(c) || c == '<')
            .len();
    let (gap, rest) = rest.split_at(between);
    if gap.chars().filter(|&c| c == '<').count() != 1 {
        return None;
    }
    whole(rest).map(|b| (a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAST: Vec2 = Vec2::new(100.0, 200.0);

    fn point(text: &str, last: Option<Vec2>, cursor: Option<Vec2>) -> Option<Vec2> {
        point_from_text(text, last, cursor, |_| None)
    }

    #[test]
    fn the_four_forms_in_the_web_order() {
        assert_eq!(
            parse_point_text("486512.34,4420118.9"),
            Some(PointText::Absolute(Vec2::new(486512.34, 4420118.9)))
        );
        assert_eq!(
            parse_point_text("@5,-3"),
            Some(PointText::Relative { dx: 5.0, dy: -3.0 })
        );
        assert_eq!(
            parse_point_text("25<45"),
            Some(PointText::Polar {
                distance: 25.0,
                angle: 45.0
            })
        );
        assert_eq!(parse_point_text(" -4 "), Some(PointText::Distance(-4.0)));
        assert_eq!(parse_point_text("abc"), None);
    }

    #[test]
    fn relative_and_polar_need_a_last_point_and_do_not_fall_through() {
        assert_eq!(
            point("@5,-3", Some(LAST), None),
            Some(Vec2::new(105.0, 197.0))
        );
        assert_eq!(point("@5,-3", None, None), None);
        assert_eq!(point("10<0", None, Some(LAST)), None);
    }

    #[test]
    fn a_distance_follows_tracking_first_then_the_cursor() {
        let cursor = Some(Vec2::new(110.0, 200.0));
        assert_eq!(
            point("5", Some(LAST), cursor),
            Some(Vec2::new(105.0, 200.0))
        );
        let tracked = point_from_text("5", Some(LAST), cursor, |d| Some(Vec2::new(0.0, d)));
        assert_eq!(tracked, Some(Vec2::new(0.0, 5.0)));
        assert_eq!(point("5", Some(LAST), None), None);
        assert_eq!(
            point("5", Some(LAST), Some(LAST)),
            None,
            "the cursor on the last point gives no direction"
        );
    }

    #[test]
    fn separators_are_one_comma_semicolon_or_space() {
        assert_eq!(point("10;20", None, None), Some(Vec2::new(10.0, 20.0)));
        assert_eq!(point("10 20", None, None), Some(Vec2::new(10.0, 20.0)));
        assert_eq!(
            point(" 10 ,\u{a0}20 ", None, None),
            Some(Vec2::new(10.0, 20.0))
        );
        assert_eq!(
            point("10\t20", None, None),
            None,
            "a tab is white space, not a separator"
        );
        assert_eq!(point("10\t 20", None, None), Some(Vec2::new(10.0, 20.0)));
        assert_eq!(point("10,,20", None, None), None);
        assert_eq!(point("10,;20", None, None), None);
    }

    #[test]
    fn javascript_white_space_not_rust_s() {
        assert_eq!(parse_number("\u{feff}12"), Some(12.0));
        assert_eq!(parse_number("12\u{85}"), None);
        assert!(looks_like_coordinate("\u{3000}@1"));
    }

    #[test]
    fn numbers_are_decimal_digits_only() {
        assert_eq!(parse_number("2,5"), Some(2.5));
        assert_eq!(parse_number("1,2,3"), None);
        assert_eq!(parse_number("+7"), Some(7.0));
        for bad in ["", "1.", ".5", "1e3", "١٢", "１２", "--1", "inf", "NaN"] {
            assert_eq!(parse_number(bad), None, "{bad:?}");
        }
        // Correctly rounded, as Number() reads it.
        assert_eq!(parse_number("9007199254740993"), Some(9007199254740992.0));
    }
}
