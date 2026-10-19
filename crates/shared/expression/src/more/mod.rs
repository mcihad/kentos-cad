//! What docs/adr/0600's functions compute, on one object's values by the rules of `scalar`: conversions, random
//! numbers, math, text, arrays and maps, times. Text a function makes is written at the end of the caller's buffer, so
//! a column evaluates without allocating per object except where the work itself needs room (a sort, a table).

mod convert;
mod lists;
mod math;
mod random;
mod text;
mod time;
mod unaccent_table;

use crate::library::Func;
use crate::scalar::{R, Scratch, V};

pub use random::{site_seed, uuid_text};

/// Calls one of docs/adr/0600's functions on its arguments (`args.len()` within its arity, and for an unseeded random
/// call the two values the resolver adds, §3).
pub fn call<'a>(f: Func, args: &[V<'a>], out: &mut String, s: &mut Scratch) -> R<'a> {
    use Func::*;
    match f {
        ToBool | ToDecimal | ToDms | ToDm | ToBase64 | FromBase64 | Hash | Md5 | Sha256 => {
            convert::call(f, args, out, s)
        }
        Rand | RandF | Uuid => random::call(f, args, out),
        Clamp | ScaleLinear | ScaleExponential | ScalePolynomial => math::call(f, args),
        Ltrim | Rtrim | Repeat | Reverse | Title | SubstrCount | ConcatWs | Format | Char
        | Ascii | Unaccent | WordWrap | Levenshtein | Hamming | CommonPart | NullIf => {
            text::call(f, args, out, s)
        }
        ArrayAll | ArrayCount | ArrayInsert | ArrayPrepend | ArrayRemoveAt | ArrayRemoveAll
        | ArrayReplace | ArrayIntersect | ArrayMajority | ArrayMinority | ArrayMedian | Series
        | MapConcat | MapPrefixKeys | UrlEncode => lists::call(f, args, out, s),
        Epoch | FromEpoch | MakeTime => time::call(f, args, out),
        _ => R::V(V::Null),
    }
}

/// The argument `i`, empty when not given.
fn arg<'a>(args: &[V<'a>], i: usize) -> V<'a> {
    args.get(i).copied().unwrap_or(V::Null)
}

/// Text written at `mark` as the result, unless it is past the longest text the language holds.
fn made(out: &str, mark: usize) -> R<'static> {
    crate::arrays::made(out, mark)
}
