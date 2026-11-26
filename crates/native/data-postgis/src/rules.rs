//! A column's rule in a database (docs/adr/9324): a check constraint `kentos_<column>` that every later write keeps
//! (`NOT VALID`: rows already there are not checked), and beside it, as the constraint's comment, the rule with its
//! values' labels in KentOS's own short text, which is read back. Other programs see the constraint.
//!
//! The comment's text: its first line `kentos-kural/1`, the column and `degerler` or `aralik`, by tabs; then a list's
//! values a line each (the value, a tab, its label), or a range's one line (least, whether it is in, most, whether it
//! is in; an open end empty). A tab, a line's end and a backslash in a value or a label are written `\t`, `\n`, `\\`.

use kentos_data::{ColumnRule, RuledColumn};

const HEAD: &str = "kentos-kural/1";

/// The constraint's name for `column`: `kentos_` and the column, within a database name's 63 bytes.
pub(crate) fn constraint_name(column: &str) -> String {
    let mut name = format!("kentos_{column}");
    while name.len() > 63 {
        name.pop();
    }
    name
}

/// A text as an SQL literal.
pub(crate) fn literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// A number as SQL writes it: the shortest digits that read back.
fn number(v: f64) -> String {
    format!("{v}")
}

/// The check that keeps `column` (quoted) to `rule`; an empty value always passes.
pub(crate) fn check_sql(column: &str, rule: &ColumnRule) -> String {
    match rule {
        ColumnRule::Values(values) => {
            let list: Vec<String> = values.iter().map(|(v, _)| literal(v)).collect();
            format!("{column} IS NULL OR {column} IN ({})", list.join(", "))
        }
        ColumnRule::Range { min, min_included, max, max_included } => {
            let mut parts = Vec::new();
            if let Some(m) = min {
                parts.push(format!("{column} {} {}", if *min_included { ">=" } else { ">" }, number(*m)));
            }
            if let Some(m) = max {
                parts.push(format!("{column} {} {}", if *max_included { "<=" } else { "<" }, number(*m)));
            }
            if parts.is_empty() {
                return "TRUE".to_owned();
            }
            format!("{column} IS NULL OR ({})", parts.join(" AND "))
        }
    }
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// The comment that keeps `rule` of `column` with its labels.
pub(crate) fn comment_of(column: &str, rule: &ColumnRule) -> String {
    match rule {
        ColumnRule::Values(values) => {
            let mut lines = vec![format!("{HEAD}\t{}\tdegerler", escape(column))];
            lines.extend(values.iter().map(|(v, l)| format!("{}\t{}", escape(v), escape(l.as_deref().unwrap_or("")))));
            lines.join("\n")
        }
        ColumnRule::Range { min, min_included, max, max_included } => {
            let end = |v: &Option<f64>| v.map(number).unwrap_or_default();
            format!(
                "{HEAD}\t{}\taralik\n{}\t{}\t{}\t{}",
                escape(column),
                end(min),
                u8::from(*min_included),
                end(max),
                u8::from(*max_included)
            )
        }
    }
}

/// The rule a comment keeps; none for a comment KentOS did not write.
pub(crate) fn rule_of(comment: &str) -> Option<RuledColumn> {
    let mut lines = comment.split('\n');
    let head: Vec<&str> = lines.next()?.split('\t').collect();
    let [HEAD, column, kind] = head.as_slice() else {
        return None;
    };
    let rule = match *kind {
        "degerler" => ColumnRule::Values(
            lines
                .filter(|l| !l.is_empty())
                .map(|l| {
                    let (v, label) = l.split_once('\t').unwrap_or((l, ""));
                    (unescape(v), Some(unescape(label)).filter(|l| !l.is_empty()))
                })
                .collect(),
        ),
        "aralik" => {
            let parts: Vec<&str> = lines.next()?.split('\t').collect();
            let [min, min_in, max, max_in] = parts.as_slice() else {
                return None;
            };
            ColumnRule::Range {
                min: min.parse().ok(),
                min_included: *min_in == "1",
                max: max.parse().ok(),
                max_included: *max_in == "1",
            }
        }
        _ => return None,
    };
    Some(RuledColumn { column: unescape(column), rule: Some(rule) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_goes_into_its_comment_and_comes_back_whole() {
        let list = ColumnRule::Values(vec![
            ("A".into(), Some("Aktif".into())),
            ("B\tx".into(), Some("Satır\nsonu".into())),
            ("C".into(), None),
        ]);
        let range = ColumnRule::Range { min: Some(1.5), min_included: true, max: None, max_included: false };
        for rule in [list, range] {
            let back = rule_of(&comment_of("durum", &rule)).expect("read back");
            assert_eq!(back, RuledColumn { column: "durum".into(), rule: Some(rule.clone()) });
        }
        assert_eq!(rule_of("Başka bir açıklama"), None);
        assert_eq!(check_sql("\"kod\"", &ColumnRule::Values(vec![("A'B".into(), None)])), "\"kod\" IS NULL OR \"kod\" IN ('A''B')");
        assert_eq!(constraint_name(&"x".repeat(80)).len(), 63);
    }
}
