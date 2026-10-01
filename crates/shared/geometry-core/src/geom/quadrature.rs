//! Lengths of curves without a closed form (an ellipse's arc, a curve's
//! span) by adaptive Gauss–Legendre quadrature (docs/adr/0149 §3): ten
//! nodes on an interval, its halves compared with it, a half refined again
//! until they agree to the tolerance. Ten nodes are exact for polynomials
//! to degree 19, so a curve's smooth speed settles in a level or two; a
//! sharp one (a very flat ellipse) is split where it bends. The same steps
//! run on every target, so the bits are the same natively and in WASM.

/// Gauss–Legendre nodes on [−1, 1] (the positive half) and their weights
/// (Abramowitz and Stegun, table 25.4).
const NODES: [f64; 5] = [
    0.148_874_338_981_631_22,
    0.433_395_394_129_247_2,
    0.679_409_568_299_024_4,
    0.865_063_366_688_984_5,
    0.973_906_528_517_171_7,
];
const WEIGHTS: [f64; 5] = [
    0.295_524_224_714_752_87,
    0.269_266_719_309_996_35,
    0.219_086_362_515_982_04,
    0.149_451_349_150_580_6,
    0.066_671_344_308_688_14,
];

/// How deep a half is refined at most (2⁻³⁰ of the interval).
const DEPTH: u32 = 30;

/// The ten-node rule on [a, b].
fn rule(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    let (m, h) = ((a + b) / 2.0, (b - a) / 2.0);
    let mut s = 0.0;
    for k in 0..5 {
        s += WEIGHTS[k] * (f(m - h * NODES[k]) + f(m + h * NODES[k]));
    }
    s * h
}

/// ∫ f over [a, b], to `rel` of the whole (an absolute floor of `rel`
/// times 10⁻³⁰⁰ keeps a zero integral from refining forever).
pub fn integrate(f: &dyn Fn(f64) -> f64, a: f64, b: f64, rel: f64) -> f64 {
    let whole = rule(f, a, b);
    let tol = rel * whole.abs() + rel * 1e-300;
    refine(f, a, b, whole, tol, 0)
}

fn refine(f: &dyn Fn(f64) -> f64, a: f64, b: f64, whole: f64, tol: f64, depth: u32) -> f64 {
    let m = (a + b) / 2.0;
    let (left, right) = (rule(f, a, m), rule(f, m, b));
    let both = left + right;
    if depth >= DEPTH || (both - whole).abs() <= tol {
        return both;
    }
    refine(f, a, m, left, tol / 2.0, depth + 1) + refine(f, m, b, right, tol / 2.0, depth + 1)
}

#[cfg(test)]
mod tests {
    use super::integrate;
    use crate::jsmath::{PI, cos, sin};

    #[test]
    fn polynomials_and_smooth_functions_come_out_exact() {
        // x⁵ on [0, 2]: 64/6.
        let p = integrate(&|x: f64| x * x * x * x * x, 0.0, 2.0, 1e-15);
        assert!((p - 64.0 / 6.0).abs() < 1e-13, "{p}");
        // sin on [0, π]: 2.
        let s = integrate(&|x: f64| sin(x), 0.0, PI, 1e-15);
        assert!((s - 2.0).abs() < 1e-14, "{s}");
        // A sharp valley: √(cos² + 10⁻⁴ sin²) on [0, 2π] (a 0.01-flat ellipse of unit major axis).
        let e = integrate(
            &|t: f64| (sin(t) * sin(t) + 1e-4 * cos(t) * cos(t)).sqrt(),
            0.0,
            2.0 * PI,
            1e-15,
        );
        // 4·E(m = 1 − 10⁻⁴), mpmath: 4.00109832972265186074746…
        assert!((e - 4.001_098_329_722_652).abs() < 1e-13, "{e}");
    }
}
