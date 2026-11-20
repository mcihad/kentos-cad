//! Which rows a request keeps (docs/adr/9100 §4): a condition over a table's fields as a tree, not as SQL, so every
//! driver writes it in its own words (PostgreSQL's `ILIKE`, SQLite's folded `LIKE`, Oracle's `UPPER(…) LIKE`).
//! Every node has one meaning whatever the source:
//!
//! - a comparison of a field with a value is false for a NULL cell (SQL's three values: NULL is neither kept nor
//!   its opposite kept; `Not` of an unknown is unknown);
//! - `Like`'s `%` is any run and `_` one character, `\` escapes them; `fold` compares Turkish letters without case
//!   (`İ`/`i`, `I`/`ı` are each a pair);
//! - `Contains` keeps a row where any of the fields, read as text, holds the words (`fold`ed), the table's quick
//!   search;
//! - `Fids` keeps the rows of those keys.
//!
//! [`Condition::holds`] is the meaning written in Rust: the conformance suite asks a driver's answer of it.

use crate::value::Value;

/// A comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    /// Its SQL sign.
    pub fn sql(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Ne => "<>",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
        }
    }
}

/// A condition on a row.
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    Compare {
        field: String,
        op: Op,
        value: Value,
    },
    In {
        field: String,
        values: Vec<Value>,
    },
    Between {
        field: String,
        low: Value,
        high: Value,
    },
    IsNull(String),
    NotNull(String),
    Like {
        field: String,
        pattern: String,
        fold: bool,
    },
    Contains {
        fields: Vec<String>,
        text: String,
    },
    Fids(Vec<i64>),
}

impl Condition {
    pub fn eq(field: &str, value: impl Into<Value>) -> Condition {
        Condition::Compare {
            field: field.to_owned(),
            op: Op::Eq,
            value: value.into(),
        }
    }

    pub fn cmp(field: &str, op: Op, value: impl Into<Value>) -> Condition {
        Condition::Compare {
            field: field.to_owned(),
            op,
            value: value.into(),
        }
    }

    /// Both (a list of one is that one).
    pub fn and(parts: Vec<Condition>) -> Condition {
        if parts.len() == 1 {
            parts
                .into_iter()
                .next()
                .unwrap_or(Condition::And(Vec::new()))
        } else {
            Condition::And(parts)
        }
    }

    /// The fields it reads, each once, in the order met.
    pub fn fields(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        fn push(out: &mut Vec<String>, f: &str) {
            if !out.iter().any(|x| x == f) {
                out.push(f.to_owned());
            }
        }
        fn walk(c: &Condition, out: &mut Vec<String>) {
            match c {
                Condition::And(cs) | Condition::Or(cs) => cs.iter().for_each(|c| walk(c, out)),
                Condition::Not(c) => walk(c, out),
                Condition::Compare { field, .. }
                | Condition::In { field, .. }
                | Condition::Between { field, .. }
                | Condition::Like { field, .. } => push(out, field),
                Condition::IsNull(f) | Condition::NotNull(f) => push(out, f),
                Condition::Contains { fields, .. } => fields.iter().for_each(|f| push(out, f)),
                Condition::Fids(_) => {}
            }
        }
        walk(self, &mut out);
        out
    }

    /// Whether the row (its key and its values by field name) is kept: `Some(true)` kept, `Some(false)` left out,
    /// `None` unknown (a NULL met; left out too, as SQL's `WHERE` does).
    pub fn holds(&self, fid: i64, get: &dyn Fn(&str) -> Option<Value>) -> Option<bool> {
        match self {
            Condition::And(cs) => {
                let mut unknown = false;
                for c in cs {
                    match c.holds(fid, get) {
                        Some(false) => return Some(false),
                        None => unknown = true,
                        Some(true) => {}
                    }
                }
                if unknown { None } else { Some(true) }
            }
            Condition::Or(cs) => {
                let mut unknown = false;
                for c in cs {
                    match c.holds(fid, get) {
                        Some(true) => return Some(true),
                        None => unknown = true,
                        Some(false) => {}
                    }
                }
                if unknown { None } else { Some(false) }
            }
            Condition::Not(c) => c.holds(fid, get).map(|b| !b),
            Condition::Compare { field, op, value } => {
                let cell = get(field).unwrap_or(Value::Null);
                compare(&cell, value).map(|o| match op {
                    Op::Eq => o == std::cmp::Ordering::Equal,
                    Op::Ne => o != std::cmp::Ordering::Equal,
                    Op::Lt => o == std::cmp::Ordering::Less,
                    Op::Le => o != std::cmp::Ordering::Greater,
                    Op::Gt => o == std::cmp::Ordering::Greater,
                    Op::Ge => o != std::cmp::Ordering::Less,
                })
            }
            Condition::In { field, values } => {
                let cell = get(field).unwrap_or(Value::Null);
                if cell.is_null() {
                    return None;
                }
                Some(
                    values
                        .iter()
                        .any(|v| compare(&cell, v) == Some(std::cmp::Ordering::Equal)),
                )
            }
            Condition::Between { field, low, high } => {
                let cell = get(field).unwrap_or(Value::Null);
                let a = compare(&cell, low)?;
                let b = compare(&cell, high)?;
                Some(a != std::cmp::Ordering::Less && b != std::cmp::Ordering::Greater)
            }
            Condition::IsNull(f) => Some(get(f).is_none_or(|v| v.is_null())),
            Condition::NotNull(f) => Some(get(f).is_some_and(|v| !v.is_null())),
            Condition::Like {
                field,
                pattern,
                fold: f,
            } => {
                let cell = get(field).unwrap_or(Value::Null);
                if cell.is_null() {
                    return None;
                }
                let text = cell.to_string();
                Some(if *f {
                    like(&fold(&text), &fold(pattern))
                } else {
                    like(&text, pattern)
                })
            }
            Condition::Contains { fields, text } => {
                let want = fold(text);
                Some(fields.iter().any(|f| {
                    get(f).is_some_and(|v| !v.is_null() && fold(&v.to_string()).contains(&want))
                }))
            }
            Condition::Fids(fids) => Some(fids.contains(&fid)),
        }
    }
}

/// Two values' order when they can be ordered: numbers by value, texts (and dates, which are ISO text) by their
/// characters; none with a NULL or between a number and a text.
pub fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    if a.is_null() || b.is_null() {
        return None;
    }
    match (a.number(), b.number()) {
        (Some(x), Some(y)) => x.partial_cmp(&y),
        (None, None) => Some(a.to_string().cmp(&b.to_string())),
        _ => None,
    }
}

/// Text folded for a comparison without case: Turkish letters by Turkish rules (`İ` → `i`, `I` → `ı`), others lower.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            'İ' => out.push('i'),
            'I' => out.push('ı'),
            c => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// SQL `LIKE` on whole strings: `%` any run, `_` one character, `\` the next character as it is.
pub fn like(text: &str, pattern: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pattern.chars().collect();
    // Iterative match with the last `%`'s place to come back to.
    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() {
            match p[pi] {
                '%' => {
                    star = Some((pi, ti));
                    pi += 1;
                    continue;
                }
                '\\' if pi + 1 < p.len() => {
                    if p[pi + 1] == t[ti] {
                        pi += 2;
                        ti += 1;
                        continue;
                    }
                }
                '_' => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                c if c == t[ti] => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                _ => {}
            }
        }
        match star {
            Some((sp, st)) => {
                pi = sp + 1;
                ti = st + 1;
                star = Some((sp, st + 1));
            }
            None => return false,
        }
    }
    while pi < p.len() && p[pi] == '%' {
        pi += 1;
    }
    pi == p.len()
}

/// `text` as a `LIKE` pattern that matches it literally (its `%`, `_` and `\` escaped).
pub fn escape_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(fid: i64) -> impl Fn(&str) -> Option<Value> {
        move |f: &str| match f {
            "ad" => Some(Value::Text(if fid == 1 {
                "Işık Sokağı".into()
            } else {
                "İnönü Caddesi".into()
            })),
            "alan" => Some(if fid == 3 {
                Value::Null
            } else {
                Value::Real(100.0 * fid as f64)
            }),
            _ => None,
        }
    }

    #[test]
    fn nulls_are_neither_kept_nor_their_opposite() {
        let c = Condition::cmp("alan", Op::Gt, 150.0);
        assert_eq!(c.holds(1, &row(1)), Some(false));
        assert_eq!(c.holds(2, &row(2)), Some(true));
        assert_eq!(c.holds(3, &row(3)), None);
        assert_eq!(Condition::Not(Box::new(c.clone())).holds(3, &row(3)), None);
        let or = Condition::Or(vec![c, Condition::eq("ad", "İnönü Caddesi")]);
        assert_eq!(or.holds(3, &row(3)), Some(true));
    }

    #[test]
    fn turkish_letters_fold_in_pairs() {
        assert_eq!(fold("IŞIK"), "ışık");
        assert_eq!(fold("İNÖNÜ"), "inönü");
        let c = Condition::Contains {
            fields: vec!["ad".into()],
            text: "IŞIK".into(),
        };
        assert_eq!(c.holds(1, &row(1)), Some(true));
        assert_eq!(c.holds(2, &row(2)), Some(false));
    }

    #[test]
    fn like_has_runs_ones_and_escapes() {
        assert!(like("İnönü Caddesi", "%Cadde%"));
        assert!(like("ab", "a_"));
        assert!(!like("abc", "a_"));
        assert!(like("50%", "50\\%"));
        assert!(!like("500", "50\\%"));
        assert!(like("", "%"));
        assert_eq!(escape_like("a_b%"), "a\\_b\\%");
    }
}
