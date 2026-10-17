//! A hatch's boundary as its source holds it (docs/adr/0328 §1): closed
//! loops whose edges are lines, circular arcs (bulges), ellipses' arcs and
//! NURBS curves, exact. The ring and holes a hatch is drawn, picked and
//! measured by are drawn from it (`rings`): arcs as a polygon's are, curves
//! within 0.1 mm (`curve_outline`). A region cut by the area operations
//! keeps its lines and arcs exact; `of_area` finds again the ellipses and
//! curves of the objects it was cut from, whole or in part. A transform
//! carries the loops (`mapped`): a similarity keeps the arcs, an affine map
//! turns them into ellipses' arcs, a curve's control points move with it.

use crate::entity::{HatchCurve, HatchLoop, Shape, polygon_ring};
use crate::geom::arc::{DEFAULT_STEP, norm_angle};
use crate::geom::arrangement::Area;
use crate::geom::bulge::{bulge_arc, bulge_path_outline, has_bulges};
use crate::geom::curve_outline::{ellipse_outline, spline_outline};
use crate::geom::ellipse::{EllipseGeom, ellipse_point, is_full_ellipse, param_of_point};
use crate::geom::nurbs::{self, Nurbs, OwnedNurbs};
use crate::geom::spline::catmull_rom_beziers;
use crate::jsmath::{TAU, atan2, cos, js_hypot, js_max, sin};
use crate::vec2::Vec2;
use std::collections::HashMap;

/// How far an edge's curve may end from its loop's point (m).
pub const END_TOL: f64 = 1e-6;

/// The shortest run of a curve's outline vertices `of_area` takes back as the curve.
const RUN: usize = 3;

/// An ellipse edge as the geometry of its counter-clockwise span, and whether it runs clockwise.
fn ellipse_span(c: Vec2, major: Vec2, ratio: f64, t0: f64, t1: f64) -> (EllipseGeom, bool) {
    let cw = t1 < t0;
    let (lo, hi) = if cw { (t1, t0) } else { (t0, t1) };
    (
        EllipseGeom {
            c,
            major,
            ratio,
            t0: lo,
            t1: hi,
        },
        cw,
    )
}

fn nurbs_view<'a>(
    degree: usize,
    ctrl: &'a [Vec2],
    knots: &'a [f64],
    weights: Option<&'a [f64]>,
) -> Nurbs<'a> {
    Nurbs {
        degree,
        ctrl,
        knots,
        weights,
    }
}

/// A curve edge's start and end.
pub fn curve_ends(c: &HatchCurve) -> Option<(Vec2, Vec2)> {
    match c {
        HatchCurve::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = EllipseGeom {
                c: *c,
                major: *major,
                ratio: *ratio,
                t0: *t0,
                t1: *t1,
            };
            Some((ellipse_point(&g, *t0), ellipse_point(&g, *t1)))
        }
        HatchCurve::Nurbs {
            degree,
            ctrl,
            knots,
            weights,
        } => {
            let n = nurbs_view(*degree, ctrl, knots, weights.as_deref());
            if !n.valid() {
                return None;
            }
            let (lo, hi) = n.domain();
            Some((n.point(lo), n.point(hi)))
        }
    }
}

/// A curve edge's points from its start to its end, both included (a whole
/// ellipse's without repeating its start).
pub fn curve_points(c: &HatchCurve) -> Vec<Vec2> {
    match c {
        HatchCurve::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let (g, cw) = ellipse_span(*c, *major, *ratio, *t0, *t1);
            let mut pts = ellipse_outline(&g);
            if cw {
                pts.reverse();
                // A whole ellipse run backwards still starts at its start.
                if is_full_ellipse(&g) {
                    pts.rotate_right(1);
                }
            }
            pts
        }
        HatchCurve::Nurbs {
            degree,
            ctrl,
            knots,
            weights,
        } => {
            let n = nurbs_view(*degree, ctrl, knots, weights.as_deref());
            if n.valid() {
                n.tessellate(nurbs::TOLERANCE)
            } else {
                ctrl.clone()
            }
        }
    }
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(a.x - b.x, a.y - b.y)
}

fn curve_at(l: &HatchLoop, i: usize) -> Option<&HatchCurve> {
    l.curves
        .as_ref()
        .and_then(|c| c.get(i))
        .and_then(Option::as_ref)
}

/// A loop's ring as a hatch is drawn by: lines and arcs as a polygon's
/// ring, a curve's chords within 0.1 mm; a whole closed curve's own ring.
pub fn loop_ring(l: &HatchLoop) -> Vec<Vec2> {
    let n = l.pts.len();
    if l.curves
        .as_ref()
        .is_none_or(|c| c.iter().all(Option::is_none))
    {
        return polygon_ring(&l.pts, l.bulges.as_deref());
    }
    if n == 1 {
        return curve_at(l, 0).map(curve_points).unwrap_or_default();
    }
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (l.pts[i], l.pts[(i + 1) % n]);
        out.push(a);
        if let Some(c) = curve_at(l, i) {
            let pts = curve_points(c);
            if pts.len() > 2 {
                out.extend_from_slice(&pts[1..pts.len() - 1]);
            }
            continue;
        }
        let bulge = l.bulges.as_ref().map_or(0.0, |b| b[i]);
        if bulge != 0.0 {
            let arc = bulge_path_outline(&[a, b], Some(&[bulge, 0.0]), false, DEFAULT_STEP);
            if arc.len() > 2 {
                out.extend_from_slice(&arc[1..arc.len() - 1]);
            }
        }
    }
    out
}

/// The ring and holes a hatch with this boundary is drawn by.
pub fn rings(loops: &[HatchLoop]) -> (Vec<Vec2>, Vec<Vec<Vec2>>) {
    let mut it = loops.iter().map(loop_ring);
    let ring = it.next().unwrap_or_default();
    (ring, it.collect())
}

/// What is wrong with a boundary the contract's form allows (docs/adr/0328
/// §1): a curve whose numbers make none, or whose ends are not its loop's
/// points within [`END_TOL`]; a ring drawn with fewer than three points.
pub fn problem(loops: &[HatchLoop]) -> Option<String> {
    for (k, l) in loops.iter().enumerate() {
        let n = l.pts.len();
        for i in 0..n {
            let Some(c) = curve_at(l, i) else { continue };
            let Some((s, e)) = curve_ends(c) else {
                return Some(format!(
                    "Tarama sınırının {}. halkasının {}. kenarı geçerli bir eğri değil.",
                    k + 1,
                    i + 1
                ));
            };
            let (a, b) = (l.pts[i], l.pts[(i + 1) % n]);
            if dist(s, a) > END_TOL || dist(e, b) > END_TOL {
                return Some(format!(
                    "Tarama sınırının {}. halkasının {}. kenarının eğrisi kenarın uçlarında başlayıp bitmiyor.",
                    k + 1,
                    i + 1
                ));
            }
        }
        if loop_ring(l).len() < 3 {
            return Some(format!(
                "Tarama sınırının {}. halkası bir alan kapatmıyor.",
                k + 1
            ));
        }
    }
    None
}

/// Whether a boundary holds more than its drawn ring: an arc or a curve. A
/// boundary of straight edges alone is its ring, and a hatch keeps none.
pub fn curved(loops: &[HatchLoop]) -> bool {
    loops.iter().any(|l| {
        l.bulges.iter().flatten().any(|b| *b != 0.0)
            || l.curves.iter().flatten().any(Option::is_some)
    })
}

/// A loop as a polygon's ring holds it (docs/adr/0329 §4): its lines and
/// arcs as they are, each curve's chords within 0.1 mm in its place; and
/// whether a curve was taken so.
pub fn chorded(l: &HatchLoop) -> (Vec<Vec2>, Vec<f64>, bool) {
    let n = l.pts.len();
    let mut pts = Vec::with_capacity(n);
    let mut bulges = Vec::with_capacity(n);
    let mut approximated = false;
    for i in 0..n {
        match curve_at(l, i) {
            Some(c) => {
                approximated = true;
                let inner = curve_points(c);
                // A whole closed curve's ring starts at its start; an edge's chords between its ends.
                if n == 1 {
                    pts.extend_from_slice(&inner);
                    bulges.extend(std::iter::repeat_n(0.0, inner.len()));
                    continue;
                }
                pts.push(l.pts[i]);
                bulges.push(0.0);
                if inner.len() > 2 {
                    pts.extend_from_slice(&inner[1..inner.len() - 1]);
                    bulges.extend(std::iter::repeat_n(0.0, inner.len() - 2));
                }
            }
            None => {
                pts.push(l.pts[i]);
                bulges.push(l.bulges.as_ref().map_or(0.0, |b| b[i]));
            }
        }
    }
    (pts, bulges, approximated)
}

/// The area a boundary encloses, arcs exact, curves as chords.
pub fn area_of(loops: &[HatchLoop]) -> Option<Area> {
    let mut rings = loops.iter().map(|l| {
        let (pts, bulges, _) = chorded(l);
        crate::geom::arrangement::Ring {
            pts,
            bulges: bulges.iter().any(|b| *b != 0.0).then_some(bulges),
        }
    });
    let outer = rings.next()?;
    Some(Area {
        outer,
        holes: rings.collect(),
    })
}

/// A loop that is one whole closed curve as a shape: an ellipse, or a
/// closed NURBS curve; none for any other loop.
pub fn whole_curve(l: &HatchLoop) -> Option<Shape> {
    if l.pts.len() != 1 {
        return None;
    }
    match curve_at(l, 0)? {
        HatchCurve::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let (lo, hi) = if t1 < t0 { (*t1, *t0) } else { (*t0, *t1) };
            Some(Shape::Ellipse {
                c: *c,
                major: *major,
                ratio: *ratio,
                t0: lo,
                t1: hi,
            })
        }
        HatchCurve::Nurbs {
            degree,
            ctrl,
            knots,
            weights,
        } => Some(Shape::Nurbs {
            degree: *degree,
            ctrl: ctrl.clone(),
            knots: knots.clone(),
            weights: weights.clone(),
            closed: true,
            periodic: false,
            fit: None,
            fit_tolerance: None,
            tangents: None,
        }),
    }
}

/// A boundary of lines and arcs: an area's rings.
pub fn of_rings(area: &Area) -> Vec<HatchLoop> {
    std::iter::once(&area.outer)
        .chain(&area.holes)
        .map(|r| HatchLoop {
            pts: r.pts.clone(),
            bulges: r.bulges.clone().filter(|b| has_bulges(Some(b))),
            curves: None,
        })
        .collect()
}

/// A closed curve an area was cut from, as `areas_of_entity` takes it: its
/// ring's vertices and the curve whose points they are.
struct Source {
    pts: Vec<Vec2>,
    at: HashMap<(u64, u64), usize>,
    curve: Closed,
}

enum Closed {
    Ellipse(EllipseGeom),
    Nurbs(OwnedNurbs),
}

fn key(p: Vec2) -> (u64, u64) {
    (p.x.to_bits(), p.y.to_bits())
}

/// A closed fit-point curve's cubic Bézier spans as one NURBS curve, exact.
fn spline_nurbs(pts: &[Vec2]) -> Option<OwnedNurbs> {
    let spans = catmull_rom_beziers(pts, true);
    if spans.is_empty() {
        return None;
    }
    let mut ctrl = vec![spans[0].ctrl[0]];
    let mut knots = vec![0.0; 4];
    for (k, s) in spans.iter().enumerate() {
        ctrl.extend_from_slice(&s.ctrl[1..]);
        let u = (k + 1) as f64;
        let times = if k + 1 == spans.len() { 4 } else { 3 };
        knots.extend(std::iter::repeat_n(u, times));
    }
    let n = OwnedNurbs {
        degree: 3,
        ctrl,
        knots,
        weights: None,
    };
    n.view().valid().then_some(n)
}

fn source_of(shape: &Shape) -> Option<Source> {
    let (pts, curve) = match shape {
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = EllipseGeom {
                c: *c,
                major: *major,
                ratio: *ratio,
                t0: *t0,
                t1: *t1,
            };
            if !is_full_ellipse(&g) {
                return None;
            }
            (ellipse_outline(&g), Closed::Ellipse(g))
        }
        Shape::Spline { pts, closed: true } if pts.len() >= 3 => {
            (spline_outline(pts, true), Closed::Nurbs(spline_nurbs(pts)?))
        }
        Shape::Nurbs { closed: true, .. } => {
            let n = nurbs::of(shape)?;
            let mut ring = nurbs::outline(shape);
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            (
                ring,
                Closed::Nurbs(OwnedNurbs {
                    degree: n.degree,
                    ctrl: n.ctrl.to_vec(),
                    knots: n.knots.to_vec(),
                    weights: n.weights.map(<[f64]>::to_vec),
                }),
            )
        }
        _ => return None,
    };
    if pts.len() < RUN {
        return None;
    }
    let at = pts.iter().enumerate().map(|(i, p)| (key(*p), i)).collect();
    Some(Source { pts, at, curve })
}

/// The parameter of a closed NURBS curve's outline vertex `i` (`p`): the
/// seam's (the outline's first) at the domain's start or end as `at_end`
/// says, any other found on the overlapping half of the curve its place in
/// the outline points to, so the seam's two ends are never mistaken.
fn nurbs_param(n: &OwnedNurbs, len: usize, i: usize, p: Vec2, at_end: bool) -> Option<f64> {
    let v = n.view();
    let (lo, hi) = v.domain();
    if i == 0 {
        return Some(if at_end { hi } else { lo });
    }
    let (a, b) = if 2 * i <= len {
        (lo, lo + (hi - lo) * 0.75)
    } else {
        (lo + (hi - lo) * 0.25, hi)
    };
    let part = v.sub(a, b)?;
    Some(part.view().closest(p))
}

/// The curve of a run of a source's outline vertices from `a` (outline
/// place `ia`) to `b` (`ib`), the way `forward` says (outline order, or
/// against it).
fn run_curve(
    src: &Source,
    (a, ia): (Vec2, usize),
    (b, ib): (Vec2, usize),
    forward: bool,
    whole: bool,
) -> Option<HatchCurve> {
    match &src.curve {
        Closed::Ellipse(g) => {
            let ta = param_of_point(g, a);
            let t1 = if whole {
                if forward { ta + TAU } else { ta - TAU }
            } else {
                let tb = param_of_point(g, b);
                if forward {
                    ta + norm_angle(tb - ta)
                } else {
                    ta - norm_angle(ta - tb)
                }
            };
            Some(HatchCurve::Ellipse {
                c: g.c,
                major: g.major,
                ratio: g.ratio,
                t0: ta,
                t1,
            })
        }
        Closed::Nurbs(n) => {
            let v = n.view();
            let part = if whole {
                n.clone()
            } else {
                let len = src.pts.len();
                // A forward run starts after the seam and may end on it; a backward one the other way round.
                let ua = nurbs_param(n, len, ia, a, !forward)?;
                let ub = nurbs_param(n, len, ib, b, forward)?;
                let (lo, hi) = if forward { (ua, ub) } else { (ub, ua) };
                v.sub(lo, hi)?
            };
            let part = if forward {
                part
            } else {
                part.view().reversed()
            };
            Some(HatchCurve::Nurbs {
                degree: part.degree,
                ctrl: part.ctrl,
                knots: part.knots,
                weights: part.weights,
            })
        }
    }
}

/// One of an area's rings as a loop, its runs of a source curve's outline
/// vertices taken back as the curve. A NURBS run stops at the curve's seam
/// (its outline's first vertex).
fn ring_loop(pts: &[Vec2], bulges: Option<&[f64]>, sources: &[Source]) -> HatchLoop {
    let n = pts.len();
    let plain = || HatchLoop {
        pts: pts.to_vec(),
        bulges: bulges.filter(|b| has_bulges(Some(b))).map(<[f64]>::to_vec),
        curves: None,
    };
    if sources.is_empty() || n < RUN {
        return plain();
    }
    let bulge = |i: usize| bulges.map_or(0.0, |b| b[i]);
    // Each vertex's source and place in its outline.
    let mark: Vec<Option<(usize, usize)>> = pts
        .iter()
        .map(|p| {
            sources
                .iter()
                .enumerate()
                .find_map(|(s, src)| src.at.get(&key(*p)).map(|&i| (s, i)))
        })
        .collect();
    // Whether the edge from vertex i to the next walks a source's outline: +1 along, −1 against.
    // A run stops at a NURBS curve's seam unless the whole ring is the curve.
    let step_at = |i: usize, through_seam: bool| -> Option<(usize, bool)> {
        let j = (i + 1) % n;
        let ((s, a), (t, b)) = (mark[i]?, mark[j]?);
        if s != t || bulge(i) != 0.0 {
            return None;
        }
        let len = sources[s].pts.len();
        let seam = !through_seam && matches!(sources[s].curve, Closed::Nurbs(_));
        if b == (a + 1) % len && !(seam && b == 0) {
            Some((s, true))
        } else if a == (b + 1) % len && !(seam && a == 0) {
            Some((s, false))
        } else {
            None
        }
    };
    let whole: Vec<Option<(usize, bool)>> = (0..n).map(|i| step_at(i, true)).collect();
    // The whole ring one closed curve.
    if let Some(Some((s, forward))) = whole.first().copied()
        && whole.iter().all(|x| *x == Some((s, forward)))
        && sources[s].pts.len() == n
    {
        let src = &sources[s];
        let start = match &src.curve {
            Closed::Ellipse(_) => pts[0],
            Closed::Nurbs(c) => {
                let v = c.view();
                v.point(v.domain().0)
            }
        };
        if let Some(curve) = run_curve(src, (start, 0), (start, 0), forward, true) {
            return HatchLoop {
                pts: vec![start],
                bulges: None,
                curves: Some(vec![Some(curve)]),
            };
        }
        return plain();
    }
    let steps: Vec<Option<(usize, bool)>> = (0..n).map(|i| step_at(i, false)).collect();
    // Start where a run cannot go on (an edge that walks no outline, or another source or way).
    let Some(first) = (0..n).find(|&i| steps[(i + n - 1) % n] != steps[i]) else {
        return plain();
    };
    let mut out_pts = Vec::with_capacity(n);
    let mut out_bulges = Vec::with_capacity(n);
    let mut out_curves = Vec::with_capacity(n);
    let mut k = 0;
    while k < n {
        let i = (first + k) % n;
        let Some((s, forward)) = steps[i] else {
            out_pts.push(pts[i]);
            out_bulges.push(bulge(i));
            out_curves.push(None);
            k += 1;
            continue;
        };
        let mut len = 1;
        while k + len < n && steps[(first + k + len) % n] == Some((s, forward)) {
            len += 1;
        }
        let end = (i + len) % n;
        let place = |k: usize| mark[k].map_or(0, |(_, at)| at);
        if len + 1 >= RUN
            && let Some(curve) = run_curve(
                &sources[s],
                (pts[i], place(i)),
                (pts[end], place(end)),
                forward,
                false,
            )
        {
            out_pts.push(pts[i]);
            out_bulges.push(0.0);
            out_curves.push(Some(curve));
        } else {
            for m in 0..len {
                let at = (i + m) % n;
                out_pts.push(pts[at]);
                out_bulges.push(bulge(at));
                out_curves.push(None);
            }
        }
        k += len;
    }
    let curved = out_curves.iter().any(Option::is_some);
    HatchLoop {
        pts: out_pts,
        bulges: out_bulges.iter().any(|b| *b != 0.0).then_some(out_bulges),
        curves: curved.then_some(out_curves),
    }
}

/// An area's boundary (docs/adr/0328 §1): its lines and arcs as they are,
/// and where its rings walk the chords of a closed ellipse, fit-point curve
/// or NURBS curve of `sources` (the objects it was cut from), that curve
/// back, whole or between the vertices the cut left on it.
pub fn of_area(area: &Area, sources: &[Shape]) -> Vec<HatchLoop> {
    let sources: Vec<Source> = sources.iter().filter_map(source_of).collect();
    std::iter::once(&area.outer)
        .chain(&area.holes)
        .map(|r| ring_loop(&r.pts, r.bulges.as_deref(), &sources))
        .collect()
}

/// J·v for J = [a, b, c, d] (columns (a, b) and (c, d)).
fn lin(j: [f64; 4], v: Vec2) -> Vec2 {
    Vec2::new(j[0] * v.x + j[2] * v.y, j[1] * v.x + j[3] * v.y)
}

/// The ellipse c + M·cos t + N·sin t (t from `t0` by the signed `sweep`)
/// under `at` with linear part J: the exact ellipse of its image, run the
/// way the image runs; none when it collapses.
fn image_arc(
    at: &dyn Fn(Vec2) -> Option<Vec2>,
    j: [f64; 4],
    c: Vec2,
    m: Vec2,
    n: Vec2,
    t0: f64,
    sweep: f64,
) -> Option<HatchCurve> {
    let (c1, c2) = (lin(j, m), lin(j, n));
    let p = c1.x * c1.x + c2.x * c2.x;
    let s = c1.y * c1.y + c2.y * c2.y;
    let q = c1.x * c1.y + c2.x * c2.y;
    let phi = 0.5 * atan2(2.0 * q, p - s);
    let root = js_hypot((p - s) / 2.0, q);
    let s1 = ((p + s) / 2.0 + root).sqrt();
    let s2 = js_max((p + s) / 2.0 - root, 0.0).sqrt();
    if !(s1 > 0.0 && s2 > s1 * 1e-12) {
        return None;
    }
    let u1 = Vec2::new(cos(phi), sin(phi));
    let u2 = Vec2::new(-u1.y, u1.x);
    let param = |t: f64| {
        let v = lin(
            j,
            Vec2::new(m.x * cos(t) + n.x * sin(t), m.y * cos(t) + n.y * sin(t)),
        );
        norm_angle(atan2(
            (v.x * u2.x + v.y * u2.y) / s2,
            (v.x * u1.x + v.y * u1.y) / s1,
        ))
    };
    let det = j[0] * j[3] - j[1] * j[2];
    // [M N] turns counter-clockwise; the image runs the source's way when J keeps the turn.
    let ccw = (sweep > 0.0) == (det > 0.0);
    let ts = param(t0);
    let whole = sweep.abs() >= TAU - 1e-12;
    let t1 = if whole {
        if ccw { ts + TAU } else { ts - TAU }
    } else {
        let te = param(t0 + sweep);
        if ccw {
            ts + norm_angle(te - ts)
        } else {
            ts - norm_angle(ts - te)
        }
    };
    Some(HatchCurve::Ellipse {
        c: at(c)?,
        major: Vec2::new(s1 * u1.x, s1 * u1.y),
        ratio: s2 / s1,
        t0: ts,
        t1,
    })
}

/// Whether J turns, scales evenly and perhaps mirrors.
fn similar(j: [f64; 4]) -> bool {
    let [a, b, c, d] = j;
    let tol = 1e-12 * js_max(js_hypot(a, b), 1e-300);
    ((a - d).abs() <= tol && (b + c).abs() <= tol) || ((a + d).abs() <= tol && (b - c).abs() <= tol)
}

/// The loops under a map that is affine (`at` the points' images, `j` its
/// linear part): points move, a similarity keeps the arcs (a mirror turns
/// their bulges), any other affine makes each arc an ellipse's arc; an
/// ellipse is its image's, a curve's control points move. None when a
/// point has no image or a curve collapses.
pub fn mapped(
    loops: &[HatchLoop],
    at: &dyn Fn(Vec2) -> Option<Vec2>,
    j: [f64; 4],
) -> Option<Vec<HatchLoop>> {
    let even = similar(j);
    let flip = j[0] * j[3] - j[1] * j[2] < 0.0;
    loops
        .iter()
        .map(|l| {
            let n = l.pts.len();
            let pts = l.pts.iter().map(|&p| at(p)).collect::<Option<Vec<_>>>()?;
            let mut bulges = l.bulges.clone().unwrap_or_else(|| vec![0.0; n]);
            let mut curves: Vec<Option<HatchCurve>> =
                l.curves.clone().unwrap_or_else(|| vec![None; n]);
            for i in 0..n {
                if let Some(c) = &curves[i] {
                    curves[i] = Some(match c {
                        HatchCurve::Ellipse {
                            c,
                            major,
                            ratio,
                            t0,
                            t1,
                        } => {
                            let minor = Vec2::new(-major.y * ratio, major.x * ratio);
                            image_arc(at, j, *c, *major, minor, *t0, t1 - t0)?
                        }
                        HatchCurve::Nurbs {
                            degree,
                            ctrl,
                            knots,
                            weights,
                        } => HatchCurve::Nurbs {
                            degree: *degree,
                            ctrl: ctrl.iter().map(|&p| at(p)).collect::<Option<Vec<_>>>()?,
                            knots: knots.clone(),
                            weights: weights.clone(),
                        },
                    });
                    continue;
                }
                if bulges[i] == 0.0 {
                    continue;
                }
                if even {
                    if flip {
                        bulges[i] = -bulges[i];
                    }
                    continue;
                }
                let arc = bulge_arc(l.pts[i], l.pts[(i + 1) % n], bulges[i])?;
                let (m, nn) = (Vec2::new(arc.r, 0.0), Vec2::new(0.0, arc.r));
                curves[i] = Some(image_arc(at, j, arc.c, m, nn, arc.a0, arc.sweep)?);
                bulges[i] = 0.0;
            }
            let curved = curves.iter().any(Option::is_some);
            Some(HatchLoop {
                pts,
                bulges: bulges.iter().any(|b| *b != 0.0).then_some(bulges),
                curves: curved.then_some(curves),
            })
        })
        .collect()
}

/// The outer loop's point `index` moved to `p` (a grip, docs/adr/0328 §1):
/// lines and arcs keep their bulges; a curve that ends there becomes its
/// chords within 0.1 mm, the rest stays exact. None for no such point.
pub fn moved_point(loops: &[HatchLoop], index: usize, p: Vec2) -> Option<Vec<HatchLoop>> {
    let l = loops.first()?;
    let n = l.pts.len();
    if index >= n {
        return None;
    }
    let before = (index + n - 1) % n;
    let mut pts = Vec::with_capacity(n);
    let mut bulges = Vec::with_capacity(n);
    let mut curves = Vec::with_capacity(n);
    for i in 0..n {
        pts.push(if i == index { p } else { l.pts[i] });
        let c = curve_at(l, i);
        if c.is_some() && (i == index || i == before) {
            // The curve's inner chord points, the edge now straight pieces.
            bulges.push(0.0);
            curves.push(None);
            let inner = c.map(curve_points).unwrap_or_default();
            if inner.len() > 2 {
                for q in &inner[1..inner.len() - 1] {
                    pts.push(*q);
                    bulges.push(0.0);
                    curves.push(None);
                }
            }
            continue;
        }
        bulges.push(l.bulges.as_ref().map_or(0.0, |b| b[i]));
        curves.push(c.cloned());
    }
    let mut out = loops.to_vec();
    out[0] = HatchLoop {
        bulges: bulges.iter().any(|b| *b != 0.0).then_some(bulges),
        curves: curves.iter().any(Option::is_some).then_some(curves),
        pts,
    };
    Some(out)
}

/// The loops' points `moves` names moved by `d` (Esnet, docs/adr/0297): an
/// arc keeps its bulge, a curve whose ends both move goes along; none when a
/// curve's ends part.
pub fn stretched(
    loops: &[HatchLoop],
    moves: &dyn Fn(Vec2) -> bool,
    d: Vec2,
) -> Option<Vec<HatchLoop>> {
    let shift = |p: Vec2| Vec2::new(p.x + d.x, p.y + d.y);
    loops
        .iter()
        .map(|l| {
            let n = l.pts.len();
            let mut curves = l.curves.clone();
            if let Some(cs) = &mut curves {
                for i in 0..n {
                    let Some(c) = &cs[i] else { continue };
                    let (a, b) = (moves(l.pts[i]), moves(l.pts[(i + 1) % n]));
                    if a != b {
                        return None;
                    }
                    if !a {
                        continue;
                    }
                    cs[i] = Some(match c {
                        HatchCurve::Ellipse {
                            c,
                            major,
                            ratio,
                            t0,
                            t1,
                        } => HatchCurve::Ellipse {
                            c: shift(*c),
                            major: *major,
                            ratio: *ratio,
                            t0: *t0,
                            t1: *t1,
                        },
                        HatchCurve::Nurbs {
                            degree,
                            ctrl,
                            knots,
                            weights,
                        } => HatchCurve::Nurbs {
                            degree: *degree,
                            ctrl: ctrl.iter().map(|&p| shift(p)).collect(),
                            knots: knots.clone(),
                            weights: weights.clone(),
                        },
                    });
                }
            }
            Some(HatchLoop {
                pts: l
                    .pts
                    .iter()
                    .map(|&p| if moves(p) { shift(p) } else { p })
                    .collect(),
                bulges: l.bulges.clone(),
                curves,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::arrangement::Ring;
    use crate::geom::region::subtract_areas;
    use crate::ops::areas::areas_of_entity;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn ellipse() -> Shape {
        Shape::Ellipse {
            c: v(10.0, 5.0),
            major: v(4.0, 0.0),
            ratio: 0.5,
            t0: 0.0,
            t1: TAU,
        }
    }

    #[test]
    fn a_whole_ellipse_comes_back_as_one_curve() {
        let e = ellipse();
        let area = areas_of_entity(&e).remove(0);
        let loops = of_area(&area, std::slice::from_ref(&e));
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].pts.len(), 1);
        let Some(Some(HatchCurve::Ellipse { t0, t1, ratio, .. })) =
            loops[0].curves.as_ref().map(|c| c[0].clone())
        else {
            panic!("{loops:?}")
        };
        assert_eq!(ratio, 0.5);
        assert!(((t1 - t0).abs() - TAU).abs() < 1e-12);
        assert!(problem(&loops).is_none());
        // Drawn as the area's ring was.
        let (ring, _) = rings(&loops);
        assert_eq!(ring.len(), area.outer.pts.len());
    }

    #[test]
    fn an_ellipse_cut_by_a_square_keeps_its_arc() {
        let e = ellipse();
        let base = areas_of_entity(&e).remove(0);
        let square = Area {
            outer: Ring {
                pts: vec![v(10.0, 0.0), v(20.0, 0.0), v(20.0, 10.0), v(10.0, 10.0)],
                bulges: None,
            },
            holes: Vec::new(),
        };
        let left = subtract_areas(&[base], &[square]);
        assert_eq!(left.len(), 1);
        let loops = of_area(&left[0], std::slice::from_ref(&e));
        assert!(problem(&loops).is_none(), "{loops:?}");
        let curves: Vec<&HatchCurve> = loops[0].curves.iter().flatten().flatten().collect();
        assert_eq!(curves.len(), 1, "{loops:?}");
        let HatchCurve::Ellipse { t0, t1, .. } = curves[0] else {
            panic!()
        };
        // The left half, from the top to the bottom: π/2 to 3π/2 one way or the other, within the cut's chord.
        assert!(((t1 - t0).abs() - TAU / 2.0).abs() < 0.05, "{t0} {t1}");
    }

    #[test]
    fn arcs_stay_bulges_and_lines_lines() {
        let area = Area {
            outer: Ring {
                pts: vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0), v(0.0, 4.0)],
                bulges: Some(vec![0.0, 0.5, 0.0, 0.0]),
            },
            holes: Vec::new(),
        };
        let loops = of_area(&area, &[]);
        assert_eq!(loops, of_rings(&area));
        assert_eq!(loops[0].bulges.as_deref(), Some(&[0.0, 0.5, 0.0, 0.0][..]));
        let (ring, _) = rings(&loops);
        assert_eq!(
            ring,
            polygon_ring(&area.outer.pts, area.outer.bulges.as_deref())
        );
    }

    #[test]
    fn an_affine_map_makes_an_arc_an_ellipse() {
        let l = HatchLoop {
            pts: vec![v(1.0, 0.0), v(-1.0, 0.0)],
            bulges: Some(vec![1.0, 1.0]),
            curves: None,
        };
        let j = [2.0, 0.0, 0.0, 1.0];
        let at = |p: Vec2| Some(lin(j, p));
        let out = mapped(&[l], &at, j).expect("maps");
        assert!(problem(&out).is_none(), "{out:?}");
        let ring = loop_ring(&out[0]);
        for p in ring {
            // On x²/4 + y² = 1.
            assert!((p.x * p.x / 4.0 + p.y * p.y - 1.0).abs() < 1e-3, "{p:?}");
        }
        // A mirror keeps arcs as bulges, turned.
        let m = [1.0, 0.0, 0.0, -1.0];
        let at = |p: Vec2| Some(lin(m, p));
        let l = HatchLoop {
            pts: vec![v(1.0, 0.0), v(-1.0, 0.0)],
            bulges: Some(vec![1.0, 1.0]),
            curves: None,
        };
        let out = mapped(&[l], &at, m).expect("maps");
        assert_eq!(out[0].bulges.as_deref(), Some(&[-1.0, -1.0][..]));
    }

    #[test]
    fn a_mirrored_ellipse_arc_runs_the_other_way() {
        let l = HatchLoop {
            pts: vec![v(14.0, 5.0), v(6.0, 5.0)],
            bulges: None,
            curves: Some(vec![
                Some(HatchCurve::Ellipse {
                    c: v(10.0, 5.0),
                    major: v(4.0, 0.0),
                    ratio: 0.5,
                    t0: 0.0,
                    t1: TAU / 2.0,
                }),
                None,
            ]),
        };
        assert!(problem(std::slice::from_ref(&l)).is_none());
        let m = [1.0, 0.0, 0.0, -1.0];
        let at = |p: Vec2| Some(lin(m, p));
        let out = mapped(&[l], &at, m).expect("maps");
        assert!(problem(&out).is_none(), "{out:?}");
        let Some(Some(HatchCurve::Ellipse { t0, t1, .. })) =
            out[0].curves.as_ref().map(|c| c[0].clone())
        else {
            panic!()
        };
        assert!(t1 < t0);
    }

    #[test]
    fn a_grip_keeps_arcs_and_breaks_only_the_curve_it_moves() {
        let l = HatchLoop {
            pts: vec![v(0.0, 0.0), v(8.0, 0.0), v(8.0, 4.0), v(0.0, 4.0)],
            bulges: Some(vec![0.0, 0.0, 0.5, 0.0]),
            curves: None,
        };
        let out = moved_point(std::slice::from_ref(&l), 1, v(9.0, 0.0)).expect("moves");
        assert_eq!(out[0].pts[1], v(9.0, 0.0));
        assert_eq!(out[0].bulges.as_deref(), Some(&[0.0, 0.0, 0.5, 0.0][..]));
        // A curve that ends at the moved point turns into its chords; the loop still holds.
        let e = HatchLoop {
            pts: vec![v(14.0, 5.0), v(6.0, 5.0)],
            bulges: None,
            curves: Some(vec![
                Some(HatchCurve::Ellipse {
                    c: v(10.0, 5.0),
                    major: v(4.0, 0.0),
                    ratio: 0.5,
                    t0: 0.0,
                    t1: TAU / 2.0,
                }),
                None,
            ]),
        };
        let out = moved_point(&[e], 1, v(5.0, 5.0)).expect("moves");
        assert!(out[0].curves.is_none() && out[0].pts.len() > 3, "{out:?}");
        assert!(problem(&out).is_none());
        assert_eq!(moved_point(&out, 999, v(0.0, 0.0)), None);
    }

    #[test]
    fn a_closed_spline_comes_back_as_a_nurbs_curve() {
        let s = Shape::Spline {
            pts: vec![v(0.0, 0.0), v(6.0, -1.0), v(8.0, 4.0), v(2.0, 6.0)],
            closed: true,
        };
        let area = areas_of_entity(&s).remove(0);
        let loops = of_area(&area, std::slice::from_ref(&s));
        assert!(problem(&loops).is_none(), "{loops:?}");
        assert_eq!(loops[0].pts.len(), 1, "{loops:?}");
        // The curve itself: every vertex of the fit-point curve's chords lies on it.
        let Some(Some(HatchCurve::Nurbs {
            degree,
            ctrl,
            knots,
            weights,
        })) = loops[0].curves.as_ref().map(|c| c[0].clone())
        else {
            panic!("{loops:?}")
        };
        let n = OwnedNurbs {
            degree,
            ctrl,
            knots,
            weights,
        };
        let len = area.outer.pts.len();
        for (i, p) in area.outer.pts.iter().enumerate() {
            let u = nurbs_param(&n, len, i, *p, false).expect("a parameter");
            let q = n.view().point(u);
            assert!(dist(*p, q) < 1e-9, "{i} {p:?} {q:?}");
        }
    }
}
