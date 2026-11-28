//! A hatch's boundary paths read into rings (docs/adr/0146, 9230): edge paths' arcs, the pieces of a boundary flagged
//! not closed joined end to end, and a path's points as the shared core samples them (moved out of emit.rs, which is
//! over the size limit, CLAUDE.md §8).

use super::*;

/// A clockwise arc: DXF stores it mirrored (angles negated). The start that
/// meets the previous edge decides when a writer stored it the other way.
pub(super) fn arc_edge(
    c: Vec2,
    r: f64,
    a0: f64,
    a1: f64,
    ccw: bool,
    prev: Option<Vec2>,
    out: &mut Vec<Vec2>,
) {
    let at = |d: f64| {
        let (s, co) = sin_cos_deg(d);
        v(c.x + r * co, c.y + r * s)
    };
    let (start, end, sweep) = if ccw {
        let span = (a1 - a0).rem_euclid(360.0);
        (a0, a1, if span == 0.0 { 360.0 } else { span })
    } else {
        let mirrored = (-a0, -a1);
        let as_is = (a0, a1);
        let (s, e) = match prev {
            Some(p) if dist(at(as_is.0), p) < dist(at(mirrored.0), p) => as_is,
            _ => mirrored,
        };
        let span = (s - e).rem_euclid(360.0);
        (s, e, -(if span == 0.0 { 360.0 } else { span }))
    };
    out.push(at(start));
    arc_points(c, r, rad(start), rad(sweep), Some(at(end)), out);
}

/// A hatch boundary path as its exact loop, in object coordinates (docs/adr/0328 §1): a polyline path's vertices
/// and bulges; an edge path's lines and arcs as vertices and bulges, its elliptic arcs and splines as curves. None
/// when an edge does not start where the one before it ended (within a millionth of the path's size), or a spline's
/// numbers make no curve: the path is then taken in pieces, as before.
pub(super) fn path_loop(path: &Path) -> Option<CoreLoop> {
    let p2 = |p: &[f64; 2]| CoreVec2::new(p[0], p[1]);
    match path {
        Path::Poly { pts, bulges } => {
            let mut pts: Vec<CoreVec2> = pts.iter().map(p2).collect();
            let mut bulges: Vec<f64> = (0..pts.len())
                .map(|i| bulges.get(i).copied().unwrap_or(0.0))
                .collect();
            if pts.len() > 1 && pts.first() == pts.last() {
                pts.pop();
                bulges.pop();
            }
            if pts.len() < 2 || !bulges.iter().all(|b| b.is_finite()) {
                return None;
            }
            let any = bulges.iter().any(|b| *b != 0.0);
            Some(CoreLoop {
                pts,
                bulges: any.then_some(bulges),
                curves: None,
            })
        }
        Path::Edges(edges) => {
            // How near an edge must start to where the one before ended.
            let size = edges
                .iter()
                .flat_map(|e| match e {
                    Edge::Line { a, b } => vec![a[0].abs(), a[1].abs(), b[0].abs(), b[1].abs()],
                    Edge::Arc { c, r, .. } => vec![c[0].abs() + r, c[1].abs() + r],
                    Edge::Ellipse { c, major, .. } => {
                        vec![c[0].abs() + hypot(major[0], major[1]), c[1].abs()]
                    }
                    Edge::Spline { ctrl, .. } => {
                        ctrl.iter().flat_map(|p| [p[0].abs(), p[1].abs()]).collect()
                    }
                })
                .fold(1.0_f64, f64::max);
            let tol = 1e-6 + 1e-9 * size;
            let near = |a: CoreVec2, b: CoreVec2| hypot(a.x - b.x, a.y - b.y) <= tol;
            let mut pts: Vec<CoreVec2> = Vec::new();
            let mut bulges: Vec<f64> = Vec::new();
            let mut curves: Vec<Option<CoreCurve>> = Vec::new();
            let mut ends: Vec<CoreVec2> = Vec::new();
            let mut prev: Option<CoreVec2> = None;
            for edge in edges {
                let (start, end, bulge, curve) = match edge {
                    Edge::Line { a, b } => (p2(a), p2(b), 0.0, None),
                    Edge::Arc { c, r, a0, a1, ccw } => {
                        let at = |d: f64| {
                            let (s, co) = sin_cos_deg(d);
                            CoreVec2::new(c[0] + r * co, c[1] + r * s)
                        };
                        // As `arc_edge` reads it.
                        let (s, e, sweep) = if *ccw {
                            let span = (a1 - a0).rem_euclid(360.0);
                            (*a0, *a1, if span == 0.0 { 360.0 } else { span })
                        } else {
                            let pv = prev.map(|p| Vec2 { x: p.x, y: p.y });
                            let to = |x: CoreVec2| Vec2 { x: x.x, y: x.y };
                            let (s, e) = match pv {
                                Some(p) if dist(to(at(*a0)), p) < dist(to(at(-a0)), p) => {
                                    (*a0, *a1)
                                }
                                _ => (-a0, -a1),
                            };
                            let span = (s - e).rem_euclid(360.0);
                            (s, e, -(if span == 0.0 { 360.0 } else { span }))
                        };
                        if sweep.abs() >= 360.0 {
                            // A whole circle: two half arcs.
                            let (p0, p1) = (at(s), at(s + sweep / 2.0));
                            let b = if sweep > 0.0 { 1.0 } else { -1.0 };
                            pts.extend([p0, p1]);
                            bulges.extend([b, b]);
                            curves.extend([None, None]);
                            ends.extend([p1, p0]);
                            prev = Some(p0);
                            continue;
                        }
                        (at(s), at(e), crate::math::tan(rad(sweep) / 4.0), None)
                    }
                    Edge::Ellipse {
                        c,
                        major,
                        ratio,
                        a0,
                        a1,
                        ccw,
                    } => {
                        // As `path_points` reads it.
                        let (a0, a1) = if *ccw { (*a0, *a1) } else { (-*a0, -*a1) };
                        let param = |a: f64| atan2(sin(rad(a)) / ratio.max(1e-12), cos(rad(a)));
                        let (t0, mut t1) = (param(a0), param(a1));
                        if *ccw {
                            if t1 <= t0 {
                                t1 += TAU;
                            }
                        } else if t1 >= t0 {
                            t1 -= TAU;
                        }
                        let curve = CoreCurve::Ellipse {
                            c: p2(c),
                            major: p2(major),
                            ratio: *ratio,
                            t0,
                            t1,
                        };
                        let (s, e) = hatch_boundary::curve_ends(&curve)?;
                        (s, e, 0.0, Some(curve))
                    }
                    Edge::Spline {
                        degree,
                        knots,
                        ctrl,
                        weights,
                    } => {
                        let curve = CoreCurve::Nurbs {
                            degree: *degree,
                            ctrl: ctrl.iter().map(p2).collect(),
                            knots: knots.clone(),
                            weights: (weights.len() == ctrl.len()).then(|| weights.clone()),
                        };
                        let (s, e) = hatch_boundary::curve_ends(&curve)?;
                        (s, e, 0.0, Some(curve))
                    }
                };
                if let Some(p) = prev
                    && !near(p, start)
                {
                    return None;
                }
                pts.push(start);
                bulges.push(if bulge.is_finite() {
                    bulge
                } else {
                    return None;
                });
                curves.push(curve);
                ends.push(end);
                prev = Some(end);
            }
            let (first, last) = (*pts.first()?, *ends.last()?);
            if !near(last, first) {
                return None;
            }
            // A curve's ends are its loop's points: snap a line's or an arc's start onto the curve's end before it.
            let n = pts.len();
            for i in 0..n {
                let j = (i + 1) % n;
                if curves[i].is_some() && curves[j].is_none() {
                    pts[j] = ends[i];
                }
            }
            let any_bulge = bulges.iter().any(|b| *b != 0.0);
            let any_curve = curves.iter().any(Option::is_some);
            Some(CoreLoop {
                pts,
                bulges: any_bulge.then_some(bulges),
                curves: any_curve.then_some(curves),
            })
        }
    }
}

/// A hatch boundary path as points (object coordinates).
pub(super) struct Boundary {
    pub(super) pts: Vec<Vec2>,
    /// It had arcs, elliptic arcs or splines (sampled).
    pub(super) curved: bool,
    /// A spline edge could not be evaluated: its control points stand in for it.
    pub(super) rough: bool,
}

/// The pieces of a hatch's boundary (paths flagged not closed, 92 bit 32) joined end to end into rings
/// (docs/adr/9230): a piece goes on where one of its ends meets the chain's end, within a millionth of the pieces'
/// size, and the chain grows at both of its ends until it closes or nothing more meets it. A column hatched within
/// four paths of one line each was no ring at all and was left out (the YILTAŞ DWGs' 130 column hatches). A piece
/// nothing meets comes back with its path's index: it stays the ring its path always made, closed by its last edge.
pub(super) fn stitch(mut pieces: Vec<(usize, Vec<Vec2>)>) -> Vec<(Option<usize>, Vec<Vec2>)> {
    let (mut lo, mut hi) = (
        v(f64::INFINITY, f64::INFINITY),
        v(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in pieces.iter().flat_map(|(_, r)| r) {
        lo = v(lo.x.min(p.x), lo.y.min(p.y));
        hi = v(hi.x.max(p.x), hi.y.max(p.y));
    }
    let tol = 1e-6 * (hi.x - lo.x).max(hi.y - lo.y);
    let near = |a: Vec2, b: Vec2| (a.x - b.x).abs() <= tol && (a.y - b.y).abs() <= tol;
    let mut out = Vec::new();
    while !pieces.is_empty() {
        let (k, mut chain) = pieces.remove(0);
        let (mut joined, mut turned) = (false, false);
        loop {
            let end = chain[chain.len() - 1];
            if chain.len() > 2 && near(chain[0], end) {
                break;
            }
            match pieces
                .iter()
                .position(|(_, p)| near(p[0], end) || near(p[p.len() - 1], end))
            {
                Some(i) => {
                    let (_, mut next) = pieces.remove(i);
                    if !near(next[0], end) {
                        next.reverse();
                    }
                    chain.extend(next.into_iter().skip(1));
                    joined = true;
                }
                None if !turned => {
                    chain.reverse();
                    turned = true;
                }
                None => break,
            }
        }
        if turned {
            chain.reverse();
        }
        // A closed chain's last point is its first, met within the tolerance.
        if joined && chain.len() > 3 && near(chain[0], chain[chain.len() - 1]) {
            chain.pop();
        }
        out.push(((!joined).then_some(k), chain));
    }
    out
}

/// A boundary path's points: a polyline path exactly as the app samples its
/// own bulged rings (the shared core), edge paths with the same step.
pub(super) fn path_points(path: &Path) -> Boundary {
    match path {
        Path::Poly { pts, bulges } => {
            let pts: Vec<Vec2> = pts.iter().map(|p| v(p[0], p[1])).collect();
            Boundary {
                pts: bulge_path_points(&pts, bulges, true),
                curved: has_arcs(bulges),
                rough: false,
            }
        }
        Path::Edges(edges) => {
            let mut out: Vec<Vec2> = Vec::new();
            let mut curved = false;
            let mut rough = false;
            for edge in edges {
                let prev = out.last().copied();
                match edge {
                    Edge::Line { a, b } => {
                        out.push(v(a[0], a[1]));
                        out.push(v(b[0], b[1]));
                    }
                    Edge::Arc { c, r, a0, a1, ccw } => {
                        curved = true;
                        arc_edge(v(c[0], c[1]), *r, *a0, *a1, *ccw, prev, &mut out);
                    }
                    Edge::Ellipse {
                        c,
                        major,
                        ratio,
                        a0,
                        a1,
                        ccw,
                    } => {
                        curved = true;
                        let (a0, a1) = if *ccw { (*a0, *a1) } else { (-*a0, -*a1) };
                        // Stored angles to parameters of the ellipse.
                        let param = |a: f64| atan2(sin(rad(a)) / ratio.max(1e-12), cos(rad(a)));
                        let (t0, mut t1) = (param(a0), param(a1));
                        if *ccw {
                            if t1 <= t0 {
                                t1 += TAU;
                            }
                        } else if t1 >= t0 {
                            t1 -= TAU;
                        }
                        let (mx, my) = (major[0], major[1]);
                        let (nx, ny) = (-my * ratio, mx * ratio);
                        let n = arc_steps(t1 - t0);
                        for i in 0..=n {
                            let t = t0 + (t1 - t0) * i as f64 / n as f64;
                            out.push(v(
                                c[0] + mx * cos(t) + nx * sin(t),
                                c[1] + my * cos(t) + ny * sin(t),
                            ));
                        }
                    }
                    Edge::Spline {
                        degree,
                        knots,
                        ctrl,
                        weights,
                    } => {
                        curved = true;
                        let cps: Vec<Vec2> = ctrl.iter().map(|p| v(p[0], p[1])).collect();
                        let w = (weights.len() == cps.len()).then_some(weights.as_slice());
                        if let Some(pts) = nurbs::sample(*degree, knots, &cps, w, SAMPLE_TOLERANCE)
                        {
                            out.extend(pts);
                        } else {
                            rough = true;
                            out.extend(cps);
                        }
                    }
                }
            }
            Boundary {
                pts: out,
                curved,
                rough,
            }
        }
    }
}
