//! The language's rules on one value at a time, as the column engine applies
//! them to each object (docs/adr/0100): values, their text, equality, order
//! and the operators (the functions are `functions`); what `value.rs` states for `Value`,
//! on a copyable view (`V`) whose text is borrowed. Text an operation makes
//! is written at the end of a buffer the caller gives, so evaluating a whole
//! column allocates only when a buffer grows. The rules are those of the
//! TypeScript the language came from (docs/adr/0008 “İfade dili”): attribute
//! values are text, arithmetic reads numbers out of text, an empty value is
//! empty, and no operation looks at another object.

use kentos_geometry_core::jsmath::js_max;

use crate::js::collate;
use crate::js::text::{self, MAX_STRING_UNITS, utf16_len};
use crate::parser::BinOp;
use crate::read;

/// A value as an operation reads it: text borrowed from the object, the
/// source or the batch's buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum V<'a> {
    Null,
    Num(f64),
    Text(&'a str),
    Bool(bool),
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

/// Buffers an operation may write text into while it reads its arguments.
#[derive(Debug, Default)]
pub struct Scratch {
    pub a: String,
    pub b: String,
    pub c: String,
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

pub fn truthy(v: V) -> bool {
    match v {
        V::Null => false,
        V::Bool(b) => b,
        // NaN is not 0.
        V::Num(x) => x != 0.0,
        V::Text(s) => !s.is_empty(),
    }
}

/// The value's text at the end of `out`: "" for empty, doğru/yanlış.
pub fn push_text(out: &mut String, v: V) {
    match v {
        V::Null => {}
        V::Bool(b) => out.push_str(if b { "doğru" } else { "yanlış" }),
        V::Num(x) => read::push_number_text(out, x),
        V::Text(s) => out.push_str(s),
    }
}

/// The value's text: a text value's own, anything else written into `buf`.
pub fn as_text<'x>(v: V<'x>, buf: &'x mut String) -> &'x str {
    match v {
        V::Text(s) => s,
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
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
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
                _ => na % nb,
            })
        }
    })
}
