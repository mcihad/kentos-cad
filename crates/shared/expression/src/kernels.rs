//! The column engine's instructions over a batch (docs/adr/0100): operators
//! and number functions first run over the whole batch as if every value
//! were a finite number, then the language's rules (`scalar`, `functions`)
//! take the objects that were not. A fast pass settles only objects where
//! the rules would give the same.

use kentos_geometry_core::jsmath::js_max;

use crate::exec::{BOOL, Col, NULL, NUM, Out, TEXT, THROWN};
use crate::functions;
use crate::library::Func;
use crate::parser::BinOp;
use crate::program::{Const, Operand, Program};
use crate::scalar::{self, R, Scratch, V};

pub(crate) fn bit(b: bool) -> f64 {
    f64::from(u8::from(b))
}

/// The constant an operand is, if it is one.
pub(crate) fn constant(p: &Program, o: Operand) -> Option<&Const> {
    match o {
        Operand::Const(c) => Some(p.constant(c)),
        Operand::Reg(_) => None,
    }
}

/// A constant text that is neither empty nor a number (`'Arsa'`): against
/// it, `=` on text comes down to comparing the two texts.
pub(crate) fn text_constant(c: Option<&Const>) -> Option<&str> {
    match c {
        Some(Const::Text(t)) if !t.is_empty() && scalar::to_number(V::Text(t)).is_none() => Some(t),
        _ => None,
    }
}

/// `=` or `!=` of `other` against such a constant text, over the batch:
/// each text value compared directly; whether some object was not text.
fn texts_equal(other: Col, c: &str, same: bool, out: &mut Out) -> bool {
    let mut rest = false;
    for i in 0..out.k.len() {
        if other.k[i] != TEXT {
            rest = true;
            continue;
        }
        out.k[i] = BOOL;
        out.x[i] = bit((other.text(i) == c) == same);
    }
    rest
}

/// Numbers within 1e-9 of each other (relative above 1): the language's equality.
#[inline]
fn close(x: f64, y: f64) -> bool {
    (x - y).abs() <= 1e-9 * js_max(js_max(1.0, x.abs()), y.abs())
}

/// `f` over the batch as if both sides held finite numbers; whether some object did not.
#[inline(always)]
fn numbers2(a: Col, b: Col, out: &mut Out, f: impl Fn(f64, f64) -> (u8, f64)) -> bool {
    let n = out.k.len();
    let (ka, xa, kb, xb) = (&a.k[..n], &a.x[..n], &b.k[..n], &b.x[..n]);
    let (ko, xo) = (&mut out.k[..n], &mut out.x[..n]);
    let mut other = false;
    for i in 0..n {
        let (x, y) = (xa[i], xb[i]);
        other |= !((ka[i] == NUM) & (kb[i] == NUM) & x.is_finite() & y.is_finite());
        let (k, v) = f(x, y);
        ko[i] = k;
        xo[i] = v;
    }
    other
}

/// `f` over the batch as if both sides held numbers or true/false; whether some object did not.
#[inline(always)]
fn truths2(a: Col, b: Col, out: &mut Out, f: impl Fn(bool, bool) -> bool) -> bool {
    let n = out.k.len();
    let (ka, xa, kb, xb) = (&a.k[..n], &a.x[..n], &b.k[..n], &b.x[..n]);
    let (ko, xo) = (&mut out.k[..n], &mut out.x[..n]);
    let plain = |k: u8| (k == NUM) | (k == BOOL);
    let mut other = false;
    for i in 0..n {
        other |= !(plain(ka[i]) & plain(kb[i]));
        ko[i] = BOOL;
        xo[i] = bit(f(xa[i] != 0.0, xb[i] != 0.0));
    }
    other
}

/// Which objects a fast pass settled; the rules take the others.
#[derive(Clone, Copy, PartialEq)]
enum Fast {
    Numbers,
    Truths,
    Texts,
    None,
}

pub(crate) fn binary(
    op: BinOp,
    a: Col,
    b: Col,
    text: Option<(&str, bool)>,
    out: &mut Out,
    scratch: &mut Scratch,
) {
    let fast = match (op, text) {
        (BinOp::And | BinOp::Or, _) => Fast::Truths,
        (BinOp::Join, _) => Fast::None,
        (BinOp::Eq | BinOp::Ne, Some(_)) => Fast::Texts,
        _ => Fast::Numbers,
    };
    // The side that is not the text constant.
    let other = match text {
        Some((_, true)) => b,
        _ => a,
    };
    let rest = match (op, text) {
        (BinOp::Eq | BinOp::Ne, Some((c, _))) => texts_equal(other, c, op == BinOp::Eq, out),
        (BinOp::Add, _) => numbers2(a, b, out, |x, y| (NUM, x + y)),
        (BinOp::Sub, _) => numbers2(a, b, out, |x, y| (NUM, x - y)),
        (BinOp::Mul, _) => numbers2(a, b, out, |x, y| (NUM, x * y)),
        (BinOp::Div, _) => numbers2(
            a,
            b,
            out,
            |x, y| if y == 0.0 { (NULL, 0.0) } else { (NUM, x / y) },
        ),
        (BinOp::Rem, _) => numbers2(
            a,
            b,
            out,
            |x, y| if y == 0.0 { (NULL, 0.0) } else { (NUM, x % y) },
        ),
        (BinOp::Eq, _) => numbers2(a, b, out, |x, y| (BOOL, bit(close(x, y)))),
        (BinOp::Ne, _) => numbers2(a, b, out, |x, y| (BOOL, bit(!close(x, y)))),
        (BinOp::Lt, _) => numbers2(a, b, out, |x, y| (BOOL, bit(x - y < 0.0))),
        (BinOp::Le, _) => numbers2(a, b, out, |x, y| (BOOL, bit(x - y <= 0.0))),
        (BinOp::Gt, _) => numbers2(a, b, out, |x, y| (BOOL, bit(x - y > 0.0))),
        (BinOp::Ge, _) => numbers2(a, b, out, |x, y| (BOOL, bit(x - y >= 0.0))),
        (BinOp::And, _) => truths2(a, b, out, |p, q| p && q),
        (BinOp::Or, _) => truths2(a, b, out, |p, q| p || q),
        (BinOp::Join, _) => true,
    };
    if !rest {
        return;
    }
    for i in 0..out.k.len() {
        let settled = match fast {
            Fast::Numbers => a.finite(i) && b.finite(i),
            Fast::Truths => matches!(a.k[i], NUM | BOOL) && matches!(b.k[i], NUM | BOOL),
            Fast::Texts => other.k[i] == TEXT,
            Fast::None => false,
        };
        if settled {
            continue;
        }
        if fast == Fast::Truths {
            match (a.truth(i), b.truth(i)) {
                (Some(p), Some(q)) => {
                    out.k[i] = BOOL;
                    out.x[i] = bit(if op == BinOp::And { p && q } else { p || q });
                }
                _ => out.k[i] = THROWN,
            }
            continue;
        }
        let (Some(x), Some(y)) = (a.view(i), b.view(i)) else {
            out.k[i] = THROWN;
            continue;
        };
        if fast == Fast::Numbers && numbers_of_text(op, x, y, out, i) {
            continue;
        }
        out.rule(i, |made| scalar::binary(op, x, y, made, scratch));
    }
}

/// Arithmetic and order on numbers read out of text (attributes are text):
/// where both sides read as numbers, the rules come down to the numbers, so
/// they are used directly. False where the rules must decide (empty values,
/// text that is no number, `=`, `+` that joins).
#[inline]
fn numbers_of_text(op: BinOp, x: V, y: V, out: &mut Out, i: usize) -> bool {
    if matches!(op, BinOp::Eq | BinOp::Ne) || scalar::is_empty(x) || scalar::is_empty(y) {
        return false;
    }
    let (Some(p), Some(q)) = (scalar::to_number(x), scalar::to_number(y)) else {
        return false;
    };
    let (k, v) = match op {
        BinOp::Add => (NUM, p + q),
        BinOp::Sub => (NUM, p - q),
        BinOp::Mul => (NUM, p * q),
        BinOp::Div | BinOp::Rem if q == 0.0 => (NULL, 0.0),
        BinOp::Div => (NUM, p / q),
        BinOp::Rem => (NUM, p % q),
        BinOp::Lt => (BOOL, bit(p - q < 0.0)),
        BinOp::Le => (BOOL, bit(p - q <= 0.0)),
        BinOp::Gt => (BOOL, bit(p - q > 0.0)),
        BinOp::Ge => (BOOL, bit(p - q >= 0.0)),
        _ => return false,
    };
    out.k[i] = k;
    out.x[i] = v;
    true
}

pub(crate) fn not(a: Col, out: &mut Out) {
    for i in 0..out.k.len() {
        match a.truth(i) {
            Some(t) => {
                out.k[i] = BOOL;
                out.x[i] = bit(!t);
            }
            None => out.k[i] = THROWN,
        }
    }
}

pub(crate) fn neg(a: Col, out: &mut Out) {
    for i in 0..out.k.len() {
        if a.finite(i) {
            out.k[i] = NUM;
            out.x[i] = -a.x[i];
            continue;
        }
        match a.view(i) {
            Some(v) => out.rule(i, |_| R::V(scalar::neg(v))),
            None => out.k[i] = THROWN,
        }
    }
}

/// A number function of finite numbers, directly (the rules give the same):
/// None where the function is not one or an argument is not a finite number.
#[inline]
fn number_function(f: Func, cols: &[Col], i: usize) -> Option<V<'static>> {
    const FEW: usize = 8;
    if cols.len() > FEW || !cols.iter().all(|c| c.finite(i)) {
        return None;
    }
    let mut xs = [0.0; FEW];
    for (x, c) in xs.iter_mut().zip(cols) {
        *x = c.x[i];
    }
    functions::number_function(f, &xs[..cols.len()])
}

/// A function over the batch: number functions of numbers directly, the
/// rest by the language's rules (`scalar::call`).
pub(crate) fn call(f: Func, cols: &[Col], out: &mut Out, scratch: &mut Scratch) {
    const FEW: usize = 8;
    let mut few = [V::Null; FEW];
    let mut many = Vec::new();
    let numeric = matches!(
        f,
        Func::Round | Func::Number | Func::Int | Func::Abs | Func::Min | Func::Max
    );
    for i in 0..out.k.len() {
        if numeric && let Some(v) = number_function(f, cols, i) {
            match v {
                V::Num(x) => {
                    out.k[i] = NUM;
                    out.x[i] = x;
                }
                _ => out.k[i] = NULL,
            }
            continue;
        }
        let values: &mut [V] = if cols.len() <= FEW {
            &mut few[..cols.len()]
        } else {
            many.resize(cols.len(), V::Null);
            &mut many
        };
        let mut thrown = false;
        for (v, c) in values.iter_mut().zip(cols) {
            match c.view(i) {
                Some(x) => *v = x,
                None => thrown = true,
            }
        }
        if thrown {
            out.k[i] = THROWN;
            continue;
        }
        let values = &*values;
        out.rule(i, |made| functions::call(f, values, made, scratch));
    }
}

/// `yuvarla(x, d)` with a constant number of digits: the power of ten once,
/// then each finite number directly; the rest by the rules.
pub(crate) fn round_to(a: Col, d: f64, out: &mut Out, scratch: &mut Scratch) {
    let k = functions::scale(d);
    for i in 0..out.k.len() {
        if a.finite(i) {
            out.k[i] = NUM;
            out.x[i] = functions::round(a.x[i], k);
            continue;
        }
        let Some(x) = a.view(i) else {
            out.k[i] = THROWN;
            continue;
        };
        out.rule(i, |made| {
            functions::call(Func::Round, &[x, V::Num(d)], made, scratch)
        });
    }
}
