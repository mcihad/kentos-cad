//! Bounded CAD curve chords for physical output, independent of screen zoom and source measurement geometry.
use crate::{
    Vec2,
    entity::Shape,
    jsmath::{TAU, cos, sin, js_hypot, js_max},
};
pub const MAX_POINTS: usize = 65_536;
fn complex() -> String {
    "plot_curve_too_complex: Eğri baskının 0,005 mm toleransında 65536 noktayı aşıyor; eğriyi bölün veya daha küçük ölçekli bir görünüm seçin.".into()
}
fn count(sweep: f64, second: f64, tol: f64) -> Result<usize, String> {
    if !tol.is_finite() || tol <= 0. || !second.is_finite() || second < 0. || !sweep.is_finite() {
        return Err(complex());
    }
    let n = js_max((sweep.abs() * (second / (8. * tol)).sqrt()).ceil(), 1.);
    if n > MAX_POINTS as f64 {
        Err(complex())
    } else {
        Ok(n as usize)
    }
}
/// A conic's parameter chord bound is max(|p''|) h²/8; no device-dependent segment ceiling.
#[allow(clippy::too_many_arguments)] // complete analytic conic parameters and its physical output tolerance
pub fn conic(
    c: Vec2,
    major: Vec2,
    ratio: f64,
    start: f64,
    sweep: f64,
    closed: bool,
    tol: f64,
) -> Result<Vec<Vec2>, String> {
    let radius = js_hypot(major.x, major.y) * js_max(ratio.abs(), 1.);
    let n = count(sweep, radius, tol)?.max(if closed { 8 } else { 1 });
    let length = n + usize::from(!closed);
    if length > MAX_POINTS {
        return Err(complex());
    }
    let mut out = Vec::with_capacity(length);
    for k in 0..length {
        let t = start + sweep * k as f64 / n as f64;
        let (s, co) = (sin(t), cos(t));
        out.push(Vec2::new(
            c.x + major.x * co - major.y * ratio * s,
            c.y + major.y * co + major.x * ratio * s,
        ));
    }
    Ok(out)
}
/// A bulged ring/run as chords. Straight vertices stay exact.
pub fn bulges(
    pts: &[Vec2],
    bs: Option<&[f64]>,
    closed: bool,
    tol: f64,
) -> Result<Vec<Vec2>, String> {
    if pts.len() > MAX_POINTS {
        return Err(complex());
    }
    let mut out = Vec::new();
    let edges = if closed {
        pts.len()
    } else {
        pts.len().saturating_sub(1)
    };
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let mut points = if let Some(arc) =
            crate::geom::bulge::bulge_arc(a, b, crate::geom::bulge::bulge_at(bs, i))
        {
            conic(
                arc.c,
                Vec2::new(arc.r, 0.),
                1.,
                arc.a0,
                arc.sweep,
                false,
                tol,
            )?
        } else {
            vec![a, b]
        };
        points[0] = a;
        let last = points.len() - 1;
        points[last] = b;
        if out.len() + points.len() - 1 > MAX_POINTS {
            return Err(complex());
        }
        out.extend(points.into_iter().take(last));
    }
    if !closed && let Some(&last) = pts.last() {
        out.push(last);
    }
    Ok(out)
}
/// None: straight or an architectural object, whose established drawing record is used.
pub fn outline(shape: &Shape, tol: f64) -> Result<Option<(Vec<Vec2>, bool)>, String> {
    let result = match shape {
        Shape::Circle { c, r } => (conic(*c, Vec2::new(*r, 0.), 1., 0., TAU, true, tol)?, true),
        Shape::Arc { c, r, a0, a1 } => {
            let sweep = crate::geom::arc::sweep(*a0, *a1);
            (
                conic(*c, Vec2::new(*r, 0.), 1., *a0, sweep, false, tol)?,
                false,
            )
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = crate::entity::ellipse_geom(*c, *major, *ratio, *t0, *t1);
            let full = crate::geom::ellipse::is_full_ellipse(&g);
            (
                conic(
                    *c,
                    *major,
                    *ratio,
                    *t0,
                    crate::geom::ellipse::ellipse_sweep(&g),
                    full,
                    tol,
                )?,
                full,
            )
        }
        Shape::Spline { pts, closed } => {
            let mut out = Vec::new();
            for span in crate::geom::spline::catmull_rom_beziers(pts, *closed) {
                let p = span.ctrl;
                let second = |a: Vec2, b: Vec2, c: Vec2| {
                    6. * js_hypot(c.x - 2. * b.x + a.x, c.y - 2. * b.y + a.y)
                };
                let n = count(
                    1.,
                    js_max(second(p[0], p[1], p[2]), second(p[1], p[2], p[3])),
                    tol,
                )?;
                if out.len() + n + 1 > MAX_POINTS {
                    return Err(complex());
                }
                for k in 0..n {
                    let t = k as f64 / n as f64;
                    let u = 1. - t;
                    out.push(Vec2::new(
                        u * u * u * p[0].x
                            + 3. * u * u * t * p[1].x
                            + 3. * u * t * t * p[2].x
                            + t * t * t * p[3].x,
                        u * u * u * p[0].y
                            + 3. * u * u * t * p[1].y
                            + 3. * u * t * t * p[2].y
                            + t * t * t * p[3].y,
                    ));
                }
                out.push(p[3]);
            }
            (out, *closed)
        }
        Shape::Nurbs { closed, .. } => {
            let points = crate::geom::nurbs::of(shape)
                .and_then(|n| n.tessellate_bounded(tol, MAX_POINTS))
                .ok_or_else(complex)?;
            (points, *closed)
        }
        Shape::Polyline {
            pts,
            bulges: Some(bs),
            closed,
            widths,
            parts: None,
            ..
        } if widths
            .as_ref()
            .is_none_or(|ws| ws.iter().all(|[a, b]| *a == 0. && *b == 0.)) =>
        {
            (bulges(pts, Some(bs), *closed, tol)?, *closed)
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twenty_conics_respect_the_paper_chord_bound_at_different_model_scales() {
        for n in 0..20 {
            let radius = [0.01, 0.1, 1., 10., 100.][n as usize % 5];
            let scale = [1., 10., 50., 1000.][n as usize / 5];
            let tolerance = scale * 0.005 / 1000.;
            let angle = f64::from(n) * 0.11;
            let major = Vec2::new(radius * cos(angle), radius * sin(angle));
            let center = Vec2::new(500000.125, 4500000.25);
            let points = conic(center, major, 0.4, 0., TAU, true, tolerance).unwrap();
            assert!(points.len() <= MAX_POINTS);
            for k in 0..points.len() {
                let a = points[k];
                let b = points[(k + 1) % points.len()];
                let t = TAU * (k as f64 + 0.5) / points.len() as f64;
                let expected = Vec2::new(
                    center.x + major.x * cos(t) - major.y * 0.4 * sin(t),
                    center.y + major.y * cos(t) + major.x * 0.4 * sin(t),
                );
                assert!(
                    js_hypot((a.x + b.x) / 2. - expected.x, (a.y + b.y) / 2. - expected.y)
                        <= tolerance + 1e-9
                );
            }
        }
    }
    #[test]
    fn impossible_output_precision_is_refused_before_allocating_a_curve() {
        assert!(
            conic(
                Vec2::new(0., 0.),
                Vec2::new(1e12, 0.),
                1.,
                0.,
                TAU,
                true,
                1e-9
            )
            .unwrap_err()
            .contains("plot_curve_too_complex")
        );
        for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(
                conic(
                    Vec2::new(0., 0.),
                    Vec2::new(1., 0.),
                    1.,
                    0.,
                    TAU,
                    true,
                    tolerance
                )
                .is_err()
            );
        }
    }
}
