//! The tokens of a table constraint's SQL (docs/adr/0755 §3), as SQLite and PostgreSQL spell them: names (quoted or
//! not), strings, numbers and operators. A text this does not know (a dollar-quoted string, an escape string, a
//! parameter) gives none, and the constraint is the table's to judge.

use super::Dialect;

/// A token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Tok {
    /// A name as written (`quoted`: in double quotes, or SQLite's brackets and back quotes; `double`: in double
    /// quotes, which SQLite takes for a string when no column has the name).
    Name {
        text: String,
        quoted: bool,
        double: bool,
    },
    /// A string literal's text.
    Str(String),
    /// A number literal as written.
    Num(String),
    /// SQLite's blob literal (`X'…'`): never judged.
    Blob,
    /// An operator or punctuation.
    Op(&'static str),
}

/// The operators, the longer before the shorter they begin with.
const OPS: [&str; 34] = [
    "!~~*", "!~~", "~~*", "!~*", "::", "||", "<=", ">=", "<>", "!=", "==", "<<", ">>", "~~", "!~",
    "~*", "~", "(", ")", ",", "[", "]", ".", "+", "-", "*", "/", "%", "^", "=", "<", ">", "&", "|",
];

/// The tokens of `sql`; none for a text the dialect does not spell so.
pub(super) fn tokens(sql: &str, dialect: Dialect) -> Option<Vec<Tok>> {
    let b = sql.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'-' && b.get(i + 1) == Some(&b'-') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            let end = sql[i + 2..].find("*/")?;
            i += 2 + end + 2;
            continue;
        }
        if c == b'\'' {
            let (text, next) = quoted(sql, i, '\'')?;
            out.push(Tok::Str(text));
            i = next;
            continue;
        }
        if c == b'"' {
            let (text, next) = quoted(sql, i, '"')?;
            out.push(Tok::Name {
                text,
                quoted: true,
                double: true,
            });
            i = next;
            continue;
        }
        if dialect == Dialect::Sqlite && (c == b'[' || c == b'`') {
            let (text, next) = if c == b'[' {
                let end = sql[i + 1..].find(']')?;
                (sql[i + 1..i + 1 + end].to_owned(), i + end + 2)
            } else {
                quoted(sql, i, '`')?
            };
            out.push(Tok::Name {
                text,
                quoted: true,
                double: false,
            });
            i = next;
            continue;
        }
        if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            let (text, next) = number(sql, i, dialect)?;
            out.push(Tok::Num(text));
            i = next;
            continue;
        }
        if c == b'_' || c.is_ascii_alphabetic() || c >= 0x80 {
            let start = i;
            while i < b.len()
                && (b[i] == b'_' || b[i] == b'$' || b[i].is_ascii_alphanumeric() || b[i] >= 0x80)
            {
                i += 1;
            }
            let word = &sql[start..i];
            // SQLite's blob literal, PostgreSQL's escape, Unicode and bit strings: not judged.
            if b.get(i) == Some(&b'\'') {
                if dialect == Dialect::Sqlite && word.eq_ignore_ascii_case("x") {
                    let (_, next) = quoted(sql, i, '\'')?;
                    out.push(Tok::Blob);
                    i = next;
                    continue;
                }
                if word.len() <= 2
                    && ["e", "u&", "b", "x", "n"]
                        .iter()
                        .any(|p| word.eq_ignore_ascii_case(p))
                {
                    return None;
                }
            }
            out.push(Tok::Name {
                text: word.to_owned(),
                quoted: false,
                double: false,
            });
            continue;
        }
        if c == b'$' || c == b'?' || c == b'@' || (c == b':' && b.get(i + 1) != Some(&b':')) {
            return None;
        }
        let op = OPS.iter().find(|op| sql[i..].starts_with(**op))?;
        out.push(Tok::Op(op));
        i += op.len();
    }
    Some(out)
}

/// A quoted text from `at` (its opening `q`) to its closing one, a doubled quote one quote; the text and the place
/// after it.
fn quoted(sql: &str, at: usize, q: char) -> Option<(String, usize)> {
    let mut text = String::new();
    let mut chars = sql[at + 1..].char_indices().peekable();
    while let Some((k, c)) = chars.next() {
        if c != q {
            text.push(c);
            continue;
        }
        if chars.peek().is_some_and(|(_, n)| *n == q) {
            chars.next();
            text.push(q);
            continue;
        }
        return Some((text, at + 1 + k + 1));
    }
    None
}

/// A number literal from `at`: digits with a point and an exponent, or SQLite's hexadecimal; the text and the place
/// after it. A number running into a name (`12abc`) or PostgreSQL's digit separators give none.
fn number(sql: &str, at: usize, dialect: Dialect) -> Option<(String, usize)> {
    let b = sql.as_bytes();
    let mut i = at;
    if dialect == Dialect::Sqlite && b[i] == b'0' && matches!(b.get(i + 1), Some(b'x' | b'X')) {
        i += 2;
        let start = i;
        while i < b.len() && b[i].is_ascii_hexdigit() {
            i += 1;
        }
        if i == start
            || i - start > 16
            || b.get(i)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            return None;
        }
        return Some((sql[at..i].to_owned(), i));
    }
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if b.get(i) == Some(&b'.') {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let mut k = i + 1;
        if matches!(b.get(k), Some(b'+' | b'-')) {
            k += 1;
        }
        if b.get(k).is_some_and(u8::is_ascii_digit) {
            i = k;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    if b.get(i)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c >= 0x80)
    {
        return None;
    }
    Some((sql[at..i].to_owned(), i))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(t: &str) -> Tok {
        Tok::Name {
            text: t.into(),
            quoted: false,
            double: false,
        }
    }

    #[test]
    fn the_spellings_of_both_dialects() {
        let t = tokens(
            "(tur)::text = ANY (ARRAY['is'::character varying, 'it''s'])",
            Dialect::Postgres,
        )
        .expect("tokens");
        assert_eq!(t[0], Tok::Op("("));
        assert_eq!(t[1], name("tur"));
        assert_eq!(t[3], Tok::Op("::"));
        assert!(t.contains(&Tok::Str("it's".into())));
        let t = tokens(
            "[a b] >= 0x1F AND \"x\"\"y\" != 1.5e-3 -- yorum\n",
            Dialect::Sqlite,
        )
        .expect("tokens");
        assert_eq!(
            t,
            vec![
                Tok::Name {
                    text: "a b".into(),
                    quoted: true,
                    double: false
                },
                Tok::Op(">="),
                Tok::Num("0x1F".into()),
                name("AND"),
                Tok::Name {
                    text: "x\"y".into(),
                    quoted: true,
                    double: true
                },
                Tok::Op("!="),
                Tok::Num("1.5e-3".into()),
            ]
        );
        assert_eq!(
            tokens("kod !~~* 'a%' /* not */ ", Dialect::Postgres),
            Some(vec![name("kod"), Tok::Op("!~~*"), Tok::Str("a%".into())])
        );
        assert_eq!(
            tokens("x'00' = y", Dialect::Sqlite).expect("blob")[0],
            Tok::Blob
        );
    }

    #[test]
    fn what_is_not_spelled_so_gives_none() {
        for sql in [
            "a = E'\\n'",
            "a = $1",
            "a = $$x$$",
            "a = 'open",
            "a = 12abc",
            "a = ?",
            "/* open",
        ] {
            assert_eq!(tokens(sql, Dialect::Postgres), None, "{sql}");
        }
        assert_eq!(tokens("a = 0x", Dialect::Sqlite), None);
    }
}
