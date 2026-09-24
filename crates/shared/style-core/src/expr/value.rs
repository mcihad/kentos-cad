//! Values of the expression language and their rules (the TypeScript's
//! `expressionLib.ts`): attribute values are text, so arithmetic reads
//! numbers out of text ("452.13" → 452.13, the decimal separator is the
//! dot) and an empty or missing value is empty (null).

use std::borrow::Cow;
use std::cmp::Ordering;

use kentos_geometry_core::jsmath::js_max;

use crate::js::{collate, number, text};

/// A value. Text borrows where it can (an attribute from the table, a
/// literal from the source), so evaluating an object allocates only for
/// text it makes.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Value<'a> {
    #[default]
    Null,
    Num(f64),
    Text(Cow<'a, str>),
    Bool(bool),
}

impl<'a> Value<'a> {
    pub fn text(s: impl Into<Cow<'a, str>>) -> Value<'a> {
        Value::Text(s.into())
    }

    /// The value without borrowing.
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Null => Value::Null,
            Value::Num(x) => Value::Num(x),
            Value::Text(s) => Value::Text(Cow::Owned(s.into_owned())),
            Value::Bool(b) => Value::Bool(b),
        }
    }
}

/// `^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$` on ASCII digits.
fn numeric(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - start
    };
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let whole = digits(&mut i);
    if whole > 0 {
        if i < b.len() && b[i] == b'.' {
            i += 1;
            digits(&mut i);
        }
    } else {
        if !(i < b.len() && b[i] == b'.') {
            return false;
        }
        i += 1;
        if digits(&mut i) == 0 {
            return false;
        }
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return false;
        }
    }
    i == b.len()
}

/// A number, or None when the value is empty or not a number. A number
/// written in text passes as it is (even "1e400", which is infinite).
pub fn to_number(v: &Value) -> Option<f64> {
    match v {
        Value::Num(x) => x.is_finite().then_some(*x),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::Text(s) => {
            let t = text::trim(s);
            if numeric(t) { t.parse().ok() } else { None }
        }
        Value::Null => None,
    }
}

pub fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Text(s) => s.is_empty(),
        _ => false,
    }
}

pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        // NaN is not 0.
        Value::Num(x) => *x != 0.0,
        Value::Text(s) => !s.is_empty(),
    }
}

/// Number to text for attributes: integers as they are, others to 12
/// significant digits, so float noise is dropped (0.1 + 0.2 → "0.3").
pub fn number_text(x: f64) -> String {
    if !x.is_finite() || x.trunc() == x {
        return number::to_string(x);
    }
    number::to_string_precision(x, 12)
}

/// Text for writing into an attribute (borrowed from a text value).
pub fn to_text<'b>(v: &'b Value<'_>) -> Cow<'b, str> {
    match v {
        Value::Null => Cow::Borrowed(""),
        Value::Bool(b) => Cow::Borrowed(if *b { "doğru" } else { "yanlış" }),
        Value::Num(x) => Cow::Owned(number_text(*x)),
        Value::Text(s) => Cow::Borrowed(s),
    }
}

/// The value's text, keeping a text value's own (no copy).
pub fn into_text(v: Value<'_>) -> Cow<'_, str> {
    match v {
        Value::Text(s) => s,
        v => Cow::Owned(to_text(&v).into_owned()),
    }
}

/// Equality: numbers by value (within 1e-9, relative above 1), text exactly,
/// and "empty" equals only "empty".
pub fn equals(a: &Value, b: &Value) -> bool {
    if is_empty(a) || is_empty(b) {
        return is_empty(a) && is_empty(b);
    }
    if matches!(a, Value::Bool(_)) || matches!(b, Value::Bool(_)) {
        return truthy(a) == truthy(b);
    }
    if let (Some(na), Some(nb)) = (to_number(a), to_number(b)) {
        return (na - nb).abs() <= 1e-9 * js_max(js_max(1.0, na.abs()), nb.abs());
    }
    to_text(a) == to_text(b)
}

/// Order of two values: numbers numerically (their difference, NaN for ∞ − ∞),
/// text in Turkish order; None when one is empty.
pub fn compare(a: &Value, b: &Value) -> Option<f64> {
    if is_empty(a) || is_empty(b) {
        return None;
    }
    if let (Some(na), Some(nb)) = (to_number(a), to_number(b)) {
        return Some(na - nb);
    }
    Some(match collate::compare_tr(&to_text(a), &to_text(b)) {
        Ordering::Less => -1.0,
        Ordering::Equal => 0.0,
        Ordering::Greater => 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_out_of_text() {
        for (s, n) in [
            ("452.13", Some(452.13)),
            (" 12 ", Some(12.0)),
            ("+.5", Some(0.5)),
            ("5.", Some(5.0)),
            ("-0", Some(-0.0)),
            ("1e400", Some(f64::INFINITY)),
            ("1,5", None),
            ("", None),
            ("abc", None),
            ("inf", None),
            ("0x10", None),
            (".", None),
            ("1e", None),
        ] {
            assert_eq!(to_number(&Value::text(s)), n, "{s:?}");
        }
    }

    #[test]
    fn text_of_numbers() {
        assert_eq!(number_text(0.1 + 0.2), "0.3");
        assert_eq!(number_text(600.0), "600");
        assert_eq!(number_text(-0.0), "0");
        assert_eq!(number_text(1234567890.125), "1234567890.13");
        assert_eq!(number_text(1e21), "1e+21");
        assert_eq!(number_text(f64::INFINITY), "Infinity");
        assert_eq!(number_text(f64::NAN), "NaN");
        assert_eq!(to_text(&Value::Bool(true)), "doğru");
    }

    #[test]
    fn equality_and_order() {
        assert!(equals(&Value::text("12"), &Value::Num(12.0)));
        assert!(equals(&Value::Null, &Value::text("")));
        assert!(!equals(&Value::text("Arsa"), &Value::text("arsa")));
        assert_eq!(
            compare(&Value::text("Arsa"), &Value::text("Bahçe")),
            Some(-1.0)
        );
        assert_eq!(compare(&Value::text(""), &Value::Num(1.0)), None);
        // "1e400" reads as ∞, and ∞ − ∞ is NaN (no order), as in the TypeScript.
        assert!(compare(&Value::text("1e400"), &Value::text("1e400")).is_some_and(f64::is_nan));
        // A number that is not finite is no number: it compares as text.
        assert_eq!(
            compare(&Value::Num(f64::INFINITY), &Value::text("1e400")),
            Some(1.0)
        );
    }
}
