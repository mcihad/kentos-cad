//! Önden and geriden kestirme: a new point from angles measured at two known
//! points (forward intersection), or at the new point towards three known
//! ones (backward intersection, resection).

use super::{Unit, bearing, distance, distinct, finite, from_bearing, positive};
use crate::api::Op;
use crate::jsmath::{PI, cos, js_hypot, sin};
use crate::op;
use crate::vec2::Vec2;

/// Önden kestirme. At A the angle `alpha` runs clockwise from B to the new
/// point P, at B the angle `beta` clockwise from P to A: P lies right of the
/// line A → B. By the sine rule, AP = AB · sin β / sin(α + β).
pub fn forward_intersection(
    unit: Unit,
    a: Vec2,
    b: Vec2,
    alpha: f64,
    beta: f64,
) -> Result<Vec2, String> {
    distinct(a, b, "A ile B")?;
    let al = unit.rad(finite(alpha, "A'daki açı")?);
    let be = unit.rad(finite(beta, "B'deki açı")?);
    if !(al > 0.0 && be > 0.0 && al + be < PI) {
        return Err(
            "Açılar üçgen kurmuyor: ikisi de sıfırdan büyük, toplamları yarım turdan küçük olmalı."
                .into(),
        );
    }
    let s = distance(a, b) * sin(be) / sin(al + be);
    Ok(from_bearing(a, positive(bearing(a, b) + al), s))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resection {
    pub p: Vec2,
    /// How far the two circles' centres lie apart for their size: near 0 the
    /// point is near the circle through A, B and C (tehlike dairesi) and the
    /// solution is weak.
    pub strength: f64,
}

crate::json_struct!(out Resection { p, strength });

/// Geriden kestirme. At the new point P, `alpha` runs clockwise from A to B
/// and `beta` clockwise from B to C. P lies on the circle through A and B
/// under which AB is seen at α, and on the one through B and C seen at β;
/// the circles meet in B and in P, so P is B mirrored in the line through
/// their centres.
pub fn resection(
    unit: Unit,
    a: Vec2,
    b: Vec2,
    c: Vec2,
    alpha: f64,
    beta: f64,
) -> Result<Resection, String> {
    distinct(a, b, "A ile B")?;
    distinct(b, c, "B ile C")?;
    let al = unit.rad(finite(alpha, "A ile B arasındaki açı")?);
    let be = unit.rad(finite(beta, "B ile C arasındaki açı")?);
    // Centre of the circle through u, v seen clockwise from u to v at the angle w from its points
    // right of u → v: the chord's middle moved along the right normal by half the chord times cot w.
    let centre = |u: Vec2, v: Vec2, w: f64| -> Option<(Vec2, f64)> {
        let s = sin(w);
        if !(s.abs() > 1e-12) {
            return None;
        }
        let (dx, dy) = (v.x - u.x, v.y - u.y);
        let k = cos(w) / s / 2.0;
        // Right normal of (dx, dy), scaled by the chord: (dy, −dx).
        let m = Vec2::new((u.x + v.x) / 2.0 + dy * k, (u.y + v.y) / 2.0 - dx * k);
        Some((m, js_hypot(m.x - u.x, m.y - u.y)))
    };
    let (Some((m1, r1)), Some((m2, r2))) = (centre(a, b, al), centre(b, c, be)) else {
        return Err(
            "Açılar sıfır ya da yarım tur olamaz: noktalar aynı doğrultuda görünüyor.".into(),
        );
    };
    let (ux, uy) = (m2.x - m1.x, m2.y - m1.y);
    let gap = js_hypot(ux, uy);
    let strength = gap / (r1 + r2);
    if !(strength > 1e-9) {
        return Err("Nokta tehlike dairesinde: A, B, C ve durulan nokta aynı çember üzerinde, çözüm belirsiz. Başka bir bilinen nokta seçin.".into());
    }
    // B mirrored in the line through the centres.
    let t = ((b.x - m1.x) * ux + (b.y - m1.y) * uy) / (gap * gap);
    let f = Vec2::new(m1.x + t * ux, m1.y + t * uy);
    let p = Vec2::new(2.0 * f.x - b.x, 2.0 * f.y - b.y);
    if !(distance(p, b) > 1e-9 * (r1 + r2)) {
        return Err("Çözüm B noktasına düşüyor: açılar bu üç noktayla uyuşmuyor.".into());
    }
    Ok(Resection { p, strength })
}

pub(crate) const FORWARD_OP: Op =
    op!("surveyForward", |unit: String,
                          a: Vec2,
                          b: Vec2,
                          alpha: f64,
                          beta: f64| {
        Unit::parse(&unit).and_then(|u| forward_intersection(u, a, b, alpha, beta))
    });
pub(crate) const RESECTION_OP: Op =
    op!("surveyResection", |unit: String,
                            a: Vec2,
                            b: Vec2,
                            c: Vec2,
                            alpha: f64,
                            beta: f64| {
        Unit::parse(&unit).and_then(|u| resection(u, a, b, c, alpha, beta))
    });
