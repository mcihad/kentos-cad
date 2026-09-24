//! Robust geometric predicates (CLAUDE.md §23.3): which side of a line a
//! point lies on is decided exactly for float64 input, never by a fixed
//! epsilon.
//!
//! Jonathan Richard Shewchuk's adaptive `orient2d` ("Adaptive Precision
//! Floating-Point Arithmetic and Fast Robust Geometric Predicates", 1997;
//! `predicates.c`, public domain), written again here instead of taken as a
//! dependency: the plain determinant when its error bound proves the sign,
//! otherwise more and more of the exact expansion until the sign is certain.
//! Only +, − and × of float64 are used, never a fused multiply-add
//! (`clippy.toml`), so native and WASM give the same bits. The error bounds
//! assume no underflow: coordinates are metres, far above 1e-150.

use crate::api::Op;
use crate::op;
use crate::vec2::Vec2;

/// 2⁻⁵³: half an ulp of 1, the relative rounding error of one operation.
const EPSILON: f64 = f64::EPSILON / 2.0;
/// 2²⁷ + 1: splits a float64 into two halves of 26 significant bits.
const SPLITTER: f64 = 134_217_729.0;
const RESULT_ERRBOUND: f64 = (3.0 + 8.0 * EPSILON) * EPSILON;
const CCW_ERRBOUND_A: f64 = (3.0 + 16.0 * EPSILON) * EPSILON;
const CCW_ERRBOUND_B: f64 = (2.0 + 12.0 * EPSILON) * EPSILON;
const CCW_ERRBOUND_C: f64 = (9.0 + 64.0 * EPSILON) * EPSILON * EPSILON;

/// Twice the signed area of the triangle a, b, c with its sign exact:
/// positive when c lies left of a→b (a counter-clockwise turn), negative
/// when right, zero only when the three points are collinear. The magnitude
/// is an approximation; decide with the sign. NaN input gives NaN or an
/// arbitrary sign, not a decision.
pub fn orient2d(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    let detleft = (a.x - c.x) * (b.y - c.y);
    let detright = (a.y - c.y) * (b.x - c.x);
    let det = detleft - detright;
    let detsum = if detleft > 0.0 {
        if detright <= 0.0 {
            return det;
        }
        detleft + detright
    } else if detleft < 0.0 {
        if detright >= 0.0 {
            return det;
        }
        -detleft - detright
    } else {
        return det;
    };
    let errbound = CCW_ERRBOUND_A * detsum;
    if det >= errbound || -det >= errbound {
        return det;
    }
    orient2d_adapt(a, b, c, detsum)
}

/// Which side of the directed line a→b the point c lies on, exactly: 1 left
/// (counter-clockwise), −1 right, 0 on the line (NaN input: 0).
pub fn orientation(a: Vec2, b: Vec2, c: Vec2) -> i8 {
    let d = orient2d(a, b, c);
    if d > 0.0 {
        1
    } else if d < 0.0 {
        -1
    } else {
        0
    }
}

/// The later stages of `orient2d`: the determinant as an exact expansion of
/// the rounded differences, then their rounding errors (the tails), each
/// stage only when the previous one cannot prove the sign.
fn orient2d_adapt(a: Vec2, b: Vec2, c: Vec2, detsum: f64) -> f64 {
    let acx = a.x - c.x;
    let bcx = b.x - c.x;
    let acy = a.y - c.y;
    let bcy = b.y - c.y;

    let (detleft, detlefttail) = two_product(acx, bcy);
    let (detright, detrighttail) = two_product(acy, bcx);
    let b4 = two_two_diff(detleft, detlefttail, detright, detrighttail);
    let mut det = estimate(&b4);
    let errbound = CCW_ERRBOUND_B * detsum;
    if det >= errbound || -det >= errbound {
        return det;
    }

    let acxtail = two_diff_tail(a.x, c.x, acx);
    let bcxtail = two_diff_tail(b.x, c.x, bcx);
    let acytail = two_diff_tail(a.y, c.y, acy);
    let bcytail = two_diff_tail(b.y, c.y, bcy);
    if acxtail == 0.0 && acytail == 0.0 && bcxtail == 0.0 && bcytail == 0.0 {
        // The differences were exact, so the expansion above is the determinant.
        return det;
    }

    let errbound = CCW_ERRBOUND_C * detsum + RESULT_ERRBOUND * det.abs();
    det += (acx * bcytail + bcy * acxtail) - (acy * bcxtail + bcx * acytail);
    if det >= errbound || -det >= errbound {
        return det;
    }

    let (s1, s0) = two_product(acxtail, bcy);
    let (t1, t0) = two_product(acytail, bcx);
    let u = two_two_diff(s1, s0, t1, t0);
    let mut c1 = [0.0; 8];
    let c1len = fast_expansion_sum_zeroelim(&b4, &u, &mut c1);

    let (s1, s0) = two_product(acx, bcytail);
    let (t1, t0) = two_product(acy, bcxtail);
    let u = two_two_diff(s1, s0, t1, t0);
    let mut c2 = [0.0; 12];
    let c2len = fast_expansion_sum_zeroelim(prefix(&c1, c1len), &u, &mut c2);

    let (s1, s0) = two_product(acxtail, bcytail);
    let (t1, t0) = two_product(acytail, bcxtail);
    let u = two_two_diff(s1, s0, t1, t0);
    let mut d = [0.0; 16];
    let dlen = fast_expansion_sum_zeroelim(prefix(&c2, c2len), &u, &mut d);

    // The most significant component carries the sign of the whole expansion.
    prefix(&d, dlen).last().copied().unwrap_or(0.0)
}

// ── Exact float64 arithmetic (Shewchuk's macros) ────────────────────────
//
// An expansion is a sum of nonoverlapping float64 components, least
// significant first; it represents its exact sum.

/// a + b = x + y exactly, when |a| ≥ |b| (or a = 0).
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bvirt = x - a;
    (x, b - bvirt)
}

/// a + b = x + y exactly.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bvirt = x - a;
    let avirt = x - bvirt;
    let bround = b - bvirt;
    let around = a - avirt;
    (x, around + bround)
}

/// The rounding error y of x = a − b: a − b = x + y exactly.
fn two_diff_tail(a: f64, b: f64, x: f64) -> f64 {
    let bvirt = a - x;
    let avirt = x + bvirt;
    let bround = bvirt - b;
    let around = a - avirt;
    around + bround
}

/// a − b = x + y exactly.
fn two_diff(a: f64, b: f64) -> (f64, f64) {
    let x = a - b;
    (x, two_diff_tail(a, b, x))
}

/// a = hi + lo, each half fitting in 26 bits, so products of halves are exact.
fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let abig = c - a;
    let hi = c - abig;
    (hi, a - hi)
}

/// a · b = x + y exactly (Dekker's product through split halves).
fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    let (ahi, alo) = split(a);
    let (bhi, blo) = split(b);
    let err1 = x - ahi * bhi;
    let err2 = err1 - alo * bhi;
    let err3 = err2 - ahi * blo;
    (x, alo * blo - err3)
}

/// (a1 + a0) − (b1 + b0) as a four-component expansion.
fn two_two_diff(a1: f64, a0: f64, b1: f64, b0: f64) -> [f64; 4] {
    let (i, x0) = two_diff(a0, b0);
    let (j, zero) = two_sum(a1, i);
    let (i, x1) = two_diff(zero, b1);
    let (x3, x2) = two_sum(j, i);
    [x0, x1, x2, x3]
}

/// An approximation of an expansion's value (its components summed in order).
fn estimate(e: &[f64]) -> f64 {
    let mut q = component(e, 0);
    for &x in e.iter().skip(1) {
        q += x;
    }
    q
}

fn component(e: &[f64], i: usize) -> f64 {
    e.get(i).copied().unwrap_or(0.0)
}

fn prefix(e: &[f64], len: usize) -> &[f64] {
    e.get(..len).unwrap_or(e)
}

/// h = e + f for two expansions, zero components dropped; returns h's
/// length (`h` holds at least e.len() + f.len() components).
fn fast_expansion_sum_zeroelim(e: &[f64], f: &[f64], h: &mut [f64]) -> usize {
    let mut n = 0;
    let (mut ei, mut fi) = (0, 0);
    let mut enow = component(e, 0);
    let mut fnow = component(f, 0);
    // Take the component of smaller magnitude first.
    let smaller_e = |fnow: f64, enow: f64| (fnow > enow) == (fnow > -enow);
    let mut q = if smaller_e(fnow, enow) {
        ei += 1;
        let q = enow;
        enow = component(e, ei);
        q
    } else {
        fi += 1;
        let q = fnow;
        fnow = component(f, fi);
        q
    };
    if ei < e.len() && fi < f.len() {
        let (qnew, hh) = if smaller_e(fnow, enow) {
            let r = fast_two_sum(enow, q);
            ei += 1;
            enow = component(e, ei);
            r
        } else {
            let r = fast_two_sum(fnow, q);
            fi += 1;
            fnow = component(f, fi);
            r
        };
        q = qnew;
        if hh != 0.0 {
            push(h, &mut n, hh);
        }
        while ei < e.len() && fi < f.len() {
            let (qnew, hh) = if smaller_e(fnow, enow) {
                let r = two_sum(q, enow);
                ei += 1;
                enow = component(e, ei);
                r
            } else {
                let r = two_sum(q, fnow);
                fi += 1;
                fnow = component(f, fi);
                r
            };
            q = qnew;
            if hh != 0.0 {
                push(h, &mut n, hh);
            }
        }
    }
    while ei < e.len() {
        let (qnew, hh) = two_sum(q, enow);
        ei += 1;
        enow = component(e, ei);
        q = qnew;
        if hh != 0.0 {
            push(h, &mut n, hh);
        }
    }
    while fi < f.len() {
        let (qnew, hh) = two_sum(q, fnow);
        fi += 1;
        fnow = component(f, fi);
        q = qnew;
        if hh != 0.0 {
            push(h, &mut n, hh);
        }
    }
    if q != 0.0 || n == 0 {
        push(h, &mut n, q);
    }
    n
}

fn push(h: &mut [f64], n: &mut usize, v: f64) {
    if let Some(slot) = h.get_mut(*n) {
        *slot = v;
        *n += 1;
    }
}

pub(crate) static OPS: &[Op] = &[op!("orientation", |a: Vec2, b: Vec2, c: Vec2| {
    f64::from(orientation(a, b, c))
})];

#[cfg(test)]
mod tests {
    use super::*;

    const fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    /// The textbook determinant, rounded at every step (what the core used).
    fn naive(a: Vec2, b: Vec2, c: Vec2) -> i8 {
        let d = (a.x - c.x) * (b.y - c.y) - (a.y - c.y) * (b.x - c.x);
        if d > 0.0 {
            1
        } else if d < 0.0 {
            -1
        } else {
            0
        }
    }

    /// The exact sign for coordinates on a 2⁻ˢ grid small enough that the
    /// scaled values, their differences and products fit in i128: plain
    /// integer arithmetic, independent of the expansion code above.
    fn exact(a: Vec2, b: Vec2, c: Vec2, scale: u32) -> i8 {
        let unit = (1u64 << scale) as f64;
        let k = |x: f64| {
            let s = x * unit;
            assert!(
                s == s.trunc() && s.abs() < 9.0e18,
                "{x} is not on the 2^-{scale} grid"
            );
            s as i128
        };
        let (ax, ay, bx, by, cx, cy) = (k(a.x), k(a.y), k(b.x), k(b.y), k(c.x), k(c.y));
        let d = (ax - cx) * (by - cy) - (ay - cy) * (bx - cx);
        d.signum() as i8
    }

    /// The next float64 up or down (no `next_up` on the toolchain's MSRV).
    fn step(x: f64, up: bool) -> f64 {
        let bits = x.to_bits();
        let next = if (x > 0.0) == up { bits + 1 } else { bits - 1 };
        f64::from_bits(next)
    }

    /// Deterministic random numbers (xorshift64*), the same on every target.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
        }
        fn range(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * self.next()
        }
    }

    #[test]
    fn plain_turns() {
        assert_eq!(orientation(v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0)), 1);
        assert_eq!(orientation(v(0.0, 0.0), v(1.0, 0.0), v(0.0, -1.0)), -1);
        assert_eq!(orientation(v(0.0, 0.0), v(1.0, 1.0), v(2.0, 2.0)), 0);
        assert_eq!(orientation(v(3.0, 4.0), v(3.0, 4.0), v(7.0, -1.0)), 0);
        assert_eq!(orientation(v(1.0, 1.0), v(1.0, 1.0), v(1.0, 1.0)), 0);
        assert!(orient2d(v(0.0, 0.0), v(2.0, 0.0), v(0.0, 3.0)) == 6.0);
    }

    #[test]
    fn nan_is_no_decision_and_does_not_panic() {
        assert_eq!(orientation(v(f64::NAN, 0.0), v(1.0, 0.0), v(0.0, 1.0)), 0);
        let _ = orient2d(v(f64::INFINITY, 0.0), v(1.0, 1e300), v(-1e300, 1.0));
    }

    /// One ulp off an exactly collinear TM point decides the side.
    #[test]
    fn one_ulp_off_a_tm_line() {
        let a = v(486_512.5, 4_420_187.25);
        let b = v(486_512.5 + 80.5, 4_420_187.25 + 60.25);
        let m = v(486_512.5 + 40.25, 4_420_187.25 + 30.125); // exactly on a→b
        assert_eq!(orientation(a, b, m), 0);
        assert_eq!(orientation(a, b, v(m.x, step(m.y, true))), 1);
        assert_eq!(orientation(a, b, v(m.x, step(m.y, false))), -1);
        assert_eq!(orientation(a, b, v(step(m.x, true), m.y)), -1);
        assert_eq!(orientation(a, b, v(step(m.x, false), m.y)), 1);
        // Far along the line the determinant is huge and the ulp tiny.
        let far = v(486_512.5 + 80.5 * 1024.0, 4_420_187.25 + 60.25 * 1024.0);
        assert_eq!(orientation(a, far, v(m.x, step(m.y, true))), 1);
        assert_eq!(orientation(far, a, v(m.x, step(m.y, true))), -1);
    }

    /// Kettner et al., "Classroom examples of robustness problems in
    /// geometric computations": points a few ulps around (0.5, 0.5) against
    /// the line through (12, 12) and (24, 24). The rounded determinant gets
    /// many of them wrong; the predicate must match exact integer arithmetic
    /// on the 2⁻⁵³ grid everywhere.
    #[test]
    fn classroom_grid_matches_exact_arithmetic() {
        let u = EPSILON; // the ulp of 0.5
        let (q, r) = (v(12.0, 12.0), v(24.0, 24.0));
        let mut naive_wrong = 0;
        for i in 0..64 {
            for j in 0..64 {
                let p = v(0.5 + f64::from(i) * u, 0.5 + f64::from(j) * u);
                for (a, b, c) in [(p, q, r), (q, r, p), (r, p, q), (q, p, r)] {
                    let want = exact(a, b, c, 53);
                    assert_eq!(orientation(a, b, c), want, "{a:?} {b:?} {c:?}");
                    if naive(a, b, c) != want {
                        naive_wrong += 1;
                    }
                }
            }
        }
        assert!(naive_wrong > 0, "the grid no longer exercises rounding");
    }

    /// Random near-collinear triples at TM coordinates (every float64 there
    /// lies on the 2⁻³⁴ grid): the predicate matches exact integer
    /// arithmetic and is consistent under permutation. (The rounded
    /// determinant is rarely wrong here, since differences of TM coordinates
    /// are exact; points computed far apart, as in the classroom grid, are
    /// where it fails.)
    #[test]
    fn tm_near_collinear_triples_match_exact_arithmetic() {
        let mut g = Rng(0x9e37_79b9_7f4a_7c15);
        for n in 0..200_000 {
            let a = v(
                g.range(400_000.0, 600_000.0),
                g.range(4_200_000.0, 4_600_000.0),
            );
            let reach = if n % 3 == 0 { 5.0 } else { 5_000.0 };
            let b = v(a.x + g.range(-reach, reach), a.y + g.range(-reach, reach));
            let t = g.range(-2.0, 3.0);
            let mut c = v(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
            for _ in 0..(n % 4) {
                c = if g.next() < 0.5 {
                    v(step(c.x, g.next() < 0.5), c.y)
                } else {
                    v(c.x, step(c.y, g.next() < 0.5))
                };
            }
            let want = exact(a, b, c, 34);
            let got = orientation(a, b, c);
            assert_eq!(got, want, "{a:?} {b:?} {c:?}");
            assert_eq!(orientation(b, c, a), want);
            assert_eq!(orientation(c, a, b), want);
            assert_eq!(orientation(b, a, c), -want);
        }
    }

    /// Far from degenerate the plain determinant is returned untouched.
    #[test]
    fn easy_cases_take_the_plain_determinant() {
        let mut g = Rng(42);
        for _ in 0..10_000 {
            let (a, b, c) = (
                v(g.range(-1e3, 1e3), g.range(-1e3, 1e3)),
                v(g.range(-1e3, 1e3), g.range(-1e3, 1e3)),
                v(g.range(-1e3, 1e3), g.range(-1e3, 1e3)),
            );
            let plain = (a.x - c.x) * (b.y - c.y) - (a.y - c.y) * (b.x - c.x);
            if plain.abs() > 1e-3 {
                assert_eq!(orient2d(a, b, c).to_bits(), plain.to_bits());
            }
        }
    }

    #[test]
    fn expansion_sums_are_exact() {
        // (2⁻⁶⁰ + 1) + (−1) = 2⁻⁶⁰ exactly, though no float64 sum gives it.
        let tiny = 1.0 / (1u64 << 60) as f64;
        let mut h = [0.0; 4];
        let n = fast_expansion_sum_zeroelim(&[tiny, 1.0], &[-1.0], &mut h);
        assert_eq!(&h[..n], &[tiny]);
        let n = fast_expansion_sum_zeroelim(&[1.0], &[-1.0], &mut h);
        assert_eq!(&h[..n], &[0.0]);
        let (x, y) = two_product(1.0 + EPSILON * 2.0, 1.0 + EPSILON * 2.0);
        assert_eq!(x, 1.0 + EPSILON * 4.0);
        assert_eq!(y, EPSILON * EPSILON * 4.0);
    }
}
