//! `sınırla` and the scales (docs/adr/0600 §3), QGIS's rules: a value outside the source range is held at its end;
//! a source range whose least end is not below its greatest, or a base not above 0, gives an empty value.

use kentos_geometry_core::jsmath::pow;

use crate::library::Func;
use crate::scalar::{R, V, to_number};

pub(super) fn call<'a>(f: Func, args: &[V<'a>]) -> R<'a> {
    let mut xs = [0.0f64; 6];
    for (i, x) in xs.iter_mut().enumerate().take(args.len()) {
        match to_number(args[i]) {
            Some(n) => *x = n,
            None => return R::V(V::Null),
        }
    }
    let finite = |x: f64| R::V(if x.is_finite() { V::Num(x) } else { V::Null });
    match f {
        Func::Clamp => {
            let [lo, v, hi, ..] = xs;
            finite(if v < lo {
                lo
            } else if v > hi {
                hi
            } else {
                v
            })
        }
        Func::ScaleLinear | Func::ScaleExponential | Func::ScalePolynomial => {
            let [v, d0, d1, r0, r1, k] = xs;
            if d0 >= d1 || (f == Func::ScaleExponential && k <= 0.0) {
                return R::V(V::Null);
            }
            if v >= d1 {
                return finite(r1);
            }
            if v <= d0 {
                return finite(r0);
            }
            finite(match f {
                Func::ScaleLinear => {
                    let m = (r1 - r0) / (d1 - d0);
                    m * v + (r0 - d0 * m)
                }
                Func::ScaleExponential => {
                    (r1 - r0) * ((pow(k, v - d0) - 1.0) / (pow(k, d1 - d0) - 1.0)) + r0
                }
                _ => (r1 - r0) / pow(d1 - d0, k) * pow(v - d0, k) + r0,
            })
        }
        _ => R::V(V::Null),
    }
}
