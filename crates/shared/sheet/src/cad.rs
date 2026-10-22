//! Analytic paper geometry. Frames are transforms/selection boxes; they never replace the source curve (ADR 0593).
use crate::{
    Result, SheetError,
    units::{RectUm, SizeUm},
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub type Point = [f64; 2];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperCadPolyline"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Polyline {
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
    pub points: Vec<Point>,
    /// tan(sweep/4), clockwise positive in paper coordinates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
    pub bulges: Vec<f64>,
    #[serde(default)]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
    pub widths: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperCadGeometry"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Geometry {
    Polyline {
        paths: Vec<Polyline>,
    },
    Circle {
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        center: Point,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        radius: f64,
    },
    Arc {
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        center: Point,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        radius: f64,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="rad")))]
        start: f64,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="rad")))]
        sweep: f64,
    },
    Ellipse {
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        center: Point,
        /// Major axis vector, from the centre.
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        major: Point,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
        ratio: f64,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="rad")))]
        start: f64,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="rad")))]
        sweep: f64,
    },
    /// Three (quadratic) or four (cubic) control points, retained exactly.
    Bezier {
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        points: Vec<Point>,
        #[serde(default)]
        closed: bool,
    },
    Nurbs {
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="count")))]
        degree: u8,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        control: Vec<Point>,
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
        knots: Vec<f64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
        weights: Vec<f64>,
        #[serde(default)]
        closed: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        periodic: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        fit: Vec<Point>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
        tangents: Option<[Option<Point>; 2]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-um")))]
        #[serde(rename = "fitTolerance")]
        fit_tolerance: Option<f64>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperCadShape"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadShape {
    /// Original selection box. Geometry is local paper micrometres, before the item's affine resize/rotation.
    pub reference: SizeUm,
    pub geometry: Geometry,
}

/// Provenance of a copy, never a second live owner of model geometry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperCadOrigin"))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Origin {
    pub layout: String,
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="index")))]
    pub record: u32,
    pub entity_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
}

fn bad(message: &str) -> SheetError {
    SheetError::new("bad_curve", message)
}
fn point(p: &Point) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() <= 1e12)
}
fn positive(r: f64) -> bool {
    r.is_finite() && r > 0. && r <= 1e12
}
fn angle(start: f64, sweep: f64) -> bool {
    start.is_finite()
        && sweep.is_finite()
        && sweep != 0.
        && sweep.abs() <= core::f64::consts::TAU + 1e-10
}

pub fn validate(cad: &CadShape) -> Result<()> {
    if cad.reference.width <= 0 || cad.reference.height <= 0 {
        return Err(bad("Eğrinin başvuru boyutları sıfırdan büyük olmalı."));
    }
    let valid = match &cad.geometry {
        Geometry::Circle { center, radius } => point(center) && positive(*radius),
        Geometry::Arc {
            center,
            radius,
            start,
            sweep,
        } => point(center) && positive(*radius) && angle(*start, *sweep),
        Geometry::Ellipse {
            center,
            major,
            ratio,
            start,
            sweep,
        } => {
            point(center)
                && point(major)
                && positive(major[0].hypot(major[1]))
                && ratio.is_finite()
                && *ratio > 0.
                && *ratio <= 1.
                && angle(*start, *sweep)
        }
        Geometry::Bezier { points, .. } => {
            (points.len() == 3 || points.len() == 4) && points.iter().all(point)
        }
        Geometry::Polyline { paths } => {
            !paths.is_empty()
                && paths.len() <= 4096
                && paths.iter().map(|p| p.points.len()).sum::<usize>() <= 65536
                && paths.iter().all(|p| {
                    p.points.len() >= 2
                        && p.points.iter().all(point)
                        && (p.bulges.is_empty() || p.bulges.len() == p.points.len())
                        && p.bulges.iter().all(|b| b.is_finite() && b.abs() <= 1e6)
                        && (p.widths.is_empty() || p.widths.len() == p.points.len())
                        && p.widths
                            .iter()
                            .flatten()
                            .all(|w| w.is_finite() && *w >= 0. && *w <= 1e12)
                })
        }
        Geometry::Nurbs {
            degree,
            control,
            knots,
            weights,
            fit,
            tangents,
            fit_tolerance,
            ..
        } => {
            if *degree > 11 || control.len() > 100000 || fit.len() > 100000 {
                return Err(bad("NURBS kaynak sınırı aşıldı."));
            }
            let pts: Vec<_> = control
                .iter()
                .map(|p| kentos_geometry_core::Vec2::new(p[0], p[1]))
                .collect();
            fit.iter().all(point)
                && tangents.iter().flatten().flatten().all(point)
                && fit_tolerance.is_none_or(|t| t.is_finite() && t >= 0.)
                && control.iter().all(point)
                && kentos_geometry_core::geom::nurbs::Nurbs {
                    degree: *degree as usize,
                    ctrl: &pts,
                    knots,
                    weights: (!weights.is_empty()).then_some(weights.as_slice()),
                }
                .valid()
        }
    };
    if !valid {
        return Err(bad(
            "Eğri noktaları, yarıçapı, açısı veya NURBS tanımı geçersiz.",
        ));
    }
    Ok(())
}

/// Local curve coordinates to the unrotated paper frame.
pub fn to_paper(cad: &CadShape, frame: &RectUm, p: Point) -> Point {
    [
        f64::from(frame.left) + p[0] * f64::from(frame.width) / f64::from(cad.reference.width),
        f64::from(frame.top) + p[1] * f64::from(frame.height) / f64::from(cad.reference.height),
    ]
}

/// Render-only path steps, before the item's rotation. Source geometry is never overwritten by these.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Move(Point),
    Line(Point),
    Cubic([Point; 3]),
    Close,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub steps: Vec<Step>,
    pub band: bool,
}
const BUDGET: usize = 65_536;
const TOLERANCE: f64 = 0.25; // paper micrometres, independent of zoom/dpi
fn complex() -> SheetError {
    SheetError::new(
        "curve_too_complex",
        "Eğri bu çerçevede 0,25 µm çizim toleransı ve 65536 adım sınırını aşıyor; eğriyi bölün veya çerçeveyi küçültün.",
    )
}

#[allow(clippy::too_many_arguments)] // analytic conic parameters plus output/transform state
fn arc_steps(
    out: &mut Vec<Step>,
    center: Point,
    major: Point,
    ratio: f64,
    start: f64,
    sweep: f64,
    gain: f64,
    connect: bool,
) -> Result<()> {
    let radius = major[0].hypot(major[1]);
    // Conservative sixth-order bound for the standard tangent-matched cubic arc. The retained model is analytic.
    let max_angle = libm::pow(0.025 * 50000. / (radius * gain).max(1.), 1. / 6.)
        .min(core::f64::consts::FRAC_PI_4);
    let n = libm::ceil(sweep.abs() / max_angle).max(1.) as usize;
    if n > BUDGET || out.len() + n > BUDGET {
        return Err(complex());
    }
    let start = start.rem_euclid(core::f64::consts::TAU);
    let dt = sweep / n as f64;
    let at = |t: f64| {
        let (s, c) = (libm::sin(t), libm::cos(t));
        [
            center[0] + major[0] * c - major[1] * ratio * s,
            center[1] + major[1] * c + major[0] * ratio * s,
        ]
    };
    let d = |t: f64| {
        let (s, c) = (libm::sin(t), libm::cos(t));
        [
            -major[0] * s - major[1] * ratio * c,
            -major[1] * s + major[0] * ratio * c,
        ]
    };
    if !connect {
        out.push(Step::Move(at(start)));
    }
    let k = 4. / 3. * libm::tan(dt / 4.);
    for i in 0..n {
        let a = start + i as f64 * dt;
        let b = a + dt;
        let p = at(a);
        let q = at(b);
        let da = d(a);
        let db = d(b);
        out.push(Step::Cubic([
            [p[0] + k * da[0], p[1] + k * da[1]],
            [q[0] - k * db[0], q[1] - k * db[1]],
            q,
        ]));
    }
    Ok(())
}

pub fn paths(cad: &CadShape, frame: &RectUm) -> Result<Vec<Path>> {
    validate(cad)?;
    let gain = (f64::from(frame.width) / f64::from(cad.reference.width))
        .max(f64::from(frame.height) / f64::from(cad.reference.height));
    let mut paths = Vec::new();
    let mut steps = Vec::new();
    match &cad.geometry {
        Geometry::Circle { center, radius } => {
            arc_steps(
                &mut steps,
                *center,
                [*radius, 0.],
                1.,
                0.,
                core::f64::consts::TAU,
                gain,
                false,
            )?;
            steps.push(Step::Close);
        }
        Geometry::Arc {
            center,
            radius,
            start,
            sweep,
        } => arc_steps(
            &mut steps,
            *center,
            [*radius, 0.],
            1.,
            *start,
            *sweep,
            gain,
            false,
        )?,
        Geometry::Ellipse {
            center,
            major,
            ratio,
            start,
            sweep,
        } => {
            arc_steps(
                &mut steps, *center, *major, *ratio, *start, *sweep, gain, false,
            )?;
            if sweep.abs() >= core::f64::consts::TAU - 1e-10 {
                steps.push(Step::Close);
            }
        }
        Geometry::Bezier { points, closed } => {
            steps.push(Step::Move(points[0]));
            steps.push(Step::Cubic(bezier_controls(points)));
            if *closed {
                steps.push(Step::Close);
            }
        }
        Geometry::Nurbs {
            degree,
            control,
            knots,
            weights,
            closed,
            ..
        } => {
            if ((*degree == 3 && control.len() == 4) || (*degree == 2 && control.len() == 3))
                && (weights.is_empty() || weights.iter().all(|w| *w == weights[0]))
                && knots[..=usize::from(*degree)]
                    .iter()
                    .all(|k| *k == knots[0])
                && knots[control.len()..]
                    .iter()
                    .all(|k| *k == knots[control.len()])
            {
                steps.push(Step::Move(control[0]));
                steps.push(Step::Cubic(bezier_controls(control)));
            } else {
                let ctrl: Vec<_> = control
                    .iter()
                    .map(|p| kentos_geometry_core::Vec2::new(p[0], p[1]))
                    .collect();
                let points = kentos_geometry_core::geom::nurbs::Nurbs {
                    degree: *degree as usize,
                    ctrl: &ctrl,
                    knots,
                    weights: (!weights.is_empty()).then_some(weights.as_slice()),
                }
                .tessellate_bounded(TOLERANCE / gain.max(1e-12), BUDGET)
                .ok_or_else(complex)?;
                for (i, p) in points.iter().enumerate() {
                    steps.push(if i == 0 {
                        Step::Move([p.x, p.y])
                    } else {
                        Step::Line([p.x, p.y])
                    });
                }
            }
            if *closed {
                steps.push(Step::Close);
            }
        }
        Geometry::Polyline { paths: lines } => {
            for p in lines {
                if p.widths.iter().flatten().any(|w| *w > 0.) {
                    let estimate = p.points.len().saturating_mul(7)
                        + p.bulges
                            .iter()
                            .map(|b| {
                                (720. * libm::atan(*b).abs() / core::f64::consts::TAU).ceil()
                                    as usize
                            })
                            .sum::<usize>();
                    if estimate > BUDGET {
                        return Err(complex());
                    }
                    let pts: Vec<_> = p
                        .points
                        .iter()
                        .map(|p| kentos_geometry_core::Vec2::new(p[0], p[1]))
                        .collect();
                    let band = kentos_geometry_core::geom::band::band(
                        &pts,
                        (!p.bulges.is_empty()).then_some(p.bulges.as_slice()),
                        p.closed,
                        &p.widths,
                    );
                    for ring in band.rings {
                        let mut s = Vec::new();
                        for (i, p) in ring.iter().enumerate() {
                            s.push(if i == 0 {
                                Step::Move([p.x, p.y])
                            } else {
                                Step::Line([p.x, p.y])
                            });
                        }
                        s.push(Step::Close);
                        paths.push(Path {
                            steps: s,
                            band: true,
                        });
                    }
                    for ring in band.thin {
                        let s = ring
                            .iter()
                            .enumerate()
                            .map(|(i, p)| {
                                if i == 0 {
                                    Step::Move([p.x, p.y])
                                } else {
                                    Step::Line([p.x, p.y])
                                }
                            })
                            .collect();
                        paths.push(Path {
                            steps: s,
                            band: false,
                        });
                    }
                    continue;
                }
                steps.push(Step::Move(p.points[0]));
                let n = if p.closed {
                    p.points.len()
                } else {
                    p.points.len() - 1
                };
                for i in 0..n {
                    let a = p.points[i];
                    let b = p.points[(i + 1) % p.points.len()];
                    let bulge = p.bulges.get(i).copied().unwrap_or(0.);
                    if let Some(arc) = kentos_geometry_core::geom::bulge::bulge_arc(
                        kentos_geometry_core::Vec2::new(a[0], a[1]),
                        kentos_geometry_core::Vec2::new(b[0], b[1]),
                        bulge,
                    ) {
                        arc_steps(
                            &mut steps,
                            [arc.c.x, arc.c.y],
                            [arc.r, 0.],
                            1.,
                            arc.a0,
                            arc.sweep,
                            gain,
                            true,
                        )?;
                    } else {
                        steps.push(Step::Line(b));
                    }
                }
                if p.closed {
                    steps.push(Step::Close);
                }
                if steps.len() > BUDGET {
                    return Err(complex());
                }
            }
        }
    }
    if !steps.is_empty() {
        paths.push(Path { steps, band: false });
    }
    if paths.iter().map(|p| p.steps.len()).sum::<usize>() > BUDGET {
        return Err(complex());
    }
    for path in &mut paths {
        for s in &mut path.steps {
            match s {
                Step::Move(p) | Step::Line(p) => *p = to_paper(cad, frame, *p),
                Step::Cubic(points) => {
                    for p in points {
                        *p = to_paper(cad, frame, *p);
                    }
                }
                Step::Close => {}
            }
        }
    }
    if paths.iter().flat_map(|p| &p.steps).any(|s| match s {
        Step::Move(p) | Step::Line(p) => {
            p.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000_000.)
        }
        Step::Cubic(ps) => ps
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000_000.),
        Step::Close => false,
    }) {
        return Err(bad("Eğri dönüştürülebilen kâğıt koordinatlarının dışında."));
    }
    Ok(paths)
}
fn bezier_controls(points: &[Point]) -> [Point; 3] {
    if points.len() == 4 {
        [points[1], points[2], points[3]]
    } else {
        let (a, b, c) = (points[0], points[1], points[2]);
        [
            [
                a[0] + 2. / 3. * (b[0] - a[0]),
                a[1] + 2. / 3. * (b[1] - a[1]),
            ],
            [
                c[0] + 2. / 3. * (b[0] - c[0]),
                c[1] + 2. / 3. * (b[1] - c[1]),
            ],
            c,
        ]
    }
}

/// Adaptive cubic outline used only for screen painting/picking, in the caller's units.
/// The retained source and PDF/SVG curves remain analytic.
pub fn flatten_cubic(start: Point, c: [Point; 3], tolerance: f64) -> Vec<Point> {
    fn distance(p: Point, a: Point, b: Point) -> f64 {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length = dx * dx + dy * dy;
        let t = if length > 0. {
            ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length
        } else {
            0.
        }
        .clamp(0., 1.);
        (p[0] - a[0] - dx * t).hypot(p[1] - a[1] - dy * t)
    }
    fn split(a: Point, c: [Point; 3], tol: f64, depth: u8, out: &mut Vec<Point>) {
        if depth >= 11 || (distance(c[0], a, c[2]) <= tol && distance(c[1], a, c[2]) <= tol) {
            out.push(c[2]);
            return;
        }
        let mid = |a: Point, b: Point| [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
        let p = mid(a, c[0]);
        let q = mid(c[0], c[1]);
        let r = mid(c[1], c[2]);
        let s = mid(p, q);
        let t = mid(q, r);
        let u = mid(s, t);
        split(a, [p, s, u], tol, depth + 1, out);
        split(u, [t, r, c[2]], tol, depth + 1, out);
    }
    let mut out = vec![start];
    split(start, c, tolerance.max(1e-9), 0, &mut out);
    out
}

pub fn outlines(
    cad: &CadShape,
    frame: &RectUm,
    tolerance: f64,
) -> Result<Vec<(Vec<Point>, bool, bool)>> {
    let mut out = Vec::new();
    for path in paths(cad, frame)? {
        let mut points = Vec::new();
        for step in path.steps {
            match step {
                Step::Move(p) => {
                    if !points.is_empty() {
                        out.push((std::mem::take(&mut points), false, path.band));
                    }
                    points.push(p);
                }
                Step::Line(p) => points.push(p),
                Step::Cubic(c) => {
                    if let Some(start) = points.last().copied() {
                        points.extend(flatten_cubic(start, c, tolerance).into_iter().skip(1));
                    }
                }
                Step::Close => {
                    out.push((std::mem::take(&mut points), true, path.band));
                }
            }
        }
        if !points.is_empty() {
            out.push((points, false, path.band));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cubic_circle_images_stay_within_submicrometre_paper_error() {
        for radius in [100., 1000., 10000., 100000., 1e6, 1e7] {
            for gain in [0.1, 1., 3., 10.] {
                let cad = CadShape {
                    reference: SizeUm {
                        width: 10000,
                        height: 10000,
                    },
                    geometry: Geometry::Circle {
                        center: [0., 0.],
                        radius,
                    },
                };
                let frame = RectUm::new(0, 0, (10000. * gain) as i32, (10000. * gain) as i32);
                let paths = paths(&cad, &frame).unwrap();
                let mut start = [radius * gain, 0.];
                for step in &paths[0].steps {
                    match step {
                        Step::Move(p) => start = *p,
                        Step::Cubic(c) => {
                            for i in 0..=100 {
                                let t = i as f64 / 100.;
                                let u = 1. - t;
                                let at = |j| {
                                    u * u * u * start[j]
                                        + 3. * u * u * t * c[0][j]
                                        + 3. * u * t * t * c[1][j]
                                        + t * t * t * c[2][j]
                                };
                                assert!(
                                    (at(0).hypot(at(1)) - radius * gain).abs() < 0.0251,
                                    "{radius} {gain}"
                                );
                            }
                            start = c[2];
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    #[test]
    fn rational_nurbs_circle_and_selection_frame_do_not_replace_the_source() {
        let cad = CadShape {
            reference: SizeUm {
                width: 50000,
                height: 50000,
            },
            geometry: Geometry::Nurbs {
                degree: 2,
                control: vec![[50000., 0.], [50000., 50000.], [0., 50000.]],
                knots: vec![0., 0., 0., 1., 1., 1.],
                weights: vec![1., core::f64::consts::FRAC_1_SQRT_2, 1.],
                closed: false,
                periodic: false,
                fit: vec![],
                tangents: None,
                fit_tolerance: None,
            },
        };
        let original = cad.clone();
        let paths = paths(&cad, &RectUm::new(0, 0, 50000, 50000)).unwrap();
        for step in &paths[0].steps {
            if let Step::Move(p) | Step::Line(p) = step {
                assert!((p[0].hypot(p[1]) - 50000.).abs() < 1e-8);
            }
        }
        assert_eq!(original, cad);
        assert!(matches!(paths[0].steps.first(), Some(Step::Move(_))));
    }
}
