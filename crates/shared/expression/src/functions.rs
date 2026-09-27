//! The language's functions on one object's values (docs/adr/0100): what
//! `yuvarla`, `metin`, `doldur` … compute, by the rules of `scalar`. The
//! names, arities and help are `library`'s table. Text a function makes is
//! written at the end of the caller's buffer.

use kentos_geometry_core::jsmath::{js_max, js_max_all, js_min, js_min_all, js_round, js_sign};

use crate::js::number;
use crate::js::text::{self, MAX_STRING_UNITS, utf16_len};
use crate::library::Func;
use crate::read::push_number_text;
use crate::scalar::{R, Scratch, V, as_text, is_empty, push_text, to_number, truthy};

const POW10: [f64; 13] = [
    1.0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12,
];

/// `Math.max(0, Math.min(12, Math.round(d)))` as an index (d from `to_number`: never NaN).
fn digits(d: f64) -> usize {
    js_max(0.0, js_min(12.0, js_round(d))) as usize
}

/// The power of ten `yuvarla` scales by for `d` digits (0 to 12).
pub fn scale(d: f64) -> f64 {
    POW10[digits(d)]
}

/// `yuvarla`: `Math.round` at a power of ten (the epsilon nudges 1.005 to
/// 1.01, as the TypeScript did).
#[inline]
pub fn round(v: f64, k: f64) -> f64 {
    js_round((v + js_sign(v) * f64::EPSILON * v.abs()) * k) / k
}

/// A number function of finite numbers (`xs`, as many as it takes), directly:
/// the value the rules give. None where `f` is not one of them.
#[inline]
pub fn number_function(f: Func, xs: &[f64]) -> Option<V<'static>> {
    let finite = |r: f64| if r.is_finite() { V::Num(r) } else { V::Null };
    let first = *xs.first()?;
    Some(match f {
        Func::Round => V::Num(round(first, scale(xs.get(1).copied().unwrap_or(0.0)))),
        Func::Number => V::Num(first),
        Func::Int => finite(first.trunc()),
        Func::Abs => finite(first.abs()),
        Func::Min => finite(js_min_all(xs.iter().copied())),
        Func::Max => finite(js_max_all(xs.iter().copied())),
        _ => return None,
    })
}

/// A position or length for `slice` and `padStart`: ∞ and anything past the text clamp.
fn units(x: f64, cap: usize) -> usize {
    if x >= cap as f64 { cap } else { x as usize }
}

/// Numeric function: empty in, empty out; a non-finite result is empty.
fn numeric_of(args: &[V], f: impl Fn(&mut dyn Iterator<Item = f64>) -> f64) -> V<'static> {
    if args.iter().any(|&a| to_number(a).is_none()) {
        return V::Null;
    }
    let r = f(&mut args.iter().filter_map(|&a| to_number(a)));
    if r.is_finite() { V::Num(r) } else { V::Null }
}

/// The value's text folded (`fold_turkish`) into `dst`; `tmp` holds a number's text.
fn fold_into(dst: &mut String, v: V, tmp: &mut String) {
    dst.clear();
    text::push_fold_turkish(dst, as_text(v, tmp));
}

/// Calls a function on its arguments (`args.len()` is within its arity).
/// A result that is one of the arguments, or a part of one, borrows it.
pub fn call<'a>(f: Func, args: &[V<'a>], out: &mut String, s: &mut Scratch) -> R<'a> {
    let arg = |i: usize| args.get(i).copied().unwrap_or(V::Null);
    let num = |i: usize| args.get(i).and_then(|&a| to_number(a));
    let first = arg(0);
    R::V(match f {
        Func::Round => {
            let d = if args.len() > 1 { num(1) } else { Some(0.0) };
            let (Some(v), Some(d)) = (num(0), d) else {
                return R::V(V::Null);
            };
            V::Num(round(v, scale(d)))
        }
        Func::Text => {
            if args.len() < 2 {
                return match first {
                    V::Num(x) => {
                        push_number_text(out, x);
                        R::Made
                    }
                    V::Text(t) => R::V(V::Text(t)),
                    V::Bool(b) => R::V(V::Text(if b { "doğru" } else { "yanlış" })),
                    V::Null => R::V(V::Text("")),
                };
            }
            let (Some(v), Some(d)) = (num(0), num(1)) else {
                return R::V(V::Null);
            };
            number::push_fixed(out, v, digits(d) as u32);
            return R::Made;
        }
        Func::Number => num(0).map_or(V::Null, V::Num),
        Func::Int => numeric_of(args, |n| n.next().unwrap_or(f64::NAN).trunc()),
        Func::Abs => numeric_of(args, |n| n.next().unwrap_or(f64::NAN).abs()),
        Func::Min => numeric_of(args, |n| js_min_all(n)),
        Func::Max => numeric_of(args, |n| js_max_all(n)),
        Func::Upper | Func::Lower | Func::Trim | Func::Length | Func::Replace
            if first == V::Null =>
        {
            V::Null
        }
        Func::Upper => {
            text::push_upper_tr(out, as_text(first, &mut s.a));
            return R::Made;
        }
        Func::Lower => {
            text::push_lower_tr(out, as_text(first, &mut s.a));
            return R::Made;
        }
        Func::Trim => match first {
            V::Text(t) => V::Text(text::trim(t)),
            v => {
                out.push_str(text::trim(as_text(v, &mut s.a)));
                return R::Made;
            }
        },
        Func::Length => V::Num(utf16_len(as_text(first, &mut s.a)) as f64),
        Func::Substr => {
            let len = if args.len() > 2 {
                num(2)
            } else {
                Some(f64::INFINITY)
            };
            let (Some(from), Some(len)) = (num(1), len) else {
                return R::V(V::Null);
            };
            if first == V::Null {
                return R::V(V::Null);
            }
            let t = as_text(first, &mut s.a);
            let total = utf16_len(t);
            let start = js_max(0.0, js_round(from) - 1.0);
            let end = if len == f64::INFINITY {
                None
            } else {
                Some(units(start + js_max(0.0, js_round(len)), total))
            };
            text::push_slice(out, t, units(start, total), end);
            return R::Made;
        }
        Func::Pad => {
            let Some(len) = num(1) else {
                return R::V(V::Null);
            };
            if first == V::Null {
                return R::V(V::Null);
            }
            let fill = match args.get(2) {
                None => u16::from(b'0'),
                Some(&c) => text::first_unit(as_text(c, &mut s.b)).unwrap_or(u16::from(b'0')),
            };
            let target = js_max(0.0, js_round(len));
            let V::Text(t) = first else {
                // A number's (or true/false's) text is written, then filled in front of.
                let mark = out.len();
                push_text(out, first);
                let n = utf16_len(&out[mark..]);
                if target <= n as f64 {
                    return R::Made;
                }
                if target > MAX_STRING_UNITS as f64 {
                    return R::Thrown;
                }
                let c = char::from_u32(u32::from(fill)).unwrap_or('\u{fffd}');
                s.a.clear();
                s.a.extend(std::iter::repeat_n(c, target as usize - n));
                out.insert_str(mark, &s.a);
                return R::Made;
            };
            if target <= utf16_len(t) as f64 {
                return R::V(V::Text(t));
            }
            if target > MAX_STRING_UNITS as f64
                || !text::push_pad_start(out, t, target as usize, fill)
            {
                return R::Thrown;
            }
            return R::Made;
        }
        Func::Replace => {
            let Scratch {
                a: sa,
                b: sb,
                c: sc,
            } = s;
            let (t, from, to) = (as_text(first, sa), as_text(arg(1), sb), as_text(arg(2), sc));
            return if text::push_split_join(out, t, from, to) {
                R::Made
            } else {
                R::Thrown
            };
        }
        Func::Contains | Func::Starts | Func::Ends if first == V::Null => V::Bool(false),
        Func::Contains | Func::Starts | Func::Ends => {
            let Scratch {
                a: sa,
                b: sb,
                c: sc,
            } = s;
            fold_into(sa, first, sc);
            fold_into(sb, arg(1), sc);
            V::Bool(match f {
                Func::Contains => sa.contains(sb.as_str()),
                Func::Starts => sa.starts_with(sb.as_str()),
                _ => sa.ends_with(sb.as_str()),
            })
        }
        Func::If => {
            if truthy(first) {
                arg(1)
            } else {
                arg(2)
            }
        }
        Func::Empty => V::Bool(is_empty(first)),
        Func::Coalesce => args
            .iter()
            .copied()
            .find(|&a| !is_empty(a))
            .unwrap_or(V::Null),
    })
}
