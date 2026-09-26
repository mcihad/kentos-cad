//! 2D affine transforms `[a, b, c, d, e, f]` (`apps/web/src/model/geom/affine.ts`):
//! x' = a·x + c·y + e, y' = b·x + d·y + f.

use crate::api::Op;
use crate::geometry::dist;
use crate::jsmath::{atan2, cos, or, sin};
use crate::op;
use crate::vec2::Vec2;

pub type Affine = [f64; 6];

pub const IDENTITY: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
const ORIGIN: Vec2 = Vec2::new(0.0, 0.0);

pub fn translation(dx: f64, dy: f64) -> Affine {
    [1.0, 0.0, 0.0, 1.0, dx, dy]
}

/// Rotation by `angle` radians (CCW) around `o`.
pub fn rotation(angle: f64, o: Vec2) -> Affine {
    let c = cos(angle);
    let s = sin(angle);
    [
        c,
        s,
        -s,
        c,
        o.x - c * o.x + s * o.y,
        o.y - s * o.x - c * o.y,
    ]
}

/// Uniform scale around `o`.
pub fn scaling(s: f64, o: Vec2) -> Affine {
    [s, 0.0, 0.0, s, o.x * (1.0 - s), o.y * (1.0 - s)]
}

/// Reflection across the line through `p` and `q`.
pub fn mirror(p: Vec2, q: Vec2) -> Affine {
    let dx = q.x - p.x;
    let dy = q.y - p.y;
    let l2 = or(dx * dx + dy * dy, 1.0);
    let a = (dx * dx - dy * dy) / l2;
    let b = (2.0 * dx * dy) / l2;
    [
        a,
        b,
        b,
        -a,
        p.x - a * p.x - b * p.y,
        p.y - b * p.x + a * p.y,
    ]
}

/// Align (Hizala, AutoCAD's ALIGN; docs/adr/0047): the first source point
/// onto the first target; with a second pair the direction from the first
/// source point to the second turns onto the direction from the first target
/// to the second, and with `scale` the one length becomes the other. `pts`
/// is s1, d1, s2, d2 as far as given; a third point alone is not a pair.
/// None without a first pair, or when a second pair's points lie within a
/// nanometre of the first's (the direction is lost).
pub fn align(pts: &[Vec2], scale: bool) -> Option<Affine> {
    let (Some(&s1), Some(&d1)) = (pts.first(), pts.get(1)) else {
        return None;
    };
    let (Some(&s2), Some(&d2)) = (pts.get(2), pts.get(3)) else {
        return Some(translation(d1.x - s1.x, d1.y - s1.y));
    };
    let ls = dist(s1, s2);
    let ld = dist(d1, d2);
    if ls < 1e-9 || ld < 1e-9 {
        return None;
    }
    let turn = atan2(d2.y - d1.y, d2.x - d1.x) - atan2(s2.y - s1.y, s2.x - s1.x);
    let k = if scale { ld / ls } else { 1.0 };
    // Around s1: scale, turn, then carry s1 onto d1.
    Some(compose(
        &translation(d1.x - s1.x, d1.y - s1.y),
        &compose(&rotation(turn, s1), &scaling(k, s1)),
    ))
}

/// The affine of a similarity as the modify tools and the product command
/// `cad.entities.transform` give it (docs/adr/0037, 0047): `move` (dx, dy),
/// `rotate` (cx, cy, angle in radians, counter-clockwise), `scale` (cx, cy,
/// factor), `mirror` (ax, ay, bx, by), `align` (the first source and target
/// points sx, sy, tx, ty, then the second pair's when it is given) or
/// `alignScale` (both pairs, scaled to fit). None for another kind, another
/// count of numbers, or an alignment whose second pair has no direction. The
/// web's handler reaches it through WASM with the numbers as float64 (no
/// JSON), the desktop's natively: one matrix, bit for bit, on both. Whether
/// the numbers make sense (a factor above zero, an axis with a direction) is
/// the command's to check first.
pub fn similarity(kind: &str, p: &[f64]) -> Option<Affine> {
    let at = |i: usize| Vec2::new(p[i], p[i + 1]);
    match (kind, p) {
        ("move", &[dx, dy]) => Some(translation(dx, dy)),
        ("rotate", &[cx, cy, angle]) => Some(rotation(angle, Vec2::new(cx, cy))),
        ("scale", &[cx, cy, factor]) => Some(scaling(factor, Vec2::new(cx, cy))),
        ("mirror", &[ax, ay, bx, by]) => Some(mirror(Vec2::new(ax, ay), Vec2::new(bx, by))),
        ("align", [_, _, _, _]) => align(&[at(0), at(2)], false),
        ("align", [_, _, _, _, _, _, _, _]) => align(&[at(0), at(2), at(4), at(6)], false),
        ("alignScale", [_, _, _, _, _, _, _, _]) => align(&[at(0), at(2), at(4), at(6)], true),
        _ => None,
    }
}

/// m2 ∘ m1: first m1, then m2.
pub fn compose(m2: &Affine, m1: &Affine) -> Affine {
    let [a1, b1, c1, d1, e1, f1] = *m1;
    let [a2, b2, c2, d2, e2, f2] = *m2;
    [
        a2 * a1 + c2 * b1,
        b2 * a1 + d2 * b1,
        a2 * c1 + c2 * d1,
        b2 * c1 + d2 * d1,
        a2 * e1 + c2 * f1 + e2,
        b2 * e1 + d2 * f1 + f2,
    ]
}

pub fn apply(m: &Affine, p: Vec2) -> Vec2 {
    Vec2::new(
        m[0] * p.x + m[2] * p.y + m[4],
        m[1] * p.x + m[3] * p.y + m[5],
    )
}

/// Only the linear part (direction vectors).
pub fn apply_linear(m: &Affine, v: Vec2) -> Vec2 {
    Vec2::new(m[0] * v.x + m[2] * v.y, m[1] * v.x + m[3] * v.y)
}

pub fn determinant(m: &Affine) -> f64 {
    m[0] * m[3] - m[1] * m[2]
}

/// Length scale factor of a similarity transform.
pub fn length_scale(m: &Affine) -> f64 {
    determinant(m).abs().sqrt()
}

/// Whether the transform flips orientation (a mirror).
pub fn is_reflection(m: &Affine) -> bool {
    determinant(m) < 0.0
}

pub(crate) static OPS: &[Op] = &[
    op!("translation", |dx: f64, dy: f64| translation(dx, dy)),
    op!("rotation", |angle: f64, o: Option<Vec2>| rotation(
        angle,
        o.unwrap_or(ORIGIN)
    )),
    op!("scaling", |s: f64, o: Option<Vec2>| scaling(
        s,
        o.unwrap_or(ORIGIN)
    )),
    op!("mirror", |p: Vec2, q: Vec2| mirror(p, q)),
    op!("compose", |m2: Affine, m1: Affine| compose(&m2, &m1)),
    op!("apply", |m: Affine, p: Vec2| apply(&m, p)),
    op!("applyLinear", |m: Affine, v: Vec2| apply_linear(&m, v)),
    op!("determinant", |m: Affine| determinant(&m)),
    op!("lengthScale", |m: Affine| length_scale(&m)),
    op!("isReflection", |m: Affine| is_reflection(&m)),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(m: Affine) -> [u64; 6] {
        m.map(f64::to_bits)
    }

    /// Each kind is its constructor, bit for bit (−0 included); another
    /// kind or another count of numbers is none.
    #[test]
    fn a_similarity_is_its_constructor() {
        let c = Vec2::new(486520.25, -0.0);
        assert_eq!(
            bits(similarity("move", &[12.5, -0.0]).unwrap()),
            bits(translation(12.5, -0.0))
        );
        assert_eq!(
            bits(similarity("rotate", &[c.x, c.y, 0.7]).unwrap()),
            bits(rotation(0.7, c))
        );
        assert_eq!(
            bits(similarity("scale", &[c.x, c.y, 2.5]).unwrap()),
            bits(scaling(2.5, c))
        );
        let (a, b) = (Vec2::new(1.0, 2.0), Vec2::new(4.0, 6.0));
        assert_eq!(
            bits(similarity("mirror", &[a.x, a.y, b.x, b.y]).unwrap()),
            bits(mirror(a, b))
        );
        assert_eq!(similarity("move", &[1.0]), None);
        assert_eq!(similarity("rotate", &[1.0, 2.0]), None);
        assert_eq!(similarity("shear", &[1.0, 2.0]), None);
    }

    /// An alignment is `align` of its pairs: one pair a translation, two
    /// pairs a turn (scaled with `alignScale`); a second pair without a
    /// direction, or a pair and a half, is none.
    #[test]
    fn an_alignment_is_align_of_its_pairs() {
        let (s1, d1) = (
            Vec2::new(487010.0, 4420010.0),
            Vec2::new(487040.0, 4420030.0),
        );
        let (s2, d2) = (
            Vec2::new(487020.0, 4420010.0),
            Vec2::new(487040.0, 4420050.0),
        );
        assert_eq!(
            bits(similarity("align", &[s1.x, s1.y, d1.x, d1.y]).unwrap()),
            bits(translation(30.0, 20.0))
        );
        let pairs = [s1.x, s1.y, d1.x, d1.y, s2.x, s2.y, d2.x, d2.y];
        assert_eq!(
            bits(similarity("align", &pairs).unwrap()),
            bits(align(&[s1, d1, s2, d2], false).unwrap())
        );
        let scaled = similarity("alignScale", &pairs).unwrap();
        assert_eq!(bits(scaled), bits(align(&[s1, d1, s2, d2], true).unwrap()));
        // The second source point lands on the second target: 10 m become 20 m, turned a quarter.
        let q = apply(&scaled, s2);
        assert!(
            (q.x - d2.x).abs() < 1e-8 && (q.y - d2.y).abs() < 1e-8,
            "{q:?}"
        );
        let lost = [s1.x, s1.y, d1.x, d1.y, s1.x, s1.y, d2.x, d2.y];
        assert_eq!(similarity("align", &lost), None);
        assert_eq!(similarity("alignScale", &[s1.x, s1.y, d1.x, d1.y]), None);
        assert_eq!(similarity("align", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]), None);
    }
}
