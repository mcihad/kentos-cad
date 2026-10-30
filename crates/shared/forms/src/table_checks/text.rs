//! Text and pattern rules of the two databases (docs/adr/0755 §3): SQLite's numeric text, its `LIKE` (ASCII letters'
//! case aside) and `GLOB`, PostgreSQL's `LIKE` with its escape and its regular expressions where they mean what
//! `regex-lite` means; elsewhere none (not known).

use super::value::{Dec, Real, V};

/// A text SQLite reads as a number whole, blanks around it allowed (a column's numeric affinity, arithmetic): a whole
/// number that fits is an integer, any other a real; none for another text (hexadecimal is not read so).
pub(super) fn sqlite_number(t: &str) -> Option<V> {
    let s = t.trim_matches(|c: char| c.is_ascii_whitespace());
    let b = s.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let (mut digits, mut point, mut exp) = (0, false, false);
    while i < b.len() {
        match b[i] {
            b'0'..=b'9' => digits += 1,
            b'.' if !point && !exp => point = true,
            b'e' | b'E' if digits > 0 && !exp => {
                exp = true;
                if matches!(b.get(i + 1), Some(b'+' | b'-')) {
                    i += 1;
                }
                if !b.get(i + 1).is_some_and(u8::is_ascii_digit) {
                    return None;
                }
            }
            _ => return None,
        }
        i += 1;
    }
    if digits == 0 {
        return None;
    }
    if !point
        && !exp
        && let Ok(n) = s.parse::<i64>()
    {
        return Some(V::Int(n));
    }
    Real::parse(s).map(V::Real)
}

/// How SQLite's text of a value reads where a text is wanted (`LIKE`, `||`, `length`): an integer's digits; none for a
/// real, whose text SQLite writes its own way, and for what is not a text.
pub(super) fn sqlite_text(v: &V) -> Option<String> {
    match v {
        V::Text(t) => Some(t.clone()),
        V::Int(i) => Some(i.to_string()),
        _ => None,
    }
}

/// A pattern's parts.
enum P {
    One,
    Many,
    Lit(char),
    Set {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}

/// Whether `s` matches the parts, `eq` telling two characters the same.
fn matches(s: &[char], p: &[P], eq: impl Fn(char, char) -> bool) -> bool {
    let (mut i, mut j) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while i < s.len() {
        if let Some(part) = p.get(j) {
            let step = match part {
                P::Many => {
                    star = Some((j, i));
                    j += 1;
                    continue;
                }
                P::One => true,
                P::Lit(c) => eq(*c, s[i]),
                P::Set { negated, ranges } => {
                    ranges.iter().any(|(a, b)| *a <= s[i] && s[i] <= *b) != *negated
                }
            };
            if step {
                i += 1;
                j += 1;
                continue;
            }
        }
        match star {
            Some((sj, si)) => {
                j = sj + 1;
                i = si + 1;
                star = Some((sj, si + 1));
            }
            None => return false,
        }
    }
    while matches!(p.get(j), Some(P::Many)) {
        j += 1;
    }
    j == p.len()
}

/// A `LIKE` pattern's parts: `%` any run, `_` one character, `escape` the next character as it is. `None`: the
/// pattern ends with its escape.
fn like_parts(pattern: &str, escape: Option<char>) -> Option<Vec<P>> {
    let mut out = Vec::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        out.push(if Some(c) == escape {
            P::Lit(chars.next()?)
        } else if c == '%' {
            P::Many
        } else if c == '_' {
            P::One
        } else {
            P::Lit(c)
        });
    }
    Some(out)
}

/// SQLite's `LIKE`: ASCII letters match in either case; a pattern ending with its escape matches nothing.
pub(super) fn sqlite_like(s: &str, pattern: &str, escape: Option<char>) -> bool {
    let Some(p) = like_parts(pattern, escape) else {
        return false;
    };
    let s: Vec<char> = s.chars().collect();
    matches(&s, &p, |a, b| {
        a == b || (a.is_ascii() && b.is_ascii() && a.eq_ignore_ascii_case(&b))
    })
}

/// PostgreSQL's `LIKE` (`~~`), case kept: the escape `\` unless another is given (`ESCAPE ''`: none); none for a
/// pattern ending with its escape, which PostgreSQL refuses.
pub(super) fn pg_like(s: &str, pattern: &str, escape: Option<char>) -> Option<bool> {
    let p = like_parts(pattern, escape)?;
    let s: Vec<char> = s.chars().collect();
    Some(matches(&s, &p, |a, b| a == b))
}

/// SQLite's `GLOB`: `*` any run, `?` one character, `[…]` one of a set (`^` first: not of it), case kept; an unclosed
/// set matches nothing.
pub(super) fn sqlite_glob(s: &str, pattern: &str) -> bool {
    let mut parts = Vec::new();
    let p: Vec<char> = pattern.chars().collect();
    let mut k = 0;
    while k < p.len() {
        match p[k] {
            '*' => parts.push(P::Many),
            '?' => parts.push(P::One),
            '[' => {
                let mut i = k + 1;
                let negated = p.get(i) == Some(&'^');
                if negated {
                    i += 1;
                }
                let mut ranges = Vec::new();
                let mut first = true;
                loop {
                    let Some(&c) = p.get(i) else {
                        return false;
                    };
                    if c == ']' && !first {
                        break;
                    }
                    first = false;
                    if p.get(i + 1) == Some(&'-') && p.get(i + 2).is_some_and(|e| *e != ']') {
                        ranges.push((c, p[i + 2]));
                        i += 3;
                    } else {
                        ranges.push((c, c));
                        i += 1;
                    }
                }
                parts.push(P::Set { negated, ranges });
                k = i;
            }
            c => parts.push(P::Lit(c)),
        }
        k += 1;
    }
    let s: Vec<char> = s.chars().collect();
    matches(&s, &parts, |a, b| a == b)
}

/// The escapes of PostgreSQL's regular expressions `regex-lite` reads alike.
const SAFE_ESCAPES: &[u8] = b"dDsSwW.\\-()[]{}*+?|^$/ntr";

/// A regular expression of PostgreSQL's (`~`) compiled where `regex-lite` reads it alike: ASCII only (a class of a
/// locale's letters is not known here), no embedded options, collating elements, back references or escapes of its own
/// (`\m`, `\y`, `\b` as a backspace); `.` matches a new line as PostgreSQL's does.
pub(super) fn compile_regex(pattern: &str, insensitive: bool) -> Option<regex_lite::Regex> {
    if !pattern.is_ascii()
        || pattern.contains("(?")
        || pattern.contains("[[.")
        || pattern.contains("[[=")
    {
        return None;
    }
    if pattern.starts_with("***") {
        return None;
    }
    let b = pattern.as_bytes();
    let mut k = 0;
    while k < b.len() {
        if b[k] == b'\\' {
            if !SAFE_ESCAPES.contains(b.get(k + 1)?) {
                return None;
            }
            k += 2;
        } else {
            k += 1;
        }
    }
    let flags = if insensitive { "(?si)" } else { "(?s)" };
    regex_lite::Regex::new(&format!("{flags}{pattern}")).ok()
}

/// `s` without the characters of `set` at its start (`left`) and its end (`right`).
pub(super) fn trimmed(s: &str, set: &str, left: bool, right: bool) -> String {
    let mut t = s;
    if left {
        t = t.trim_start_matches(|c| set.contains(c));
    }
    if right {
        t = t.trim_end_matches(|c| set.contains(c));
    }
    t.to_owned()
}

/// An exact decimal of a number value.
pub(super) fn exact_of(v: &V) -> Option<Dec> {
    match v {
        V::Int(i) => Some(Dec::int(*i)),
        V::Real(r) => r.exact,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_s_numbers() {
        assert!(matches!(sqlite_number(" 42 "), Some(V::Int(42))));
        assert!(matches!(sqlite_number("-7"), Some(V::Int(-7))));
        assert!(matches!(sqlite_number("1.5"), Some(V::Real(_))));
        assert!(matches!(sqlite_number("3.0e+5"), Some(V::Real(_))));
        assert!(matches!(
            sqlite_number("99999999999999999999"),
            Some(V::Real(_))
        ));
        for not in [
            "",
            "abc",
            "5abc",
            "0x10",
            "1e",
            ".",
            "+",
            "1.2.3",
            "2024-01-15",
        ] {
            assert!(sqlite_number(not).is_none(), "{not}");
        }
    }

    #[test]
    fn like_and_glob() {
        assert!(sqlite_like("Ankara", "an%", None));
        assert!(sqlite_like("ANKARA", "%kar_", None));
        assert!(!sqlite_like("İzmir", "izmir", None));
        assert!(sqlite_like("50%", "50!%", Some('!')));
        assert!(!sqlite_like("500", "50!%", Some('!')));
        assert_eq!(pg_like("Ankara", "an%", Some('\\')), Some(false));
        assert_eq!(pg_like("a_b", "a\\_b", Some('\\')), Some(true));
        assert_eq!(pg_like("axb", "a\\_b", Some('\\')), Some(false));
        assert_eq!(pg_like("ab", "ab\\", Some('\\')), None);
        assert!(sqlite_glob("06", "[0-9][0-9]"));
        assert!(!sqlite_glob("6a", "[0-9][0-9]"));
        assert!(sqlite_glob("A-1", "[A-Z]-*"));
        assert!(sqlite_glob("x", "[^0-9]"));
        assert!(sqlite_glob("]", "[]]"));
        assert!(!sqlite_glob("a", "[a"));
        assert!(!sqlite_glob("abc", "ABC"));
    }

    #[test]
    fn regular_expressions() {
        let m = |p: &str, s: &str| compile_regex(p, false).map(|r| r.is_match(s));
        assert_eq!(m("^[0-9]{2}$", "06"), Some(true));
        assert_eq!(m("^[0-9]{2}$", "6"), Some(false));
        assert_eq!(m("^\\d+$", "123"), Some(true));
        assert_eq!(m("a.b", "a\nb"), Some(true));
        assert_eq!(m("[[:alpha:]]+", "abc"), Some(true));
        assert_eq!(m("\\bx", "x"), None);
        assert_eq!(m("\\mx", "x"), None);
        assert_eq!(m("(?i)x", "X"), None);
        assert_eq!(m("ç", "ç"), None);
        assert_eq!(m("(a", "a"), None);
        assert_eq!(
            compile_regex("^ab$", true).map(|r| r.is_match("AB")),
            Some(true)
        );
    }
}
