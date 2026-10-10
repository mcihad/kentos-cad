//! Yeniden sınıflandır's table (docs/adr/0233 §4): one rule a line (lines
//! by a newline or `;`, numbers by spaces, a decimal `.` or `,`): `alt üst
//! yeni` a range, `değer yeni` one value, `boş yeni` the cells without a
//! value; `*` an open end; `yeni` may be `boş`. The first rule that holds
//! gives the cell its new value.

/// How a range's ends hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bounds {
    /// alt < değer ≤ üst (QGIS's default).
    UpperClosed,
    /// alt ≤ değer < üst.
    LowerClosed,
}

impl Bounds {
    pub fn from_key(key: &str) -> Option<Bounds> {
        Some(match key {
            "upperClosed" => Bounds::UpperClosed,
            "lowerClosed" => Bounds::LowerClosed,
            _ => return None,
        })
    }
}

/// A rule; `new` none makes the cell empty.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rule {
    /// An open end is none.
    Range {
        lo: Option<f64>,
        hi: Option<f64>,
        new: Option<f64>,
    },
    Value {
        x: f64,
        new: Option<f64>,
    },
    Empty {
        new: Option<f64>,
    },
}

/// A number as the table writes it (`.` or `,` for the decimal point).
fn number(word: &str) -> Option<f64> {
    let w = word.replace(',', ".");
    let ok = !w.is_empty()
        && w.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'));
    if !ok {
        return None;
    }
    w.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn is_empty_word(word: &str) -> bool {
    matches!(
        word,
        "boş" | "Boş" | "BOŞ" | "bos" | "null" | "NULL" | "nodata"
    )
}

/// A rule's new value: a number, `boş`, or one of the caller's words (`kısıt`, docs/adr/0237 §6).
fn new_value(word: &str, line: usize, words: &[(&str, f64)]) -> Result<Option<f64>, String> {
    if is_empty_word(word) {
        return Ok(None);
    }
    if let Some(&(_, v)) = words.iter().find(|(w, _)| *w == word) {
        return Ok(Some(v));
    }
    number(word).map(Some).ok_or_else(|| match words.first() {
        Some((w, _)) => {
            format!("Tablonun {line}. kuralında “{word}” bir sayı, boş ya da {w} değil.")
        }
        None => format!("Tablonun {line}. kuralında “{word}” bir sayı ya da boş değil."),
    })
}

/// An end of a range: a number or `*`.
fn end(word: &str, line: usize) -> Result<Option<f64>, String> {
    if word == "*" {
        return Ok(None);
    }
    number(word)
        .map(Some)
        .ok_or_else(|| format!("Tablonun {line}. kuralında “{word}” bir sayı ya da * değil."))
}

/// The table's rules, in order.
pub fn parse(text: &str) -> Result<Vec<Rule>, String> {
    parse_with(text, &[])
}

/// The table's rules, a new value also one of `keywords` (each standing for its number).
pub fn parse_with(text: &str, keywords: &[(&str, f64)]) -> Result<Vec<Rule>, String> {
    let mut rules = Vec::new();
    for (k, line) in text.split(['\n', ';']).enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let at = k + 1;
        let rule = match words.as_slice() {
            [] => continue,
            [first, new] if is_empty_word(first) => Rule::Empty {
                new: new_value(new, at, keywords)?,
            },
            [x, new] => Rule::Value {
                x: number(x).ok_or_else(|| {
                    format!("Tablonun {at}. kuralında “{x}” bir sayı ya da boş değil.")
                })?,
                new: new_value(new, at, keywords)?,
            },
            [lo, hi, new] => {
                let (lo, hi) = (end(lo, at)?, end(hi, at)?);
                if let (Some(a), Some(b)) = (lo, hi)
                    && a > b
                {
                    return Err(format!(
                        "Tablonun {at}. kuralında alt sınır üst sınırdan büyük."
                    ));
                }
                Rule::Range {
                    lo,
                    hi,
                    new: new_value(new, at, keywords)?,
                }
            }
            _ => {
                return Err(format!(
                    "Tablonun {at}. kuralı “alt üst yeni”, “değer yeni” ya da “boş yeni” olmalı."
                ));
            }
        };
        rules.push(rule);
    }
    if rules.is_empty() {
        return Err("Sınıflandırma tablosu boş: en az bir kural yazın (örnek: 0 100 1).".into());
    }
    Ok(rules)
}

/// What a cell becomes: `Some(new)` (none: empty) when a rule holds, else none.
#[inline]
pub fn apply(rules: &[Rule], bounds: Bounds, v: f64) -> Option<Option<f64>> {
    let empty = v.is_nan();
    for r in rules {
        match *r {
            Rule::Empty { new } if empty => return Some(new),
            Rule::Value { x, new } if !empty && v == x => return Some(new),
            Rule::Range { lo, hi, new } if !empty => {
                let above = match (lo, bounds) {
                    (None, _) => true,
                    (Some(a), Bounds::UpperClosed) => v > a,
                    (Some(a), Bounds::LowerClosed) => v >= a,
                };
                let below = match (hi, bounds) {
                    (None, _) => true,
                    (Some(b), Bounds::UpperClosed) => v <= b,
                    (Some(b), Bounds::LowerClosed) => v < b,
                };
                if above && below {
                    return Some(new);
                }
            }
            _ => {}
        }
    }
    None
}

/// The new values the rules write (for the type's range check).
pub fn new_values(rules: &[Rule]) -> impl Iterator<Item = f64> + '_ {
    rules.iter().filter_map(|r| match *r {
        Rule::Range { new, .. } | Rule::Value { new, .. } | Rule::Empty { new } => new,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_read_and_hold_in_order() {
        let r = parse("0 100 1; 100 200 2\n200 * 3\n* 0 boş; -5 9; boş 0").unwrap();
        assert_eq!(r.len(), 6);
        let up = Bounds::UpperClosed;
        assert_eq!(apply(&r, up, 50.0), Some(Some(1.0)));
        // 100 is in the first range (alt < değer ≤ üst).
        assert_eq!(apply(&r, up, 100.0), Some(Some(1.0)));
        assert_eq!(apply(&r, Bounds::LowerClosed, 100.0), Some(Some(2.0)));
        assert_eq!(apply(&r, up, 1e9), Some(Some(3.0)));
        // At or below 0: empty (the fourth rule); −5 never reaches the fifth.
        assert_eq!(apply(&r, up, -5.0), Some(None));
        assert_eq!(apply(&r, up, f64::NAN), Some(Some(0.0)));
        let r = parse("1,5 2,5 7").unwrap();
        assert_eq!(apply(&r, up, 2.0), Some(Some(7.0)));
        assert_eq!(apply(&r, up, 3.0), None);
    }

    #[test]
    fn bad_tables_say_where() {
        assert!(parse("").unwrap_err().contains("boş"));
        assert!(parse("0 1 a").unwrap_err().contains("1. kural"));
        assert!(parse("1 2; 5 1 3").unwrap_err().contains("2. kural"));
        assert!(parse("1 2 3 4").unwrap_err().contains("olmalı"));
    }
}
