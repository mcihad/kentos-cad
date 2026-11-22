//! A cell's value as every driver hands it (docs/adr/9100 §3): a source's own types brought to a few kinds a table,
//! a style and an expression can read the same way. Dates and times stay text in ISO 8601 (`2026-11-20`,
//! `2026-11-20T21:45:23`, with the source's offset when it gives one), so no source loses a digit on the way; a
//! decimal (`numeric`) the source can hold exactly travels as its text too.

use std::fmt;

/// A value of a cell.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    /// A decimal number as the source writes it (`12.50`): exact, read as a number only when asked.
    Decimal(String),
    Text(String),
    /// `YYYY-MM-DD`.
    Date(String),
    /// `YYYY-MM-DDTHH:MM:SS[.fff][±HH:MM|Z]`.
    DateTime(String),
    /// `HH:MM:SS[.fff]`.
    Time(String),
    Blob(Vec<u8>),
    /// A JSON document as its text.
    Json(String),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The value as a number, when it is one (an integer, a real, a decimal, true as 1 and false as 0).
    pub fn number(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Real(r) => Some(*r),
            Value::Decimal(d) => d.trim().parse::<f64>().ok(),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// The value as an integer, when it is a whole number.
    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Bool(b) => Some(i64::from(*b)),
            Value::Real(r) if r.fract() == 0.0 && r.abs() < 9.0e15 => Some(*r as i64),
            Value::Decimal(d) => d.trim().parse::<i64>().ok(),
            _ => None,
        }
    }

    /// The kind's name in the interface: “metin”, “tam sayı” …
    pub fn kind_words(&self) -> &'static str {
        match self {
            Value::Null => "boş",
            Value::Bool(_) => "evet/hayır",
            Value::Int(_) => "tam sayı",
            Value::Real(_) => "ondalık sayı",
            Value::Decimal(_) => "kesin ondalık",
            Value::Text(_) => "metin",
            Value::Date(_) => "tarih",
            Value::DateTime(_) => "tarih ve saat",
            Value::Time(_) => "saat",
            Value::Blob(_) => "ikili veri",
            Value::Json(_) => "JSON",
        }
    }
}

impl fmt::Display for Value {
    /// The value as text: what a table cell shows before the field's own format, and what a search looks in. A
    /// blob says its size.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => Ok(()),
            Value::Bool(b) => f.write_str(if *b { "evet" } else { "hayır" }),
            Value::Int(i) => write!(f, "{i}"),
            Value::Real(r) => write!(f, "{r}"),
            Value::Decimal(t)
            | Value::Text(t)
            | Value::Date(t)
            | Value::DateTime(t)
            | Value::Time(t)
            | Value::Json(t) => f.write_str(t),
            Value::Blob(b) => write!(f, "{} bayt", b.len()),
        }
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Real(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_owned())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_from_every_kind_that_holds_one() {
        assert_eq!(Value::Int(3).number(), Some(3.0));
        assert_eq!(Value::Decimal("12.50".into()).number(), Some(12.5));
        assert_eq!(Value::Bool(true).int(), Some(1));
        assert_eq!(Value::Real(4.0).int(), Some(4));
        assert_eq!(Value::Real(4.5).int(), None);
        assert_eq!(Value::Text("4".into()).number(), None);
        assert_eq!(Value::Null.to_string(), "");
        assert_eq!(Value::Blob(vec![1, 2]).to_string(), "2 bayt");
    }
}
