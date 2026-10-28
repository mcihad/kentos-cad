//! A dimension recipe's layout beyond its rows (docs/adr/0730): its rows' offsets at another sheet scale, its values'
//! places where they do not fit between their extension lines, and whether its rows' rounded values add up to its
//! rounded overall. Pure: the caller gives the frame's numbers, the texts' widths and the written values.

use crate::jsmath::{js_max, js_min};

/// A row's dimension line's offset at a sheet scale (docs/adr/0730 §1): its row `scale / plot_scale` times as far across
/// from the reference line as at the drawing's scale (`delta` its distance there, signed along the frame's normal), its
/// measured points where they are. The rows' layout on paper is the same at every scale.
pub fn scaled_offset(offset: f64, delta: f64, scale: f64, plot_scale: f64) -> f64 {
    offset + (scale / plot_scale - 1.0) * delta
}

/// A dimension's value as its placement reads it (docs/adr/0730 §2), in the plan's frame: `t` along its direction,
/// `o` across it (to its left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueBox {
    /// Its row's line: its place in the lines given.
    pub line: usize,
    /// The two measured points' `t`, the smaller first.
    pub t0: f64,
    pub t1: f64,
    /// Its text's width and height.
    pub width: f64,
    pub height: f64,
    /// Its baseline above its line, up as it reads: its text gap times its height (a centred value's −0.35 times).
    pub lift: f64,
    /// Its text gap's factor.
    pub gap: f64,
    /// +1 when it reads along the frame's direction (its up the frame's left), −1 when it reads the other way.
    pub reading: f64,
    /// A place the user moved it to, (along, across) as it reads: kept, and in the others' way.
    pub fixed: Option<(f64, f64)>,
}

/// A row's dimension line: its place across and its reach along.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowLine {
    pub o: f64,
    pub t0: f64,
    pub t1: f64,
}

/// Where a value went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// It fits between its extension lines: in its place.
    Fits,
    /// The user's place, kept.
    Fixed,
    /// It did not fit and was moved, (along, across) as it reads.
    Moved(f64, f64),
    /// It did not fit and no place was free: in its place.
    Tight,
}

/// A text's rectangle in the frame.
#[derive(Clone, Copy, Debug)]
struct Rect {
    t0: f64,
    t1: f64,
    o0: f64,
    o1: f64,
    line: usize,
}

/// Whether a value fits between its extension lines (docs/adr/0729 §6): its width and a height to spare within its
/// measured length.
pub fn fits(v: &ValueBox) -> bool {
    v.width + v.height <= v.t1 - v.t0
}

fn rect(v: &ValueBox, lines: &[RowLine], along: f64, across: f64) -> Rect {
    let c = (v.t0 + v.t1) * 0.5 + v.reading * along;
    let o = lines.get(v.line).map_or(0.0, |l| l.o);
    let (a, b) = (
        o + v.reading * (v.lift + across),
        o + v.reading * (v.lift + v.height + across),
    );
    Rect {
        t0: c - v.width * 0.5,
        t1: c + v.width * 0.5,
        o0: js_min(a, b),
        o1: js_max(a, b),
        line: v.line,
    }
}

/// Whether `r` meets a placed text (along within `margin`, across overlapping) or crosses another row's line.
fn blocked(r: &Rect, placed: &[Rect], lines: &[RowLine], margin: f64) -> bool {
    placed
        .iter()
        .any(|q| r.t0 < q.t1 + margin && r.t1 > q.t0 - margin && r.o0 < q.o1 && r.o1 > q.o0)
        || lines.iter().enumerate().any(|(k, l)| {
            k != r.line && r.o0 < l.o && r.o1 > l.o && r.t0 < l.t1 && r.t1 > l.t0
        })
}

/// The values' places (docs/adr/0730 §2), in the order given (rows in the recipe's order, along each row): those that fit
/// and those the user moved stand first; each that does not fit then takes the first free of: beyond its reading end,
/// before its reading start, above its line, below it (half a text height apart from another text along, never across
/// another row's line); none free, it stays.
pub fn place_values(values: &[ValueBox], lines: &[RowLine]) -> Vec<Placement> {
    let mut out = vec![Placement::Fits; values.len()];
    let mut placed: Vec<Rect> = Vec::with_capacity(values.len());
    for (i, v) in values.iter().enumerate() {
        if let Some((along, across)) = v.fixed {
            out[i] = Placement::Fixed;
            placed.push(rect(v, lines, along, across));
        } else if fits(v) {
            placed.push(rect(v, lines, 0.0, 0.0));
        }
    }
    for (i, v) in values.iter().enumerate() {
        if v.fixed.is_some() || fits(v) {
            continue;
        }
        let margin = v.height * 0.5;
        let c = (v.t0 + v.t1) * 0.5;
        // Beyond its reading end and before its reading start, as far along as half a height clears the line's end.
        let (end, start) = if v.reading >= 0.0 { (v.t1, v.t0) } else { (v.t0, v.t1) };
        let beyond = v.reading * (end + v.reading * (margin + v.width * 0.5) - c);
        let before = v.reading * (start - v.reading * (margin + v.width * 0.5) - c);
        let up = v.height * (1.0 + v.gap);
        let down = js_min(v.lift, 0.0) - v.lift - v.height - v.gap * v.height;
        let candidates = [(beyond, 0.0), (before, 0.0), (0.0, up), (0.0, down)];
        let chosen = candidates.into_iter().find(|&(along, across)| {
            !blocked(&rect(v, lines, along, across), &placed, lines, margin)
        });
        match chosen {
            Some((along, across)) => {
                out[i] = Placement::Moved(along, across);
                placed.push(rect(v, lines, along, across));
            }
            None => {
                out[i] = Placement::Tight;
                placed.push(rect(v, lines, 0.0, 0.0));
            }
        }
    }
    out
}

/// A written value as a whole number of its last decimal: `"-1.175"` is (−1175, 3); none when it is not a plain decimal
/// number (a sign, digits, at most one point with digits after it).
fn decimal(text: &str) -> Option<(i128, usize)> {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = match body.split_once('.') {
        Some((w, f)) if !f.is_empty() => (w, f),
        Some(_) => return None,
        None => (body, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || whole.len() + fraction.len() > 30
    {
        return None;
    }
    let digits: i128 = format!("{whole}{fraction}").parse().ok()?;
    Some((if negative { -digits } else { digits }, fraction.len()))
}

/// A row's written values against its overall's (docs/adr/0730 §3): their sum written in their decimals, when every one is
/// a plain decimal number with as many decimals as the overall and the sum is not what the overall says; none otherwise.
pub fn rounding_gap(parts: &[&str], overall: &str) -> Option<String> {
    let (total, decimals) = decimal(overall)?;
    let mut sum: i128 = 0;
    for part in parts {
        let (value, d) = decimal(part)?;
        if d != decimals {
            return None;
        }
        sum = sum.checked_add(value)?;
    }
    if sum == total || parts.is_empty() {
        return None;
    }
    let negative = sum < 0;
    let digits = sum.unsigned_abs().to_string();
    let text = if decimals == 0 {
        digits
    } else {
        let padded = format!("{digits:0>width$}", width = decimals + 1);
        let (w, f) = padded.split_at(padded.len() - decimals);
        format!("{w}.{f}")
    };
    Some(if negative { format!("-{text}") } else { text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rows_offset_grows_with_the_sheet_and_stays_at_the_drawings_scale() {
        // A row 0.9 m out from the face at 1:50: 1.8 m out at 1:100, as far on paper; at 1:50 as it was.
        assert_eq!(scaled_offset(-0.9, -0.9, 100.0, 50.0), -1.8);
        assert_eq!(scaled_offset(-0.9, -0.9, 50.0, 50.0), -0.9);
        // A row's member measuring points off the face (a wall's far side): only the row's distance grows.
        assert_eq!(scaled_offset(-1.0, -0.9, 200.0, 50.0), -1.0 + 3.0 * -0.9);
    }

    fn value(t0: f64, t1: f64, width: f64) -> ValueBox {
        ValueBox {
            line: 0,
            t0,
            t1,
            width,
            height: 0.125,
            lift: 0.0625,
            gap: 0.5,
            reading: 1.0,
            fixed: None,
        }
    }

    #[test]
    fn a_value_that_does_not_fit_goes_beyond_its_end_then_before_its_start_then_above() {
        let lines = [RowLine { o: -1.0, t0: 0.0, t1: 10.0 }];
        // Alone: beyond its reading end, half a height clear of its line's end.
        let thin = value(0.0, 0.25, 0.3);
        assert!(!fits(&thin));
        let beyond = 0.25 + 0.0625 + 0.15 - 0.125;
        assert_eq!(place_values(&[thin], &lines), [Placement::Moved(beyond, 0.0)]);
        // A fitting neighbour's value there: before its start.
        let next = value(0.25, 1.25, 0.4);
        assert!(fits(&next));
        assert_eq!(
            place_values(&[thin, next], &lines),
            [Placement::Moved(-beyond, 0.0), Placement::Fits]
        );
        // A value the user moved where its start's place is: above its line, the user's kept.
        let moved = ValueBox {
            fixed: Some((0.5 + 0.125 - beyond, 0.0)),
            ..value(-1.0, 0.0, 0.3)
        };
        assert_eq!(
            place_values(&[moved, thin, next], &lines),
            [Placement::Fixed, Placement::Moved(0.0, 0.125 * 1.5), Placement::Fits]
        );
    }

    #[test]
    fn a_value_with_no_free_place_stays_and_another_rows_line_is_never_crossed() {
        // Its own line's neighbours on both sides, another row's line through its places above and below.
        let lines = [
            RowLine { o: 0.0, t0: 0.0, t1: 10.0 },
            RowLine { o: 0.3, t0: 0.0, t1: 10.0 },
            RowLine { o: -0.1, t0: 0.0, t1: 10.0 },
        ];
        let thin = value(1.0, 1.25, 0.3);
        let left = value(0.0, 1.0, 0.6);
        let right = value(1.25, 2.25, 0.6);
        assert_eq!(
            place_values(&[left, thin, right], &lines),
            [Placement::Fits, Placement::Tight, Placement::Fits]
        );
    }

    #[test]
    fn rounded_parts_against_the_overall_are_added_exactly() {
        assert_eq!(rounding_gap(&["1.000", "1.000", "1.000"], "3.001").as_deref(), Some("3.000"));
        assert_eq!(rounding_gap(&["1.175", "0.900", "1.925"], "4.000"), None);
        assert_eq!(rounding_gap(&["-0.300", "0.100"], "-0.150").as_deref(), Some("-0.200"));
        assert_eq!(rounding_gap(&["0.001", "0.001"], "0.003").as_deref(), Some("0.002"));
        // Not plain decimals in the same decimals: nothing said.
        assert_eq!(rounding_gap(&["1.00", "2.000"], "3.001"), None);
        assert_eq!(rounding_gap(&["~1.000", "2.000"], "3.001"), None);
        assert_eq!(rounding_gap(&[], "3.001"), None);
    }
}
