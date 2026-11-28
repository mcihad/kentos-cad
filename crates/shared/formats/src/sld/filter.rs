//! The filter a style's rule holds, as the style readers and writers share it (SLD, QML, MapLibre): its model, its
//! reading from an OGC filter's text form and from a GIS expression, and its writing as KentOS's expression.

use super::num;

/// A filter as the writer and the reader share it.
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    Cmp(Op, String, Lit),
    Between(String, f64, f64),
    Null(String),
    Like(String, String),
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
}

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
    pub(super) fn ogc(self) -> &'static str {
        match self {
            Op::Eq => "PropertyIsEqualTo",
            Op::Ne => "PropertyIsNotEqualTo",
            Op::Lt => "PropertyIsLessThan",
            Op::Le => "PropertyIsLessThanOrEqualTo",
            Op::Gt => "PropertyIsGreaterThan",
            Op::Ge => "PropertyIsGreaterThanOrEqualTo",
        }
    }

    pub(super) fn of_ogc(name: &str) -> Option<Op> {
        Some(match name {
            "PropertyIsEqualTo" => Op::Eq,
            "PropertyIsNotEqualTo" => Op::Ne,
            "PropertyIsLessThan" => Op::Lt,
            "PropertyIsLessThanOrEqualTo" => Op::Le,
            "PropertyIsGreaterThan" => Op::Gt,
            "PropertyIsGreaterThanOrEqualTo" => Op::Ge,
            _ => return None,
        })
    }

    fn text(self) -> &'static str {
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

#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Num(f64),
    Text(String),
}

/// The words KentOS's expressions keep for themselves, which no bare field takes.
const KEYWORDS: [&str; 17] = [
    "ve",
    "veya",
    "değil",
    "boş",
    "gibi",
    "arasında",
    "and",
    "or",
    "not",
    "is",
    "null",
    "like",
    "between",
    "doğru",
    "yanlış",
    "true",
    "false",
];

/// Whether `w` is a bare field name of KentOS's expressions: a word of letters, digits and `_`, not a keyword.
pub fn bare_field(w: &str) -> bool {
    let mut chars = w.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && w.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&w.to_lowercase().as_str())
}

/// A field alone: `[Ad Soyad]` or a bare word `Ad`.
pub fn field_of(expr: &str) -> Option<String> {
    let e = expr.trim();
    if let Some(inner) = e.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let inner = inner.trim();
        return (!inner.is_empty() && !inner.contains([']', '['])).then(|| inner.to_owned());
    }
    bare_field(e).then(|| e.to_owned())
}

/// The tokens of the expressions the filter grammar can say.
#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Field(String),
    Num(f64),
    Text(String),
    Op(Op),
    Word(String),
    Open,
    Close,
    Comma,
}

fn tokens(src: &str, qgis: bool) -> Option<Vec<Tok>> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '[' {
            let j = c[i + 1..].iter().position(|&x| x == ']')? + i + 1;
            out.push(Tok::Field(
                c[i + 1..j].iter().collect::<String>().trim().to_owned(),
            ));
            i = j + 1;
        } else if ch == '\'' || ch == '"' {
            // A text in either quote, the quote doubled inside it (as KentOS's lexer reads it); in a GIS reader's
            // expressions a double-quoted word is a field.
            let q = ch;
            let mut s = String::new();
            let mut j = i + 1;
            loop {
                let x = *c.get(j)?;
                if x == q {
                    if c.get(j + 1) == Some(&q) {
                        s.push(q);
                        j += 2;
                        continue;
                    }
                    break;
                }
                s.push(x);
                j += 1;
            }
            out.push(if qgis && q == '"' {
                Tok::Field(s)
            } else {
                Tok::Text(s)
            });
            i = j + 1;
        } else if ch.is_ascii_digit()
            || (ch == '-' && c.get(i + 1).is_some_and(char::is_ascii_digit))
            || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let mut j = i + 1;
            while j < c.len()
                && (c[j].is_ascii_digit() || c[j] == '.' || c[j] == 'e' || c[j] == 'E')
            {
                j += 1;
            }
            out.push(Tok::Num(c[i..j].iter().collect::<String>().parse().ok()?));
            i = j;
        } else if ch == '(' {
            out.push(Tok::Open);
            i += 1;
        } else if ch == ')' {
            out.push(Tok::Close);
            i += 1;
        } else if ch == ',' {
            out.push(Tok::Comma);
            i += 1;
        } else if "<>=!".contains(ch) {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            let (op, n) = match two.as_str() {
                "<=" => (Op::Le, 2),
                ">=" => (Op::Ge, 2),
                "<>" | "!=" => (Op::Ne, 2),
                "==" => (Op::Eq, 2),
                _ => match ch {
                    '<' => (Op::Lt, 1),
                    '>' => (Op::Gt, 1),
                    '=' => (Op::Eq, 1),
                    _ => return None,
                },
            };
            out.push(Tok::Op(op));
            i += n;
        } else if ch.is_alphabetic() || ch == '_' {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
                j += 1;
            }
            let w: String = c[i..j].iter().collect();
            // a GIS reader's `IN`, which is no word of KentOS's.
            if qgis && w.eq_ignore_ascii_case("in") {
                out.push(Tok::Word("in".into()));
            } else if bare_field(&w) {
                out.push(Tok::Field(w));
            } else {
                out.push(Tok::Word(w.to_lowercase()));
            }
            i = j;
        } else {
            return None;
        }
    }
    Some(out)
}

/// The expression as a filter, when the grammar can say it: comparisons of a field with a number or a text,
/// `arasında`/`between`, `boş`/`is null`, `gibi`/`like`, joined by `ve`/`and`, `veya`/`or`, `değil`/`not` and
/// parentheses.
pub fn parse_filter(src: &str) -> Option<Filter> {
    let toks = tokens(src, false)?;
    let mut at = 0;
    let f = or_of(&toks, &mut at, 0)?;
    (at == toks.len()).then_some(f)
}

/// A GIS expression as a filter, when the grammar can say it: `"field"` and bare words are fields, `'text'`
/// texts, `AND`, `OR`, `NOT`, `IS NULL`, `LIKE`, `BETWEEN … AND …` and `IN (…)` (one comparison each, joined by OR).
pub fn parse_qgis(src: &str) -> Option<Filter> {
    let toks = tokens(src, true)?;
    let mut at = 0;
    let f = or_of(&toks, &mut at, 0)?;
    (at == toks.len()).then_some(f)
}

fn word(t: Option<&Tok>, words: &[&str]) -> bool {
    matches!(t, Some(Tok::Word(w)) if words.contains(&w.as_str()))
}

/// The most levels a filter nests (brackets and `not`s): a deeper one is not read (none), as one the grammar cannot
/// say. The readers call themselves a level at a time; a hostile style file would overflow the stack (docs/adr/9401 §2).
const MAX_FILTER_DEPTH: usize = 64;

fn or_of(t: &[Tok], at: &mut usize, depth: usize) -> Option<Filter> {
    let mut parts = vec![and_of(t, at, depth)?];
    while word(t.get(*at), &["veya", "or"]) {
        *at += 1;
        parts.push(and_of(t, at, depth)?);
    }
    Some(if parts.len() == 1 {
        parts.remove(0)
    } else {
        Filter::Or(parts)
    })
}

fn and_of(t: &[Tok], at: &mut usize, depth: usize) -> Option<Filter> {
    let mut parts = vec![not_of(t, at, depth)?];
    while word(t.get(*at), &["ve", "and"]) {
        *at += 1;
        parts.push(not_of(t, at, depth)?);
    }
    Some(if parts.len() == 1 {
        parts.remove(0)
    } else {
        Filter::And(parts)
    })
}

fn not_of(t: &[Tok], at: &mut usize, depth: usize) -> Option<Filter> {
    if depth > MAX_FILTER_DEPTH {
        return None;
    }
    if word(t.get(*at), &["değil", "not"]) {
        *at += 1;
        return Some(Filter::Not(Box::new(not_of(t, at, depth + 1)?)));
    }
    if t.get(*at) == Some(&Tok::Open) {
        *at += 1;
        let f = or_of(t, at, depth + 1)?;
        if t.get(*at) != Some(&Tok::Close) {
            return None;
        }
        *at += 1;
        return Some(f);
    }
    let Some(Tok::Field(name)) = t.get(*at) else {
        return None;
    };
    let name = name.clone();
    *at += 1;
    match t.get(*at) {
        Some(Tok::Op(op)) => {
            let op = *op;
            *at += 1;
            let lit = match t.get(*at)? {
                Tok::Num(n) => Lit::Num(*n),
                Tok::Text(s) => Lit::Text(s.clone()),
                _ => return None,
            };
            *at += 1;
            Some(Filter::Cmp(op, name, lit))
        }
        Some(Tok::Word(w)) if w == "boş" || w == "is" => {
            *at += 1;
            if w == "is" {
                if word(t.get(*at), &["null"]) {
                    *at += 1;
                    return Some(Filter::Null(name));
                }
                if word(t.get(*at), &["not"]) && word(t.get(*at + 1), &["null"]) {
                    *at += 2;
                    return Some(Filter::Not(Box::new(Filter::Null(name))));
                }
                return None;
            }
            if word(t.get(*at), &["değil"]) {
                *at += 1;
                return Some(Filter::Not(Box::new(Filter::Null(name))));
            }
            Some(Filter::Null(name))
        }
        Some(Tok::Word(w)) if w == "gibi" || w == "like" => {
            *at += 1;
            let Some(Tok::Text(p)) = t.get(*at) else {
                return None;
            };
            *at += 1;
            Some(Filter::Like(name, p.clone()))
        }
        Some(Tok::Word(w)) if w == "in" => {
            *at += 1;
            if t.get(*at) != Some(&Tok::Open) {
                return None;
            }
            *at += 1;
            let mut parts = Vec::new();
            loop {
                let lit = match t.get(*at)? {
                    Tok::Num(n) => Lit::Num(*n),
                    Tok::Text(s) => Lit::Text(s.clone()),
                    _ => return None,
                };
                parts.push(Filter::Cmp(Op::Eq, name.clone(), lit));
                *at += 1;
                match t.get(*at)? {
                    Tok::Comma => *at += 1,
                    Tok::Close => {
                        *at += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            Some(if parts.len() == 1 {
                parts.remove(0)
            } else {
                Filter::Or(parts)
            })
        }
        Some(Tok::Word(w)) if w == "arasında" || w == "between" => {
            *at += 1;
            let Some(Tok::Num(a)) = t.get(*at) else {
                return None;
            };
            if !word(t.get(*at + 1), &["ve", "and"]) {
                return None;
            }
            let Some(Tok::Num(b)) = t.get(*at + 2) else {
                return None;
            };
            *at += 3;
            Some(Filter::Between(name, *a, *b))
        }
        _ => None,
    }
}

/// A filter as KentOS's expression.
pub fn filter_text(f: &Filter) -> String {
    let lit = |l: &Lit| match l {
        Lit::Num(n) => num(*n),
        Lit::Text(s) => format!("'{}'", s.replace('\'', "''")),
    };
    match f {
        Filter::Cmp(op, field, l) => format!("[{field}] {} {}", op.text(), lit(l)),
        Filter::Between(field, a, b) => format!("[{field}] arasında {} ve {}", num(*a), num(*b)),
        Filter::Null(field) => format!("[{field}] boş"),
        Filter::Like(field, p) => format!("[{field}] gibi '{}'", p.replace('\'', "''")),
        Filter::And(parts) => parts
            .iter()
            .map(|p| wrap(p, true))
            .collect::<Vec<_>>()
            .join(" ve "),
        Filter::Or(parts) => parts
            .iter()
            .map(|p| wrap(p, false))
            .collect::<Vec<_>>()
            .join(" veya "),
        Filter::Not(inner) => match &**inner {
            Filter::Null(field) => format!("[{field}] boş değil"),
            other => format!("değil ({})", filter_text(other)),
        },
    }
}

fn wrap(f: &Filter, in_and: bool) -> String {
    match f {
        Filter::Or(_) if in_and => format!("({})", filter_text(f)),
        Filter::And(_) if !in_and => format!("({})", filter_text(f)),
        _ => filter_text(f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A filter nested past the limit is not read, on a thread as small as a worker's: before the limit a style
    /// file's 100 000 brackets overflowed the stack (docs/adr/9401 §2).
    #[test]
    fn a_filter_nested_past_the_limit_is_not_read() {
        let nested = |n: usize| format!("{}\"a\" = 1{}", "(".repeat(n), ")".repeat(n));
        let on_a_small_thread = |source: String| {
            std::thread::Builder::new()
                .stack_size(2 << 20)
                .spawn(move || {
                    (
                        parse_qgis(&source).is_some(),
                        parse_filter(&source).is_some(),
                    )
                })
                .expect("a thread")
                .join()
                .expect("neither an overflow nor a panic")
        };
        assert_eq!(on_a_small_thread(nested(100_000)), (false, false));
        assert!(!on_a_small_thread(format!("{}\"a\" = 1", "not ".repeat(100_000))).0);
        assert!(parse_qgis(&nested(MAX_FILTER_DEPTH)).is_some());
        assert!(parse_qgis(&nested(MAX_FILTER_DEPTH + 1)).is_none());
    }
}
