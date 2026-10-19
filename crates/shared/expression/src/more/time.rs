//! `epok`, `epoktan` and `saat_yap` (docs/adr/0600 §3). Dates are without a zone (docs/adr/0214 §2.1): a moment's
//! milliseconds count from 1970-01-01T00:00:00 as UTC. A time of day is text `SS:DD:ss`, with `.mmm` when it has
//! milliseconds.

use std::fmt::Write as _;

use kentos_geometry_core::jsmath::js_round;
use kentos_geometry_core::time;

use crate::dates;
use crate::library::Func;
use crate::scalar::{R, V, to_number};

const DAY: f64 = 86_400_000.0;
/// The moments the language writes (years 1–9999, docs/adr/0214 §2.2).
const FIRST: f64 = -62_135_596_800_000.0;
const LAST: f64 = 253_402_300_799_999.0;

fn push_time(out: &mut String, ms: i64) {
    let (h, m, s, f) = (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000);
    let _ = write!(out, "{h:02}:{m:02}:{s:02}");
    if f != 0 {
        let _ = write!(out, ".{f:03}");
    }
}

/// A time of day read from text: `S:DD`, `S:DD:ss`, `S:DD:ss.fff`; None otherwise.
fn read_time(t: &str) -> Option<i64> {
    let t = t.trim();
    let mut parts = t.split(':');
    let h: i64 = parts.next()?.trim().parse().ok()?;
    let m: i64 = parts.next()?.trim().parse().ok()?;
    let s: f64 = match parts.next() {
        Some(p) => p.trim().parse().ok()?,
        None => 0.0,
    };
    if parts.next().is_some()
        || !(0..24).contains(&h)
        || !(0..60).contains(&m)
        || !(0.0..60.0).contains(&s)
    {
        return None;
    }
    Some(h * 3_600_000 + m * 60_000 + js_round(s * 1000.0) as i64)
}

pub(super) fn call<'a>(f: Func, args: &[V<'a>], out: &mut String) -> R<'a> {
    let arg = |i: usize| super::arg(args, i);
    match f {
        Func::Epoch => match dates::moment(arg(0)) {
            Some((t, _)) => R::V(V::Num(t as f64)),
            None => R::V(V::Null),
        },
        Func::FromEpoch => {
            let Some(ms) = to_number(arg(0)) else {
                return R::V(V::Null);
            };
            let ms = js_round(ms);
            if !(FIRST..=LAST).contains(&ms) {
                return R::V(V::Null);
            }
            out.push_str(&time::write(ms, false));
            R::Made
        }
        Func::MakeTime => {
            let ms = if args.len() == 1 {
                match arg(0) {
                    V::Text(t) => read_time(crate::compound::shown(t)),
                    _ => None,
                }
            } else {
                let h = to_number(arg(0));
                let m = to_number(arg(1));
                let s = if args.len() > 2 {
                    to_number(arg(2))
                } else {
                    Some(0.0)
                };
                match (h, m, s) {
                    (Some(h), Some(m), Some(s))
                        if h.fract() == 0.0
                            && m.fract() == 0.0
                            && (0.0..24.0).contains(&h)
                            && (0.0..60.0).contains(&m)
                            && (0.0..60.0).contains(&s) =>
                    {
                        let ms = h * 3_600_000.0 + m * 60_000.0 + js_round(s * 1000.0);
                        (ms < DAY).then_some(ms as i64)
                    }
                    _ => None,
                }
            };
            match ms {
                Some(ms) => {
                    push_time(out, ms);
                    R::Made
                }
                None => R::V(V::Null),
            }
        }
        _ => R::V(V::Null),
    }
}
