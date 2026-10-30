//! A table constraint said in Turkish (docs/adr/0755 §3) in its common shapes: a list of values, a range, a bound, a
//! comparison of two fields, a length, a pattern. Another shape has no sentence; its text is said instead.

use super::parse::{Bin, Cmp, Expr, Match};

/// How a name of the constraint is said: its field's label, else the column's name; none for a name no column has.
pub(super) type Labels<'a> = dyn Fn(&str, bool) -> Option<String> + 'a;

/// The most values of a list a sentence says.
const MOST_SAID: usize = 12;

/// The constraint's sentence, without its full stop; none for a shape this does not say.
pub(super) fn sentence(e: &Expr, label: &Labels<'_>) -> Option<String> {
    match e {
        Expr::And(a, b) => range(a, b, label).or_else(|| {
            Some(format!(
                "{} ve {}",
                sentence(a, label)?,
                sentence(b, label)?
            ))
        }),
        Expr::Or(a, b) => {
            // `x IS NULL OR …`: an empty value holds anyway.
            if is_null_of(a).is_some() {
                sentence(b, label)
            } else if is_null_of(b).is_some() {
                sentence(a, label)
            } else {
                None
            }
        }
        Expr::In { e, not, list } => listed(
            column(e, label)?,
            list.iter().map(constant).collect::<Option<Vec<_>>>()?,
            *not,
        ),
        Expr::Quantified { e, op, all, array } => {
            let not = match (op, all) {
                (Cmp::Eq, false) => false,
                (Cmp::Ne, true) => true,
                _ => return None,
            };
            listed(column(e, label)?, elements(array)?, not)
        }
        Expr::Between {
            e,
            not,
            symmetric: false,
            lo,
            hi,
        } => {
            let (c, lo, hi) = (column(e, label)?, constant(lo)?, constant(hi)?);
            Some(if *not {
                format!("{c} {lo} ile {hi} arasında olmamalı")
            } else {
                format!("{c} {lo} ile {hi} arasında olmalı")
            })
        }
        Expr::IsNull { e, not: true } => Some(format!("{} boş bırakılamaz", column(e, label)?)),
        Expr::Same { a, b, negate: true } if matches!(&**b, Expr::Str(s) if s.is_empty()) => {
            Some(format!("{} boş metin olmamalı", column(a, label)?))
        }
        Expr::Bin(Bin::Cmp(c), a, b) => compared(*c, a, b, label),
        Expr::Like {
            e,
            not,
            how,
            pattern,
            escape: None,
        } if *how != Match::Opaque => {
            let (c, p) = (column(e, label)?, constant(pattern)?);
            Some(if *not {
                format!("{c} şu kalıba uymamalı: {p}")
            } else {
                format!("{c} şu kalıba uymalı: {p}")
            })
        }
        _ => None,
    }
}

/// `x >= lo AND x <= hi` as a range.
fn range(a: &Expr, b: &Expr, label: &Labels<'_>) -> Option<String> {
    let (Expr::Bin(Bin::Cmp(Cmp::Ge), x, lo), Expr::Bin(Bin::Cmp(Cmp::Le), y, hi)) = (a, b) else {
        return None;
    };
    let c = column(x, label)?;
    (column(y, label)? == c).then_some(())?;
    Some(format!(
        "{c} {} ile {} arasında olmalı",
        constant(lo)?,
        constant(hi)?
    ))
}

fn listed(c: String, values: Vec<String>, not: bool) -> Option<String> {
    if values.is_empty() {
        return None;
    }
    let mut said: Vec<String> = values.iter().take(MOST_SAID).cloned().collect();
    if values.len() > MOST_SAID {
        said.push("…".to_owned());
    }
    let said = said.join(", ");
    Some(if not {
        format!("{c} şunlardan biri olmamalı: {said}")
    } else {
        format!("{c} şunlardan biri olmalı: {said}")
    })
}

/// A comparison: a field with a value, two fields, a length with a number.
fn compared(c: Cmp, a: &Expr, b: &Expr, label: &Labels<'_>) -> Option<String> {
    if let Some((x, trim)) = length_of(a, label)
        && let Some(n) = whole(b)
    {
        let blanks = if trim {
            " baştaki ve sondaki boşluklar sayılmadan"
        } else {
            ""
        };
        return Some(match c {
            Cmp::Ge => format!("{x}{blanks} en az {n} karakter olmalı"),
            Cmp::Gt => format!("{x}{blanks} en az {} karakter olmalı", n.checked_add(1)?),
            Cmp::Le => format!("{x}{blanks} en çok {n} karakter olmalı"),
            Cmp::Lt => format!("{x}{blanks} en çok {} karakter olmalı", n.checked_sub(1)?),
            Cmp::Eq => format!("{x}{blanks} {n} karakter olmalı"),
            Cmp::Ne => format!("{x}{blanks} {n} karakter olmamalı"),
        });
    }
    match (column(a, label), column(b, label)) {
        (Some(x), Some(y)) => Some(match c {
            Cmp::Gt => format!("{x}, {y} alanından büyük olmalı"),
            Cmp::Ge => format!("{x}, {y} alanından küçük olmamalı"),
            Cmp::Lt => format!("{x}, {y} alanından küçük olmalı"),
            Cmp::Le => format!("{x}, {y} alanından büyük olmamalı"),
            Cmp::Eq => format!("{x}, {y} alanına eşit olmalı"),
            Cmp::Ne => format!("{x}, {y} alanına eşit olmamalı"),
        }),
        (Some(x), None) => bound(c, &x, b),
        (None, Some(y)) => {
            let flipped = match c {
                Cmp::Lt => Cmp::Gt,
                Cmp::Le => Cmp::Ge,
                Cmp::Gt => Cmp::Lt,
                Cmp::Ge => Cmp::Le,
                same => same,
            };
            bound(flipped, &y, a)
        }
        (None, None) => None,
    }
}

fn bound(c: Cmp, x: &str, value: &Expr) -> Option<String> {
    let v = constant(value)?;
    Some(match c {
        Cmp::Ge => format!("{x} en az {v} olmalı"),
        Cmp::Le => format!("{x} en çok {v} olmalı"),
        Cmp::Gt => format!("{x} {v} değerinden büyük olmalı"),
        Cmp::Lt => format!("{x} {v} değerinden küçük olmalı"),
        Cmp::Eq => format!("{x} {v} olmalı"),
        Cmp::Ne if v == EMPTY => format!("{x} boş metin olmamalı"),
        Cmp::Ne => format!("{x} {v} olmamalı"),
    })
}

/// How an empty text is said.
const EMPTY: &str = "boş metin";

/// A column a side is, through its casts.
fn column(e: &Expr, label: &Labels<'_>) -> Option<String> {
    match e {
        Expr::Name { text, quoted, .. } => label(text, *quoted),
        Expr::Cast(e, _) | Expr::Collate(e, _) => column(e, label),
        _ => None,
    }
}

/// The field of `IS NULL`.
fn is_null_of(e: &Expr) -> Option<&Expr> {
    match e {
        Expr::IsNull { e, not: false } => Some(e),
        _ => None,
    }
}

/// `length(x)` (or `char_length`) of a field, and whether of it trimmed.
fn length_of(e: &Expr, label: &Labels<'_>) -> Option<(String, bool)> {
    let Expr::Call(name, args) = e else {
        return None;
    };
    if !matches!(name.as_str(), "length" | "char_length" | "character_length") {
        return None;
    }
    let [arg] = args.as_slice() else {
        return None;
    };
    if let Some(c) = column(arg, label) {
        return Some((c, false));
    }
    let Expr::Call(trim, inner) = arg else {
        return None;
    };
    let [x] = inner.as_slice() else {
        return None;
    };
    (matches!(trim.as_str(), "trim" | "btrim")).then_some(())?;
    Some((column(x, label)?, true))
}

fn whole(e: &Expr) -> Option<i64> {
    match e {
        Expr::Num(n) => n.parse().ok(),
        Expr::Cast(e, _) => whole(e),
        Expr::Neg(e) => whole(e).and_then(i64::checked_neg),
        _ => None,
    }
}

/// A constant as said: a text as it is (an empty one said so), a number as written.
fn constant(e: &Expr) -> Option<String> {
    match e {
        Expr::Str(s) if s.is_empty() => Some(EMPTY.to_owned()),
        Expr::Str(s) => Some(s.clone()),
        Expr::Num(n) => Some(n.clone()),
        Expr::Bool(b) => Some(if *b { "doğru" } else { "yanlış" }.to_owned()),
        Expr::Neg(e) => Some(format!("-{}", constant(e)?)),
        Expr::Pos(e) | Expr::Cast(e, _) | Expr::Collate(e, _) => constant(e),
        Expr::Name {
            text, double: true, ..
        } => Some(text.clone()),
        _ => None,
    }
}

/// An array's elements as said: `ARRAY[…]`, through its casts, or PostgreSQL's `'{a,b}'`.
fn elements(e: &Expr) -> Option<Vec<String>> {
    match e {
        Expr::Array(items) => items.iter().map(constant).collect(),
        Expr::Cast(e, _) => match &**e {
            Expr::Str(t) => {
                let inner = t.trim().strip_prefix('{')?.strip_suffix('}')?;
                Some(
                    inner
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').to_owned())
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            }
            other => elements(other),
        },
        _ => None,
    }
}
