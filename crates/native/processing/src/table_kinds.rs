//! A tool's table written with its columns' kinds when the tool says none (docs/adr/9190): a column whose every cell the
//! tool shows as a whole number (digits, a sign) is INTEGER; one whose every cell is a number with the same count of
//! decimals after a point (what fixed formatting writes: `fixed(x, n)`) is REAL; any other is text. An empty cell is no
//! value and decides nothing. A grouped number (“1.024”), a unit (“12 m”), a dash or a decimal comma is no number
//! here, nor a code with a leading zero (“007”) or a whole number past 64 bits: such a column stays text, so nothing
//! is read wrong; a tool that knows its values gives them itself (`kinds`, `values`: docs/adr/9136).

use kentos_native_application::geopackage::CellKind;
use serde_json::Value;

/// A cell's form: a whole number, a number with this many decimals, no value, or text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    Whole,
    Decimals(usize),
    Empty,
    Text,
}

fn form(cell: &str) -> Form {
    let c = cell.trim();
    if c.is_empty() {
        return Form::Empty;
    }
    let digits = c.strip_prefix('-').unwrap_or(c);
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    // A leading zero is a code's (“007”), never what a number's formatting writes (“0”, “0.5” are numbers).
    if !all_digits(whole) || (whole.len() > 1 && whole.starts_with('0')) {
        return Form::Text;
    }
    match fraction {
        None if c.parse::<i64>().is_ok() => Form::Whole,
        Some(f) if all_digits(f) => Form::Decimals(f.len()),
        _ => Form::Text,
    }
}

/// The kinds of `rows`' columns and their cells' plain values (a number, nothing, or the shown text).
pub fn read(rows: &[Vec<String>]) -> (Vec<CellKind>, Vec<Vec<Value>>) {
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let kinds: Vec<CellKind> = (0..width)
        .map(|j| {
            let mut seen: Option<Form> = None;
            for r in rows {
                let f = r.get(j).map_or(Form::Empty, |c| form(c));
                match (f, seen) {
                    (Form::Empty, _) => {}
                    (Form::Text, _) => return CellKind::Text,
                    (f, None) => seen = Some(f),
                    (f, Some(s)) if f == s => {}
                    _ => return CellKind::Text,
                }
            }
            match seen {
                Some(Form::Whole) => CellKind::Integer,
                Some(Form::Decimals(_)) => CellKind::Real,
                _ => CellKind::Text,
            }
        })
        .collect();
    let values = rows
        .iter()
        .map(|r| {
            (0..width)
                .map(|j| {
                    let cell = r.get(j).map_or("", String::as_str).trim();
                    match kinds[j] {
                        _ if cell.is_empty() => Value::Null,
                        CellKind::Integer => cell
                            .parse::<i64>()
                            .map_or_else(|_| Value::String(cell.to_owned()), Value::from),
                        CellKind::Real => cell
                            .parse::<f64>()
                            .map_or_else(|_| Value::String(cell.to_owned()), Value::from),
                        CellKind::Text => Value::String(cell.to_owned()),
                    }
                })
                .collect()
        })
        .collect();
    (kinds, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(cells: &[&[&str]]) -> Vec<Vec<String>> {
        cells
            .iter()
            .map(|r| r.iter().map(|c| (*c).to_owned()).collect())
            .collect()
    }

    #[test]
    fn whole_numbers_fixed_decimals_and_anything_else_are_told_apart() {
        let (kinds, values) = read(&rows(&[
            &["Ada 101", "1", "12.500", "1.024", "12 m", "3"],
            &["Ada 102", "-2", "0.125", "512", "", "3.5"],
            &["Ada 103", "", "-7.000", "2.048", "—", "4"],
        ]));
        assert_eq!(
            kinds,
            [
                CellKind::Text,
                CellKind::Integer,
                CellKind::Real,
                CellKind::Text,
                CellKind::Text,
                CellKind::Text
            ]
        );
        assert_eq!(values[1][1], Value::from(-2));
        assert_eq!(values[2][1], Value::Null, "an empty cell is no value");
        assert_eq!(values[0][2], Value::from(12.5));
        // “1.024” with “512” is no fixed formatting (a grouped count): text, read as shown.
        assert_eq!(values[0][3], Value::String("1.024".into()));
        // A whole number and a decimal in one column: not one formatting, text.
        assert_eq!(values[1][5], Value::String("3.5".into()));
    }

    #[test]
    fn a_decimal_comma_a_plus_and_spaces_inside_are_no_numbers() {
        let (kinds, _) = read(&rows(&[&["1,5", "+3", "1 000"], &["2,5", "+4", "2 000"]]));
        assert_eq!(kinds, [CellKind::Text, CellKind::Text, CellKind::Text]);
    }

    #[test]
    fn a_code_with_a_leading_zero_and_a_number_past_64_bits_stay_text() {
        let (kinds, values) = read(&rows(&[
            &["007", "0", "0.50", "12345678901234567890", "00.5"],
            &["012", "10", "10.25", "1", "1.5"],
        ]));
        assert_eq!(
            kinds,
            [
                CellKind::Text,
                CellKind::Integer,
                CellKind::Real,
                CellKind::Text,
                CellKind::Text
            ]
        );
        assert_eq!(
            values[0][0],
            Value::String("007".into()),
            "a code keeps its zeros"
        );
        assert_eq!(values[0][1], Value::from(0));
        assert_eq!(values[0][2], Value::from(0.5));
    }
}
