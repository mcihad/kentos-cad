//! User-toleranced parallel curves (CAD-ADV-04). Exact rational Bézier spans
//! are subdivided using a bound on the parallel's second derivative. This
//! is independent of display tessellation. Singular normals, inward folds,
//! discontinuous joins and exhausted work budgets return no geometry.

mod bezier;
mod crossings;
#[cfg(test)]
mod tests;

use crate::entity::Shape;
use crate::geom::ellipse::{closest_param, ellipse_derivative, ellipse_point, ellipse_sweep};
use crate::geom::nurbs::{self, Nurbs, OwnedNurbs};
use crate::geom::spline::catmull_rom_beziers;
use crate::jsmath::{cos, js_hypot, js_max, js_min, sin};
use crate::ops::curve_cuts::ellipse_of;
use crate::vec2::Vec2;
use bezier::Bezier;

const LIMIT: usize = 16_384;
const DEPTH: u32 = 24;
pub const SINGULAR: &str =
    "Öteleme eğride tekil normal veya içe katlanma oluşturuyor; daha küçük mesafe seçin.";
const BUDGET: &str =
    "Öteleme bu toleransta hesap sınırını aşıyor; toleransı büyütün veya eğriyi bölün.";
const JOIN: &str =
    "Öteleme eğrinin birleşiminde boşluk oluşturuyor; uçları ve teğet sürekliliğini düzeltin.";

#[derive(Debug)]
pub struct BoundedOffset {
    pub shape: Shape,
    /// Chord interpolation bound plus a conservative f64 arithmetic allowance,
    /// in metres. Not a sampling estimate or display chord tolerance.
    pub error_bound: f64,
    pub vertices: usize,
    pub crossings: usize,
}
struct Source {
    spans: Vec<Bezier>,
    origin: Vec2,
    signed: f64,
    closed: bool,
    area: bool,
    roundoff: f64,
}
fn point_distance(a: Vec2, b: Vec2) -> f64 {
    js_hypot(a.x - b.x, a.y - b.y)
}
fn side(t: Vec2, p: Vec2, q: Vec2, d: f64) -> f64 {
    if t.x * (q.y - p.y) - t.y * (q.x - p.x) > 0. {
        d
    } else {
        -d
    }
}
fn local(p: Vec2, o: Vec2) -> Vec2 {
    Vec2::new(p.x - o.x, p.y - o.y)
}

pub fn is_approximate(s: &Shape) -> bool {
    matches!(s, Shape::Spline { .. } | Shape::Nurbs { .. }) || ellipse_of(s).is_some()
}
fn source(s: &Shape, d: f64, through: Vec2) -> Result<Source, String> {
    let (spans, origin, signed, closed, area, degree, ratio) = if let Some(e) = ellipse_of(s) {
        let t = closest_param(&e, through);
        let signed = side(ellipse_derivative(&e, t), ellipse_point(&e, t), through, d);
        let sw = ellipse_sweep(&e);
        if !(e.ratio > 0.) || !e.ratio.is_finite() {
            return Err(SINGULAR.into());
        }
        let mut e0 = e;
        e0.c = Vec2::new(0., 0.);
        let count = js_max((sw / core::f64::consts::FRAC_PI_2).ceil(), 1.) as usize;
        let mut spans = Vec::new();
        for i in 0..count {
            let a = e.t0 + sw * i as f64 / count as f64;
            let b = e.t0 + sw * (i + 1) as f64 / count as f64;
            let mid = a + (b - a) * 0.5;
            let w = cos((b - a) * 0.5);
            let p = ellipse_point(&e0, a);
            let q = ellipse_point(&e0, b);
            let u = Vec2::new(
                (e.major.x * cos(mid) - e.major.y * e.ratio * sin(mid)) / w,
                (e.major.y * cos(mid) + e.major.x * e.ratio * sin(mid)) / w,
            );
            spans.push(Bezier::from_points(&[p, u, q], Some(&[1., w, 1.])));
        }
        let closed = crate::geom::ellipse::is_full_ellipse(&e);
        (spans, e.c, signed, closed, closed, 2, 2.)
    } else if let Some(n) = nurbs::of(s) {
        if n.ctrl.len() > 2048
            || n.degree > 11
            || n.ctrl.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            return Err(BUDGET.into());
        }
        let origin = n.ctrl[0];
        let (lo, hi) = n.domain();
        let max = n.weights.map_or(1., |w| w.iter().copied().fold(0., js_max));
        let min = n
            .weights
            .map_or(1., |w| w.iter().copied().fold(f64::INFINITY, js_min));
        if !max.is_finite() || !min.is_finite() || max / min > 1e6 || !(hi - lo).is_finite() {
            return Err("Eğrinin ağırlık veya düğüm aralığı sayısal güvenlik sınırını aşıyor; eğriyi düzenleyin.".into());
        }
        let mut owned = OwnedNurbs {
            degree: n.degree,
            ctrl: n.ctrl.iter().map(|p| local(*p, origin)).collect(),
            knots: n.knots.iter().map(|k| (*k - lo) / (hi - lo)).collect(),
            weights: n.weights.map(|w| w.iter().map(|x| x / max).collect()),
        };
        if !owned.view().valid() {
            return Err(SINGULAR.into());
        }
        let p = local(through, origin);
        let ds = owned.view().derivs(owned.view().closest(p), 1);
        let signed = side(ds[1], ds[0], p, d);
        let mut breaks: Vec<_> = owned
            .knots
            .iter()
            .copied()
            .filter(|u| *u >= 0. && *u <= 1.)
            .collect();
        breaks.dedup();
        if breaks.len() * n.ctrl.len() * n.degree > 200_000 {
            return Err(BUDGET.into());
        }
        for u in breaks {
            owned = owned.view().insert(u, n.degree);
        }
        let mut spans = Vec::new();
        for k in n.degree..owned.ctrl.len() {
            if owned.knots[k] >= 0.
                && owned.knots[k + 1] <= 1.
                && owned.knots[k + 1] > owned.knots[k]
            {
                let range = k - n.degree..=k;
                spans.push(Bezier::from_points(
                    &owned.ctrl[range.clone()],
                    owned.weights.as_ref().map(|w| &w[range]),
                ));
            }
        }
        let closed = matches!(s, Shape::Nurbs { closed: true, .. });
        (spans, origin, signed, closed, false, n.degree, max / min)
    } else if let Shape::Spline { pts, closed } = s {
        let Some(&origin) = pts.first() else {
            return Err(SINGULAR.into());
        };
        if pts.len() > 2048 || pts.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err(BUDGET.into());
        }
        let pts: Vec<_> = pts.iter().map(|p| local(*p, origin)).collect();
        let spans: Vec<_> = catmull_rom_beziers(&pts, *closed)
            .iter()
            .map(|b| Bezier::from_points(&b.ctrl, None))
            .collect();
        let mut nearest = (f64::INFINITY, 0.);
        for b in &spans {
            let ctrl: Vec<_> = b.0.iter().map(|p| Vec2::new(p[0], p[1])).collect();
            let n = Nurbs {
                degree: 3,
                ctrl: &ctrl,
                knots: &[0., 0., 0., 0., 1., 1., 1., 1.],
                weights: None,
            };
            let q = local(through, origin);
            let ds = n.derivs(n.closest(q), 1);
            let gap = point_distance(ds[0], q);
            if gap < nearest.0 {
                nearest = (gap, side(ds[1], ds[0], q, d));
            }
        }
        (spans, origin, nearest.1, *closed, false, 3, 1.)
    } else {
        return Err("Bu nesne yaklaşık eğri ötelemesi gerektirmiyor.".into());
    };
    if spans.is_empty() {
        return Err(SINGULAR.into());
    }
    let size = spans
        .iter()
        .flat_map(|b| b.0.iter())
        .map(|p| js_hypot(p[0] / p[2], p[1] / p[2]))
        .fold(d, js_max);
    let magnitude = js_max(js_max(js_max(origin.x.abs(), origin.y.abs()), size), 1.);
    let roundoff = 256.
        * f64::EPSILON
        * magnitude
        * ((degree + 1) as f64)
        * ((degree + 1) as f64)
        * ratio
        * ratio;
    Ok(Source {
        spans,
        origin,
        signed,
        closed,
        area,
        roundoff,
    })
}

struct Builder {
    signed: f64,
    tolerance: f64,
    roundoff: f64,
    points: Vec<Vec2>,
    bound: f64,
    visits: usize,
}
impl Builder {
    fn append(&mut self, b: &Bezier, depth: u32) -> Result<(), String> {
        self.visits += 1;
        if self.visits > LIMIT * 3 || self.points.len() >= LIMIT || depth > DEPTH {
            return Err(BUDGET.into());
        }
        if let Some(bounds) = b.bounds(self.signed) {
            if bounds.regular.1 <= 0. || (depth == DEPTH && bounds.regular.0 <= 1e-8) {
                return Err(SINGULAR.into());
            }
            let bound = bounds.second / 8. + self.roundoff;
            if bounds.regular.0 > 1e-8 && bound <= self.tolerance {
                let a = b.end(false, self.signed).ok_or(SINGULAR)?;
                let z = b.end(true, self.signed).ok_or(SINGULAR)?;
                if let Some(&prev) = self.points.last() {
                    if point_distance(prev, a) > self.roundoff {
                        return Err(JOIN.into());
                    }
                } else {
                    self.points.push(a);
                }
                self.points.push(z);
                self.bound = js_max(self.bound, bound);
                return Ok(());
            }
        }
        if (depth == DEPTH && b.bounds(self.signed).is_none())
            || b.0.windows(2).all(|w| w[0] == w[1])
        {
            return Err(SINGULAR.into());
        }
        let (left, right) = b.split();
        self.append(&left, depth + 1)?;
        self.append(&right, depth + 1)
    }
}

/// No write occurs on any error. The caller records source and result identity.
pub fn offset(s: &Shape, d: f64, through: Vec2, tolerance: f64) -> Result<BoundedOffset, String> {
    if !tolerance.is_finite()
        || tolerance <= 0.
        || !d.is_finite()
        || d <= 0.
        || !through.x.is_finite()
        || !through.y.is_finite()
    {
        return Err("Mesafe ve yaklaşım toleransı sonlu, sıfırdan büyük olmalı.".into());
    }
    let src = source(s, d, through)?;
    if !src.roundoff.is_finite() || src.roundoff >= tolerance * 0.5 {
        return Err("Tolerans bu koordinat ve ağırlıklarda sayısal güvenlik payından küçük; toleransı büyütün.".into());
    }
    let mut b = Builder {
        signed: src.signed,
        tolerance,
        roundoff: src.roundoff,
        points: Vec::new(),
        bound: 0.,
        visits: 0,
    };
    for span in &src.spans {
        b.append(span, 0)?;
    }
    if src.closed {
        if point_distance(b.points[0], b.points.last().copied().ok_or(SINGULAR)?) > src.roundoff {
            return Err(JOIN.into());
        }
        b.points.pop();
    }
    for p in &mut b.points {
        p.x += src.origin.x;
        p.y += src.origin.y;
    }
    if b.points
        .iter()
        .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Err(BUDGET.into());
    }
    let vertices = b.points.len();
    let shape = if src.area {
        Shape::Polygon {
            pts: b.points,
            bulges: None,
            holes: None,
            parts: None,
        }
    } else {
        Shape::Polyline {
            pts: b.points,
            bulges: None,
            holes: None,
            parts: None,
            closed: src.closed,
            widths: None,
        }
    };
    let crossings = crossings::count(&shape)?;
    Ok(BoundedOffset {
        crossings,
        shape,
        error_bound: b.bound,
        vertices,
    })
}

/// The analytic curve's closest point and tangent, used for through-point
/// distance and the opposite side. Display chords never choose this distance.
pub fn nearest(s: &Shape, q: Vec2) -> Option<(Vec2, Vec2)> {
    if let Some(e) = ellipse_of(s) {
        let t = closest_param(&e, q);
        return Some((ellipse_point(&e, t), ellipse_derivative(&e, t)));
    }
    if let Some(n) = nurbs::of(s) {
        if n.ctrl.len() > 2048 || n.degree > 11 {
            return None;
        }
        let origin = n.ctrl[0];
        let ctrl: Vec<_> = n.ctrl.iter().map(|p| local(*p, origin)).collect();
        let max = n.weights.map_or(1., |w| w.iter().copied().fold(0., js_max));
        let weights = n
            .weights
            .map(|w| w.iter().map(|x| x / max).collect::<Vec<_>>());
        let n = Nurbs {
            degree: n.degree,
            ctrl: &ctrl,
            knots: n.knots,
            weights: weights.as_deref(),
        };
        if !n.valid() {
            return None;
        }
        let ds = n.derivs(n.closest(local(q, origin)), 1);
        return Some((Vec2::new(ds[0].x + origin.x, ds[0].y + origin.y), ds[1]));
    }
    if let Shape::Spline { pts, closed } = s {
        let origin = *pts.first()?;
        let pts: Vec<_> = pts.iter().map(|p| local(*p, origin)).collect();
        return catmull_rom_beziers(&pts, *closed)
            .into_iter()
            .filter_map(|b| {
                let n = Nurbs {
                    degree: 3,
                    ctrl: &b.ctrl,
                    knots: &[0., 0., 0., 0., 1., 1., 1., 1.],
                    weights: None,
                };
                let ds = n.derivs(n.closest(local(q, origin)), 1);
                let p = Vec2::new(ds[0].x + origin.x, ds[0].y + origin.y);
                point_distance(p, q).is_finite().then_some((p, ds[1]))
            })
            .min_by(|a, b| point_distance(a.0, q).total_cmp(&point_distance(b.0, q)));
    }
    None
}
