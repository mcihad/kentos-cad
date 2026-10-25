//! Expressions with functions in the style files (docs/adr/0635 §6): KentOS's expression and QGIS's, word by word.
//! The two languages share their operators and QGIS's keywords (KentOS reads `AND`, `CASE … END`, `IS NULL`, `IN`,
//! `BETWEEN`, `LIKE` as its own); they differ in a field's spelling (`[ad]` and `"ad"`, a string `"…"` in KentOS is
//! `'…'` in QGIS) and in the functions' names. Only the functions of [`FUNCTIONS`], which mean the same in both (ADR
//! 0600 holds them to QGIS's own engine), go across; an expression with any other function, a `$` or `@` value or an
//! operator one side lacks is none, and its writer or reader says it lost.

/// The functions that go across: KentOS's name, QGIS's name.
pub const FUNCTIONS: [(&str, &str); 39] = [
    ("yuvarla", "round"),
    ("metin", "to_string"),
    ("sayı", "to_real"),
    ("tamsayı", "to_int"),
    ("mutlak", "abs"),
    ("min", "min"),
    ("max", "max"),
    ("büyük", "upper"),
    ("küçük", "lower"),
    ("kırp", "trim"),
    ("uzunluk", "length"),
    ("parça", "substr"),
    ("doldur", "lpad"),
    ("sağdoldur", "rpad"),
    ("değiştir", "replace"),
    ("başlar", "starts_with"),
    ("eğer", "if"),
    ("varsayılan", "coalesce"),
    ("kök", "sqrt"),
    ("tavan", "ceil"),
    ("taban", "floor"),
    ("pi", "pi"),
    ("ln", "ln"),
    ("log10", "log10"),
    ("log", "log"),
    ("üstel", "exp"),
    ("sin", "sin"),
    ("cos", "cos"),
    ("tan", "tan"),
    ("asin", "asin"),
    ("acos", "acos"),
    ("atan", "atan"),
    ("atan2", "atan2"),
    ("derece", "degrees"),
    ("radyan", "radians"),
    ("sol", "left"),
    ("sağ", "right"),
    ("bul", "strpos"),
    ("birleştir", "concat"),
];

/// A word with Turkish letters folded and upper-cased (`değil` and `DEGIL` alike).
fn fold(w: &str) -> String {
    w.chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'C',
            'ğ' | 'Ğ' => 'G',
            'ı' | 'I' | 'i' | 'İ' => 'I',
            'ö' | 'Ö' => 'O',
            'ş' | 'Ş' => 'S',
            'ü' | 'Ü' => 'U',
            c => c.to_ascii_uppercase(),
        })
        .collect()
}

/// A function's pair by either side's name.
fn function(w: &str) -> Option<(&'static str, &'static str)> {
    let f = fold(w);
    FUNCTIONS
        .iter()
        .find(|(k, q)| fold(k) == f || fold(q) == f)
        .copied()
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Field(String),
    Str(String),
    Num(String),
    Word(String),
    Op(String),
}

/// The tokens of `src`: KentOS's (`[a]` a field, `'s'` and `"s"` strings) or QGIS's (`"a"` a field, `'s'` a string).
fn tokens(src: &str, qgis: bool) -> Option<Vec<Tok>> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let quoted = |i: usize, q: char| -> Option<(String, usize)> {
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
                return Some((s, j + 1));
            }
            s.push(x);
            j += 1;
        }
    };
    const OPS: [&str; 18] = [
        "<=", ">=", "!=", "<>", "==", "||", "=", "<", ">", "+", "-", "*", "/", "%", "(", ")", ",",
        "^",
    ];
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if !qgis && ch == '[' {
            let j = c[i + 1..].iter().position(|&x| x == ']')? + i + 1;
            let name = c[i + 1..j].iter().collect::<String>().trim().to_owned();
            if name.is_empty() {
                return None;
            }
            out.push(Tok::Field(name));
            i = j + 1;
        } else if ch == '\'' || ch == '"' {
            let (s, j) = quoted(i, ch)?;
            if qgis && ch == '"' {
                if s.is_empty() {
                    return None;
                }
                out.push(Tok::Field(s));
            } else {
                out.push(Tok::Str(s));
            }
            i = j;
        } else if ch.is_ascii_digit()
            || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let mut j = i + 1;
            while j < c.len()
                && (c[j].is_ascii_digit()
                    || c[j] == '.'
                    || ((c[j] == 'e' || c[j] == 'E')
                        && c.get(j + 1)
                            .is_some_and(|x| x.is_ascii_digit() || *x == '-' || *x == '+')))
            {
                if c[j] == 'e' || c[j] == 'E' {
                    j += 1;
                }
                j += 1;
            }
            let text: String = c[i..j].iter().collect();
            text.parse::<f64>().ok()?;
            out.push(Tok::Num(text));
            i = j;
        } else if ch.is_alphabetic() || ch == '_' {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
                j += 1;
            }
            out.push(Tok::Word(c[i..j].iter().collect()));
            i = j;
        } else {
            let rest: String = c[i..c.len().min(i + 2)].iter().collect();
            let op = OPS.iter().find(|o| rest.starts_with(**o))?;
            out.push(Tok::Op((*op).to_owned()));
            i += op.chars().count();
        }
    }
    (!out.is_empty()).then_some(out)
}

/// The keywords both languages read; KentOS's Turkish ones as QGIS's.
fn keyword(w: &str) -> Option<&'static str> {
    Some(match fold(w).as_str() {
        "VE" | "AND" => "AND",
        "VEYA" | "OR" => "OR",
        "DEGIL" | "NOT" => "NOT",
        "DOGRU" | "TRUE" => "TRUE",
        "YANLIS" | "FALSE" => "FALSE",
        "DURUM" | "CASE" => "CASE",
        "EGER" | "WHEN" => "WHEN",
        "ISE" | "THEN" => "THEN",
        "YOKSA" | "ELSE" => "ELSE",
        "SON" | "END" => "END",
        "ICINDE" | "IN" => "IN",
        "ARASINDA" | "BETWEEN" => "BETWEEN",
        "GIBI" | "LIKE" => "LIKE",
        "BENZER" | "ILIKE" => "ILIKE",
        "IS" => "IS",
        "BOS" | "NULL" => "NULL",
        _ => return None,
    })
}

fn quote(s: &str, q: char) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push(q);
    for ch in s.chars() {
        if ch == q {
            out.push(q);
        }
        out.push(ch);
    }
    out.push(q);
    out
}

/// Pieces as text (each with whether it names a function): a space between them, none after an opening parenthesis,
/// before a closing one or a comma, or between a function's name and its parenthesis.
fn join(parts: &[(String, bool)]) -> String {
    let mut s = String::new();
    let mut prev: Option<&(String, bool)> = None;
    for part in parts {
        let p = part.0.as_str();
        let tight = match prev {
            None => true,
            Some((q, function)) => q == "(" || p == ")" || p == "," || (p == "(" && *function),
        };
        if !tight {
            s.push(' ');
        }
        s.push_str(p);
        prev = Some(part);
    }
    s
}

fn is_operand(t: Option<&Tok>) -> bool {
    match t {
        Some(Tok::Field(_) | Tok::Str(_) | Tok::Num(_)) => true,
        Some(Tok::Op(o)) => o == ")",
        Some(Tok::Word(w)) => keyword(w).is_none() || matches!(keyword(w), Some("TRUE" | "FALSE")),
        None => false,
    }
}

/// KentOS's expression as QGIS's, or none when it cannot go across.
pub fn to_qgis(kentos: &str) -> Option<String> {
    let toks = tokens(kentos, false)?;
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let next_open = matches!(toks.get(i + 1), Some(Tok::Op(o)) if o == "(");
        let piece = match &toks[i] {
            Tok::Field(f) => quote(f, '"'),
            Tok::Str(s) => quote(s, '\''),
            Tok::Num(n) => n.clone(),
            Tok::Op(o) if o == "==" => "=".to_owned(),
            Tok::Op(o) => o.clone(),
            // `eğer(…)` is a function (`if`); `WHEN (…)` stays the keyword.
            Tok::Word(w) if next_open && (keyword(w).is_none() || fold(w) == "EGER") => {
                out.push((function(w)?.1.to_owned(), true));
                i += 1;
                continue;
            }
            Tok::Word(w) => match keyword(w) {
                // `[a] boş` is QGIS's `"a" IS NULL`, `[a] boş değil` its `IS NOT NULL`.
                Some("NULL") if is_operand(if i > 0 { toks.get(i - 1) } else { None }) => {
                    if matches!(toks.get(i + 1), Some(Tok::Word(n)) if keyword(n) == Some("NOT")) {
                        i += 1;
                        "IS NOT NULL".to_owned()
                    } else {
                        "IS NULL".to_owned()
                    }
                }
                Some(k) => k.to_owned(),
                None if crate::sld::bare_field(w) => quote(w, '"'),
                None => return None,
            },
        };
        out.push((piece, false));
        i += 1;
    }
    Some(join(&out))
}

/// QGIS's expression as KentOS's, or none when it cannot go across.
pub fn to_kentos(qgis: &str) -> Option<String> {
    let toks = tokens(qgis, true)?;
    let mut out: Vec<(String, bool)> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        let next_open = matches!(toks.get(i + 1), Some(Tok::Op(o)) if o == "(");
        let piece = match t {
            Tok::Field(f) => format!("[{f}]"),
            Tok::Str(s) => quote(s, '\''),
            Tok::Num(n) => n.clone(),
            Tok::Op(o) => o.clone(),
            Tok::Word(w) if next_open && keyword(w).is_none() => {
                out.push((function(w)?.0.to_owned(), true));
                continue;
            }
            Tok::Word(w) => match keyword(w) {
                Some(k) => k.to_owned(),
                None => format!("[{w}]"),
            },
        };
        out.push((piece, false));
    }
    Some(join(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expressions_go_across_both_ways() {
        for (k, q) in [
            ("büyük([ad]) = 'A'", "upper(\"ad\") = 'A'"),
            (
                "yuvarla([alan] / 1000, 1) > 2 AND [tür] IN ('konut', 'ticaret')",
                "round(\"alan\" / 1000, 1) > 2 AND \"tür\" IN ('konut', 'ticaret')",
            ),
            (
                "CASE WHEN [kat] > 3 THEN 'yüksek' ELSE 'alçak' END",
                "CASE WHEN \"kat\" > 3 THEN 'yüksek' ELSE 'alçak' END",
            ),
            ("[ad] IS NULL", "\"ad\" IS NULL"),
            ("[ad] IS NOT NULL", "\"ad\" IS NOT NULL"),
            (
                "varsayılan([ad], '?') || ' (' || metin([no]) || ')'",
                "coalesce(\"ad\", '?') || ' (' || to_string(\"no\") || ')'",
            ),
        ] {
            assert_eq!(to_qgis(k).as_deref(), Some(q), "{k}");
            assert_eq!(to_kentos(q).as_deref(), Some(k), "{q}");
        }
        // KentOS's own words.
        assert_eq!(
            to_qgis("[ad] boş değil ve büyük(ad) gibi 'A%'").as_deref(),
            Some("\"ad\" IS NOT NULL AND upper(\"ad\") LIKE 'A%'")
        );
        assert_eq!(
            to_qgis("\"metin\" == [a]").as_deref(),
            Some("'metin' = \"a\"")
        );
        // What does not go across.
        assert!(to_qgis("$alan > 10").is_none());
        assert!(to_kentos("regexp_match(\"ad\", 'x')").is_none());
        assert!(to_kentos("\"ad\" ~ 'x'").is_none());
        assert!(to_kentos("@map_scale > 10").is_none());
    }
}
