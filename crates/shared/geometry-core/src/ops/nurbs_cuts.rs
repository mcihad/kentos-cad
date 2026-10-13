//! Buda, Kır and Yönü çevir on a NURBS curve (docs/adr/0259 §2): the cuts
//! are found on the curve's chords, moved onto the curve and the boundary
//! in turn until they agree, and the curve is split there by knot insertion:
//! every piece is the same curve, its own control points and knots, never a
//! polyline. A closed curve's remaining piece runs from the cut after the
//! picked part round to the one before it, its two halves joined where the
//! curve closed.

use crate::entity::{Entity, Shape};
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges};
use crate::geom::nurbs::{Nurbs, OwnedNurbs, of, with_curve};
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::ops::curve_cuts::Cut;
use crate::vec2::Vec2;

/// The drawing's chords a crossing is first found on.
const CHORDS: f64 = 1e-3;

/// The parameters where `boundaries` cross the curve, in order, each found on a chord and refined onto
/// both the curve and the boundary edge.
pub fn cuts(c: &Nurbs, boundaries: &[Edge]) -> Vec<f64> {
    let pts = c.tessellate(CHORDS);
    let mut out = Vec::new();
    for w in pts.windows(2) {
        let chord = Edge::Seg { a: w[0], b: w[1] };
        for b in boundaries {
            for h in intersect_edges(&chord, b) {
                let mut q = h.p;
                for _ in 0..40 {
                    let on = c.point(c.closest(q));
                    let next = closest_on_edge(b, on).p;
                    let moved = js_hypot(next.x - q.x, next.y - q.y);
                    q = next;
                    if moved <= 1e-13 {
                        break;
                    }
                }
                out.push(c.closest(q));
            }
        }
    }
    out.sort_by(f64::total_cmp);
    let span = {
        let (lo, hi) = c.domain();
        hi - lo
    };
    out.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * span);
    out
}

/// Whether the curve's ends meet (a closed curve, whatever its flag says).
fn ends_meet(c: &Nurbs) -> bool {
    let (lo, hi) = c.domain();
    let (a, b) = (c.point(lo), c.point(hi));
    js_hypot(a.x - b.x, a.y - b.y) <= 1e-9 * (1.0 + js_hypot(a.x, a.y))
}

fn piece(like: &Entity, c: OwnedNurbs, closed: bool) -> Entity {
    like.with(with_curve(&like.shape, c, closed))
}

/// The curve between `a` and `b` (parameters), running past the end round to the start when `b < a`.
fn around(c: &Nurbs, a: f64, b: f64) -> Option<OwnedNurbs> {
    if a < b {
        return c.sub(a, b);
    }
    let (lo, hi) = c.domain();
    match (c.sub(a, hi), c.sub(lo, b)) {
        (Some(x), Some(y)) => join(&x, &y),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// Two clamped curves of one degree where the first ends where the second starts, as one curve (the
/// join a knot of multiplicity `degree`; the second's weights scaled to meet the first's).
pub fn join(a: &OwnedNurbs, b: &OwnedNurbs) -> Option<OwnedNurbs> {
    let p = a.degree;
    if b.degree != p {
        return None;
    }
    let end = *a.knots.last()?;
    let shift = end - b.knots[0];
    let wa = a.weights.as_ref().map_or(1.0, |w| w[w.len() - 1]);
    let wb = b.weights.as_ref().map_or(1.0, |w| w[0]);
    let scale = wa / wb;
    let mut knots = a.knots[..a.knots.len() - 1].to_vec();
    knots.extend(b.knots[p + 1..].iter().map(|k| k + shift));
    let mut ctrl = a.ctrl.clone();
    ctrl.extend_from_slice(&b.ctrl[1..]);
    let weights = if a.weights.is_some() || b.weights.is_some() {
        let mut w = a.weights.clone().unwrap_or_else(|| vec![1.0; a.ctrl.len()]);
        let bw = b.weights.clone().unwrap_or_else(|| vec![1.0; b.ctrl.len()]);
        w.extend(bw[1..].iter().map(|x| x * scale));
        Some(w)
    } else {
        None
    };
    let out = OwnedNurbs {
        degree: p,
        ctrl,
        knots,
        weights,
    };
    out.view().valid().then_some(out)
}

/// Buda: the part between the cuts that bracket `pick` taken away.
pub fn trim(target: &Entity, pick: Vec2, boundaries: &[Edge]) -> Cut {
    let Some(c) = of(&target.shape) else {
        return Cut::Error("Eğrinin sayıları bir eğri tanımlamıyor.".into());
    };
    let (lo, hi) = c.domain();
    let span = hi - lo;
    let eps = 1e-9 * span;
    let up = c.closest(pick);
    let all = cuts(&c, boundaries);
    let closed = ends_meet(&c);
    if !closed {
        let before = all
            .iter()
            .copied()
            .filter(|u| *u < up - eps)
            .fold(lo, js_max);
        let after = all
            .iter()
            .copied()
            .filter(|u| *u > up + eps)
            .fold(hi, js_min);
        if before <= lo + eps && after >= hi - eps {
            return Cut::Error("Tıklanan kısmı kesen bir kenar yok.".into());
        }
        let mut pieces = Vec::new();
        if before > lo + eps
            && let Some(p) = c.sub(lo, before)
        {
            pieces.push(piece(target, p, false));
        }
        if after < hi - eps
            && let Some(p) = c.sub(after, hi)
        {
            pieces.push(piece(target, p, false));
        }
        return Cut::Pieces(pieces);
    }
    let inside: Vec<f64> = all
        .into_iter()
        .filter(|u| *u > lo + eps && *u < hi - eps)
        .collect();
    if inside.len() < 2 {
        return Cut::Error("Kapalı eğriyi budamak için en az iki kesişim gerekir.".into());
    }
    let before = inside
        .iter()
        .copied()
        .rfind(|u| *u < up)
        .unwrap_or(inside[inside.len() - 1]);
    let after = inside
        .iter()
        .copied()
        .find(|u| *u > up)
        .unwrap_or(inside[0]);
    match around(&c, after, before) {
        Some(p) => Cut::Pieces(vec![piece(target, p, false)]),
        None => Cut::Error("Eğri bu noktalardan budanamadı.".into()),
    }
}

/// Kır: the part between the points nearest `p1` and `p2` taken away; the same point splits the curve
/// in two (an open one) or opens it there (a closed one).
pub fn break_at(e: &Entity, p1: Vec2, p2: Vec2) -> Cut {
    let Some(c) = of(&e.shape) else {
        return Cut::Error("Eğrinin sayıları bir eğri tanımlamıyor.".into());
    };
    let (lo, hi) = c.domain();
    let eps = 1e-9 * (hi - lo);
    let (u1, u2) = (c.closest(p1), c.closest(p2));
    let closed = ends_meet(&c);
    let same = (u1 - u2).abs() <= eps;
    if !closed {
        if same {
            if u1 <= lo + eps || u1 >= hi - eps {
                return Cut::Error(
                    "Eğri ucundan kırılamaz; iç kısmında bir nokta gösterin.".into(),
                );
            }
            let Some((a, b)) = c.split(u1) else {
                return Cut::Error("Eğri bu noktada kırılamadı.".into());
            };
            return Cut::Pieces(vec![piece(e, a, false), piece(e, b, false)]);
        }
        let (a, b) = (js_min(u1, u2), js_max(u1, u2));
        let mut pieces = Vec::new();
        if a > lo + eps
            && let Some(p) = c.sub(lo, a)
        {
            pieces.push(piece(e, p, false));
        }
        if b < hi - eps
            && let Some(p) = c.sub(b, hi)
        {
            pieces.push(piece(e, p, false));
        }
        return if pieces.is_empty() {
            Cut::Error("İki nokta eğrinin tamamını kapsıyor; silmek için Sil kullanın.".into())
        } else {
            Cut::Pieces(pieces)
        };
    }
    // Closed: opened at the point, or the part from p1 on to p2 taken away.
    let (from, to) = if same { (u1, u1) } else { (u2, u1) };
    let opened = if same {
        around(&c, from, if from > lo + eps { from } else { hi })
    } else {
        around(&c, from, to)
    };
    match opened {
        Some(p) => Cut::Pieces(vec![piece(e, p, false)]),
        None => Cut::Error("Eğri bu noktalardan kırılamadı.".into()),
    }
}

/// Yönü çevir: the same curve run the other way, its fit points and tangents turned too.
pub fn reversed(s: &Shape) -> Option<Shape> {
    let c = of(s)?;
    let r = c.reversed();
    let Shape::Nurbs {
        closed,
        periodic,
        fit,
        fit_tolerance,
        tangents,
        ..
    } = s
    else {
        return None;
    };
    Some(Shape::Nurbs {
        degree: r.degree,
        ctrl: r.ctrl,
        knots: r.knots,
        weights: r.weights,
        closed: *closed,
        periodic: *periodic,
        fit: fit.as_ref().map(|f| f.iter().rev().copied().collect()),
        fit_tolerance: *fit_tolerance,
        tangents: tangents.map(|[a, b]| {
            let neg = |t: Option<Vec2>| t.map(|t| Vec2::new(-t.x, -t.y));
            [neg(b), neg(a)]
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s_curve() -> Entity {
        Entity::new(Shape::Nurbs {
            degree: 3,
            ctrl: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(2.0, 4.0),
                Vec2::new(6.0, -4.0),
                Vec2::new(8.0, 0.0),
            ],
            knots: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            weights: None,
            closed: false,
            periodic: false,
            fit: None,
            fit_tolerance: None,
            tangents: None,
        })
    }

    #[test]
    fn a_trim_keeps_the_curve_and_cuts_on_the_boundary() {
        let e = s_curve();
        let x4 = Edge::Seg {
            a: Vec2::new(4.0, -10.0),
            b: Vec2::new(4.0, 10.0),
        };
        let Cut::Pieces(p) = trim(&e, Vec2::new(7.0, -1.0), &[x4]) else {
            panic!("pieces")
        };
        assert_eq!(p.len(), 1);
        let c = of(&p[0].shape).expect("a curve");
        let (lo, hi) = c.domain();
        let end = c.point(hi);
        assert!((end.x - 4.0).abs() < 1e-12, "{end:?}");
        let start = c.point(lo);
        assert_eq!((start.x, start.y), (0.0, 0.0));
        // The piece is the original curve: its points are on it.
        let whole = of(&e.shape).unwrap();
        for i in 0..=10 {
            let q = c.point(lo + (hi - lo) * i as f64 / 10.0);
            let on = whole.point(whole.closest(q));
            assert!(js_hypot(on.x - q.x, on.y - q.y) < 1e-10);
        }
    }

    #[test]
    fn a_break_splits_and_a_reverse_turns() {
        let e = s_curve();
        let Cut::Pieces(p) = break_at(&e, Vec2::new(4.0, 0.0), Vec2::new(4.0, 0.0)) else {
            panic!("pieces")
        };
        assert_eq!(p.len(), 2);
        let r = reversed(&e.shape).unwrap();
        let c = of(&r).unwrap();
        let (lo, _) = c.domain();
        assert_eq!(c.point(lo), Vec2::new(8.0, 0.0));
    }
}
