//! `rastgele`, `rastgele_ondalık` and `uuid` (docs/adr/0600 §3). The generator is SplitMix64 (docs/adr/0220's): a
//! seed gives the same number on every platform. A call without a seed is given one when it compiles (`resolve`): the
//! call's own number, made of the run's moment and the call's place in the expression, and the object's position in the
//! run; so a call is never folded into a constant, two calls are two registers, and each object and each run draws anew,
//! while the language stays pure (no clock or system call in the core).

use std::fmt::Write as _;

use kentos_geometry_core::jsmath::js_min;

use crate::library::Func;
use crate::scalar::{R, V, to_number};

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a of a text, the seed of a text value.
fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// A seed value's bits: a number's, a text's hash, true/false; none for an empty value.
fn seed_of(v: V) -> Option<u64> {
    match v {
        V::Null => None,
        V::Bool(b) => Some(u64::from(b)),
        V::Text(t) => match to_number(v) {
            Some(x) => Some((x + 0.0).to_bits()),
            None => Some(fnv(t)),
        },
        V::Num(x) => Some((x + 0.0).to_bits()),
    }
}

/// The number an unseeded call is compiled with: the run's moment (its text, or none) and the call's place in the
/// expression, as a whole number a double holds exactly.
pub fn site_seed(moment: Option<&str>, place: u32) -> f64 {
    let base = moment.map_or(0, fnv);
    (mix(base ^ u64::from(place).wrapping_mul(0xA24B_AED4_963E_E407)) >> 11) as f64
}

/// The draw of a call: its seed when given, else the compiled site with the object's position (and, inside an inner
/// expression, the item's).
fn draw(seed: V, site: V, index: V, item: V) -> Option<u64> {
    if let Some(s) = seed_of(seed) {
        return Some(mix(s));
    }
    let site = to_number(site)? as u64;
    let index = to_number(index).map_or(0, |x| (x + 0.0).to_bits());
    let item = to_number(item).map_or(0, |x| mix((x + 0.0).to_bits()));
    Some(mix(site ^ mix(index) ^ item))
}

/// A number in [0, 1) from 53 bits.
fn unit(r: u64) -> f64 {
    (r >> 11) as f64 / (1u64 << 53) as f64
}

/// A version 4 UUID from two draws.
pub fn uuid_text(out: &mut String, a: u64, b: u64) {
    let a = (a & 0xFFFF_FFFF_FFFF_0FFF) | 0x0000_0000_0000_4000;
    let b = (b & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;
    let _ = write!(
        out,
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        a >> 32,
        (a >> 16) & 0xFFFF,
        a & 0xFFFF,
        b >> 48,
        b & 0xFFFF_FFFF_FFFF
    );
}

pub(super) fn call<'a>(f: Func, args: &[V<'a>], out: &mut String) -> R<'a> {
    let arg = |i: usize| super::arg(args, i);
    match f {
        Func::Uuid => {
            // Compiled with the site and the position (§3); a call the resolver did not see draws at 0.
            let r = draw(V::Null, arg(0), arg(1), arg(2)).unwrap_or_else(|| mix(0));
            uuid_text(out, r, mix(r));
            R::Made
        }
        Func::Rand => {
            let (Some(lo), Some(hi)) = (to_number(arg(0)), to_number(arg(1))) else {
                return R::V(V::Null);
            };
            let (lo, hi) = (lo.trunc(), hi.trunc());
            if hi < lo {
                return R::V(V::Null);
            }
            let Some(r) = draw(arg(2), arg(3), arg(4), arg(5)) else {
                return R::V(V::Null);
            };
            let span = hi - lo + 1.0;
            R::V(V::Num(js_min(lo + (unit(r) * span).floor(), hi)))
        }
        Func::RandF => {
            let lo = if args.is_empty() {
                Some(0.0)
            } else {
                to_number(arg(0))
            };
            let hi = if args.len() < 2 {
                Some(1.0)
            } else {
                to_number(arg(1))
            };
            let (Some(lo), Some(hi)) = (lo, hi) else {
                return R::V(V::Null);
            };
            if hi < lo {
                return R::V(V::Null);
            }
            let Some(r) = draw(arg(2), arg(3), arg(4), arg(5)) else {
                return R::V(V::Null);
            };
            R::V(V::Num(lo + unit(r) * (hi - lo)))
        }
        _ => R::V(V::Null),
    }
}
