//! The language's rules on one value at a time, as the column engine applies
//! them to each object (docs/adr/0100): values, their text, equality, order
//! and the operators (the functions are `functions`); what `value.rs` states for `Value`,
//! on a copyable view (`V`) whose text is borrowed. Text an operation makes
//! is written at the end of a buffer the caller gives, so evaluating a whole
//! column allocates only when a buffer grows. The rules are those of the
//! TypeScript the language came from (docs/adr/0008 “İfade dili”): attribute
//! values are text, arithmetic reads numbers out of text, an empty value is
//! empty, and no operation looks at another object.

use kentos_geometry_core::jsmath::{js_max, pow};

use crate::compound::{is_compound, is_empty_compound, shown};
use crate::js::collate;
use crate::js::text::{self, MAX_STRING_UNITS, utf16_len};
use crate::parser::BinOp;
use crate::{patterns, read};

/// A value as an operation reads it: text borrowed from the object, the
/// source or the batch's buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum V<'a> {
    Null,
    Num(f64),
    Text(&'a str),
    Bool(bool),
}

impl<'a> V<'a> {
    /// The value as it shows: an array's or a map's JSON as plain text (any other value as it is).
    pub fn shown(self) -> V<'a> {
        match self {
            V::Text(s) => V::Text(shown(s)),
            v => v,
        }
    }
}

/// An operation's result: a value (text in it borrowed from an argument),
/// the text it wrote at the end of the buffer since it began (`Made`), or
/// what JavaScript would have thrown at (text past V8's longest string),
/// which empties the whole expression.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum R<'a> {
    V(V<'a>),
    Made,
    Thrown,
}

/// Buffers an operation may write text into while it reads its arguments,
/// and the regular expressions read last (docs/adr/0214 §2.4).
#[derive(Debug, Default)]
pub struct Scratch {
    pub a: String,
    pub b: String,
    pub c: String,
    pub patterns: patterns::Kept,
}

impl Scratch {
    /// Empties the buffers, keeping their room.
    pub fn clear(&mut self) {
        self.a.clear();
        self.b.clear();
        self.c.clear();
    }
}

/// A number, or None when the value is empty or not a number. A number
/// written in text passes as it is (even "1e400", which is infinite); a
/// number value that is not finite is no number.
pub fn to_number(v: V) -> Option<f64> {
    match v {
        V::Num(x) => x.is_finite().then_some(x),
        V::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        V::Text(s) => read::number(text::trim(s)),
        V::Null => None,
    }
}

pub fn is_empty(v: V) -> bool {
    matches!(v, V::Null | V::Text(""))
}

/// Whether a value counts as true: an empty text, array or map does not
/// (docs/adr/0214 §2.1).
pub fn truthy(v: V) -> bool {
    match v {
        V::Null => false,
        V::Bool(b) => b,
        // NaN is not 0.
        V::Num(x) => x != 0.0,
        V::Text(s) => !s.is_empty() && !is_empty_compound(s),
    }
}

/// The value's text at the end of `out`: "" for empty, doğru/yanlış, an
/// array's or a map's JSON.
pub fn push_text(out: &mut String, v: V) {
    match v {
        V::Null => {}
        V::Bool(b) => out.push_str(if b { "doğru" } else { "yanlış" }),
        V::Num(x) => read::push_number_text(out, x),
        V::Text(s) => out.push_str(shown(s)),
    }
}

/// The value's text: a text value's own (an array's or a map's JSON),
/// anything else written into `buf`.
pub fn as_text<'x>(v: V<'x>, buf: &'x mut String) -> &'x str {
    match v {
        V::Text(s) => shown(s),
        V::Bool(b) => {
            if b {
                "doğru"
            } else {
                "yanlış"
            }
        }
        V::Null => "",
        V::Num(x) => {
            buf.clear();
            read::push_number_text(buf, x);
            buf
        }
    }
}

/// Equality: numbers by value (within 1e-9, relative above 1), text exactly,
/// and "empty" equals only "empty".
pub fn equals(a: V, b: V, s: &mut Scratch) -> bool {
    if is_empty(a) || is_empty(b) {
        return is_empty(a) && is_empty(b);
    }
    if matches!(a, V::Bool(_)) || matches!(b, V::Bool(_)) {
        return truthy(a) == truthy(b);
    }
    // An array or a map equals only the same array or map (its text with its
    // mark: never a text that shows the same JSON; docs/adr/0214 §2.1).
    match (a, b) {
        (V::Text(x), V::Text(y)) if is_compound(x) || is_compound(y) => return x == y,
        (V::Text(x), _) | (_, V::Text(x)) if is_compound(x) => return false,
        _ => {}
    }
    if let (Some(na), Some(nb)) = (to_number(a), to_number(b)) {
        return (na - nb).abs() <= 1e-9 * js_max(js_max(1.0, na.abs()), nb.abs());
    }
    let Scratch { a: sa, b: sb, .. } = s;
    as_text(a, sa) == as_text(b, sb)
}

/// Order of two values: numbers numerically (their difference, NaN for ∞ − ∞),
/// text in Turkish order; None when one is empty.
pub fn compare(a: V, b: V, s: &mut Scratch) -> Option<f64> {
    if is_empty(a) || is_empty(b) {
        return None;
    }
    if let (Some(na), Some(nb)) = (to_number(a), to_number(b)) {
        return Some(na - nb);
    }
    let Scratch { a: sa, b: sb, .. } = s;
    Some(match collate::compare_tr(as_text(a, sa), as_text(b, sb)) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    })
}

/// Below this many bytes a join is shorter than V8's longest string whatever it holds.
pub const MAX_JOIN_BYTES: usize = MAX_STRING_UNITS;

/// A bound on the length in bytes of a value's text: its own for text, and
/// more than any number, true/false or empty writes.
fn text_len_bound(v: V) -> usize {
    match v {
        V::Text(s) => s.len(),
        _ => 32,
    }
}

/// Joined text, where JavaScript throws past its longest string.
fn join<'a>(a: V<'a>, b: V<'a>, out: &mut String, s: &mut Scratch) -> R<'a> {
    if text_len_bound(a) + text_len_bound(b) > MAX_STRING_UNITS {
        let Scratch { a: sa, b: sb, .. } = s;
        let (ta, tb) = (as_text(a, sa), as_text(b, sb));
        if ta.len() + tb.len() > MAX_STRING_UNITS
            && utf16_len(ta) + utf16_len(tb) > MAX_STRING_UNITS
        {
            return R::Thrown;
        }
    }
    push_text(out, a);
    push_text(out, b);
    R::Made
}

pub fn not(a: V) -> V<'static> {
    V::Bool(!truthy(a))
}

pub fn neg(a: V) -> V<'static> {
    to_number(a).map_or(V::Null, |x| V::Num(-x))
}

pub fn binary<'a>(op: BinOp, a: V<'a>, b: V<'a>, out: &mut String, s: &mut Scratch) -> R<'a> {
    R::V(match op {
        BinOp::Or => V::Bool(truthy(a) || truthy(b)),
        BinOp::And => V::Bool(truthy(a) && truthy(b)),
        BinOp::Eq => V::Bool(equals(a, b, s)),
        BinOp::Ne => V::Bool(!equals(a, b, s)),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match compare(a, b, s) {
            None => V::Bool(false),
            Some(c) => V::Bool(match op {
                BinOp::Lt => c < 0.0,
                BinOp::Le => c <= 0.0,
                BinOp::Gt => c > 0.0,
                _ => c >= 0.0,
            }),
        },
        BinOp::Join => return join(a, b, out, s),
        BinOp::Add => {
            if a == V::Null || b == V::Null {
                return R::V(V::Null);
            }
            match (to_number(a), to_number(b)) {
                (Some(na), Some(nb)) => V::Num(na + nb),
                _ => return join(a, b, out, s),
            }
        }
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::Pow => {
            let (Some(na), Some(nb)) = (to_number(a), to_number(b)) else {
                return R::V(V::Null);
            };
            if matches!(op, BinOp::Div | BinOp::Rem) && nb == 0.0 {
                return R::V(V::Null);
            }
            V::Num(match op {
                BinOp::Sub => na - nb,
                BinOp::Mul => na * nb,
                BinOp::Div => na / nb,
                BinOp::Pow => pow(na, nb),
                _ => na % nb,
            })
        }
    })
}

// ── Since docs/adr/0100 §4: CASE, IN, BETWEEN, LIKE ─────────────────

/// `durum eğer c ise v … [yoksa e] son`: the value of the first condition
/// that holds; `parts` alternate conditions and values, the else last when
/// `otherwise`. None is an operand that threw. What is not reached does not
/// count: a condition after the one that holds, a value not chosen.
pub fn case<'a>(parts: &[Option<V<'a>>], otherwise: bool) -> R<'a> {
    let (pairs, rest) = parts.split_at(parts.len() - usize::from(otherwise));
    for pair in pairs.chunks_exact(2) {
        match pair[0] {
            None => return R::Thrown,
            Some(c) if truthy(c) => return pair[1].map_or(R::Thrown, R::V),
            Some(_) => {}
        }
    }
    match rest.first() {
        Some(Some(v)) => R::V(*v),
        Some(None) => R::Thrown,
        None => R::V(V::Null),
    }
}

/// `x içinde (a, b, …)`: whether x equals one of them, as `=` compares.
pub fn within(x: V, items: &[V], s: &mut Scratch) -> bool {
    items.iter().any(|&i| equals(x, i, s))
}

/// `x arasında a ve b`: a ≤ x ≤ b, in the order `<` uses; false with an empty value.
pub fn between(x: V, low: V, high: V, s: &mut Scratch) -> bool {
    matches!(compare(x, low, s), Some(c) if c >= 0.0)
        && matches!(compare(x, high, s), Some(c) if c <= 0.0)
}

/// A pattern's next part at byte `p`: `%`, `_`, or one character (`\`
/// takes the next character as it is), and where the part ends.
fn part(pattern: &str, p: usize) -> Option<(Part, usize)> {
    let mut chars = pattern.get(p..)?.chars();
    let c = chars.next()?;
    Some(match c {
        '%' => (Part::Any, p + 1),
        '_' => (Part::One, p + 1),
        '\\' => match chars.next() {
            Some(e) => (Part::Char(e), p + 1 + e.len_utf8()),
            None => (Part::Char('\\'), p + 1),
        },
        c => (Part::Char(c), p + c.len_utf8()),
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Any,
    One,
    Char(char),
}

/// Whether `text` fits `pattern` (`%` any text, `_` one character): the
/// greedy match with one step back to the last `%`, over byte positions.
fn fits(text: &str, pattern: &str) -> bool {
    let (mut t, mut p) = (0, 0);
    // The last `%` seen: the pattern after it, and where the text stood.
    let mut back: Option<(usize, usize)> = None;
    loop {
        let next = text.get(t..).and_then(|r| r.chars().next());
        match (part(pattern, p), next) {
            (Some((Part::Any, after)), _) => {
                back = Some((after, t));
                p = after;
            }
            (Some((Part::One, after)), Some(c)) => {
                t += c.len_utf8();
                p = after;
            }
            (Some((Part::Char(e), after)), Some(c)) if e == c => {
                t += c.len_utf8();
                p = after;
            }
            (None, None) => return true,
            _ => {
                // A mismatch (or the pattern ended first): the last `%` takes one character more.
                let Some((after, from)) = back else {
                    return false;
                };
                let Some(c) = text.get(from..).and_then(|r| r.chars().next()) else {
                    return false;
                };
                back = Some((after, from + c.len_utf8()));
                t = from + c.len_utf8();
                p = after;
            }
        }
    }
}

/// `x gibi p` (LIKE): `%` any text, `_` one character, `\` the next
/// character as it is; `fold` (`benzer`, ILIKE) ignores case and the Turkish
/// letters' marks as `içerir` does, but keeps the spaces at either end.
/// False with an empty value or pattern.
pub fn like(x: V, p: V, fold: bool, s: &mut Scratch) -> bool {
    if is_empty(x) || p == V::Null {
        return false;
    }
    let Scratch { a, b, c, .. } = s;
    if fold {
        let (mut fx, mut fp) = (std::mem::take(a), std::mem::take(b));
        fx.clear();
        text::push_fold(&mut fx, as_text(x, c));
        fp.clear();
        text::push_fold(&mut fp, as_text(p, c));
        let fit = fits(&fx, &fp);
        (*a, *b) = (fx, fp);
        return fit;
    }
    fits(as_text(x, a), as_text(p, b))
}

#[cfg(test)]
mod language {
    use super::*;

    #[test]
    fn like_takes_percent_underscore_and_escapes() {
        let s = &mut Scratch::default();
        let t = |x: &str, p: &str| like(V::Text(x), V::Text(p), false, &mut Scratch::default());
        assert!(t("1245", "12%") && t("1245", "%45") && t("1245", "%2%") && t("1245", "1_4_"));
        assert!(!t("1245", "12") && !t("1245", "_2%5_") && !t("", "%"));
        assert!(t("a%b", "a\\%b") && !t("axb", "a\\%b") && t("a_", "a\\_"));
        assert!(t("aXbXc", "a%b%c") && t("abcabc", "%abc") && !t("abcab", "%abc"));
        assert!(t("Çınar", "Ç_nar") && t("😀x", "_x"));
        assert!(like(V::Text("Çınar"), V::Text("CIN%"), true, s));
        assert!(!like(V::Text("Çınar"), V::Text("CIN%"), false, s));
        assert!(like(V::Num(12.5), V::Text("12._"), false, s));
        // benzer keeps the spaces: ' a' does not end in a space.
        assert!(
            !like(V::Text("a"), V::Text("% "), true, s)
                && like(V::Text("a "), V::Text("A "), true, s)
        );
    }

    #[test]
    fn case_in_and_between_read_what_they_must() {
        let s = &mut Scratch::default();
        let n = |x| Some(V::Num(x));
        assert_eq!(
            case(
                &[Some(V::Bool(false)), n(1.0), Some(V::Bool(true)), n(2.0)],
                false
            ),
            R::V(V::Num(2.0))
        );
        // A value not chosen, or a condition not reached, may have thrown.
        assert_eq!(
            case(&[Some(V::Bool(true)), n(1.0), None, None], false),
            R::V(V::Num(1.0))
        );
        assert_eq!(
            case(&[None, n(1.0), Some(V::Bool(true)), n(2.0)], false),
            R::Thrown
        );
        assert_eq!(
            case(&[Some(V::Bool(false)), n(1.0), n(9.0)], true),
            R::V(V::Num(9.0))
        );
        assert_eq!(case(&[Some(V::Null), n(1.0)], false), R::V(V::Null));
        assert!(within(V::Text("12.0"), &[V::Num(3.0), V::Num(12.0)], s));
        assert!(!within(V::Null, &[V::Num(1.0)], s) && within(V::Null, &[V::Text("")], s));
        assert!(between(V::Num(5.0), V::Num(5.0), V::Num(9.0), s));
        assert!(!between(V::Null, V::Num(1.0), V::Num(9.0), s));
        assert!(between(V::Text("Çınar"), V::Text("Çam"), V::Text("Dut"), s));
        assert!(
            !between(V::Text("Ceviz"), V::Text("Çam"), V::Text("Dut"), s),
            "C comes before Ç"
        );
        let mut out = String::new();
        assert_eq!(
            binary(BinOp::Pow, V::Num(2.0), V::Text("10"), &mut out, s),
            R::V(V::Num(1024.0))
        );
        assert_eq!(
            binary(BinOp::Pow, V::Num(2.0), V::Null, &mut out, s),
            R::V(V::Null)
        );
    }
}
