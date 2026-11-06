//! A CAD line type's dash measured along the source curve (ADR 1122). The
//! drawn carrier keeps its own points; each vertex gets its true distance on
//! the arc, ellipse, bulge path or NURBS it lies on, and the pattern is cut at
//! those distances. Every ink run is a solid stroke on the carrier itself, so
//! the GPU, the CPU raster and the PDF draw the same pieces with the same
//! phase across segments, vertices and a closed path's seam. A dot keeps the
//! renderers' own dot rule: a two-point stroke whose only dash is that dot.
use super::{
    cad_marks::Route,
    prim::{Sink, StrokeStyle},
};
use kentos_geometry_core::{
    Vec2,
    geom::ellipse::param_of_point,
    jsmath::{atan2, cos, js_hypot, js_max, js_min, js_sign, sin},
};
use std::f64::consts::TAU;

/// The most ink runs and dots one path may become; past it the renderer's
/// chord-measured dash stays (the earlier output, ADR 1122 §4).
const MAX_PIECES: usize = 20_000;

/// The share of a sweep a parameter may run past an end and still be that end.
fn along_sweep(angle: f64, start: f64, sweep: f64) -> f64 {
    let d = ((angle - start) * js_sign(sweep)).rem_euclid(TAU);
    let s = sweep.abs();
    if d <= s {
        d / s
    } else if d - s < TAU - d {
        1.
    } else {
        0.
    }
}

/// Each carrier vertex's distance on the route, non-decreasing; none when a
/// vertex is not on it (another carrier) or the order does not hold.
pub(super) fn distances(route: &Route, path: &[Vec2]) -> Option<Vec<f64>> {
    let total = route.total;
    let far = 1e-3 * (total + 1.);
    let mut out = Vec::with_capacity(path.len());
    if let Some(e) = &route.ellipse {
        let sweep = kentos_geometry_core::geom::ellipse::ellipse_sweep(&e.geom);
        for p in path {
            let f = along_sweep(param_of_point(&e.geom, *p), e.geom.t0, sweep);
            out.push(e.distance(e.geom.t0 + sweep * f));
        }
    } else if let Some(n) = &route.nurbs {
        let c = n.curve.view();
        let (lo, hi) = c.domain();
        let mut u_prev = lo;
        let mut at = 0.;
        for (i, p) in path.iter().enumerate() {
            let mut u = c.closest(*p);
            if i == 0 && js_hypot(p.x - c.point(lo).x, p.y - c.point(lo).y) <= far {
                u = lo;
            }
            // A closed curve's last vertex is its first point: the end, not the start again.
            if u < u_prev && i + 1 == path.len() && js_hypot(p.x - c.point(hi).x, p.y - c.point(hi).y) <= far {
                u = hi;
            }
            let q = c.point(u);
            if u < u_prev || js_hypot(q.x - p.x, q.y - p.y) > far {
                return None;
            }
            at += c.length_between(u_prev, u);
            u_prev = u;
            out.push(at);
        }
    } else {
        let edges = &route.edges;
        if edges.is_empty() {
            return None;
        }
        let place = |k: usize, p: Vec2| -> (f64, f64) {
            let e = &edges[k];
            let f = match e.arc {
                Some(a) => along_sweep(atan2(p.y - a.c.y, p.x - a.c.x), a.a0, a.sweep),
                None => {
                    let (dx, dy) = (e.b.x - e.a.x, e.b.y - e.a.y);
                    (((p.x - e.a.x) * dx + (p.y - e.a.y) * dy) / (dx * dx + dy * dy)).clamp(0., 1.)
                }
            };
            let q = match e.arc {
                Some(a) => {
                    let t = a.a0 + a.sweep * f;
                    Vec2::new(a.c.x + a.r * cos(t), a.c.y + a.r * sin(t))
                }
                None => Vec2::new(e.a.x + (e.b.x - e.a.x) * f, e.a.y + (e.b.y - e.a.y) * f),
            };
            (e.start + e.length * f, js_hypot(q.x - p.x, q.y - p.y))
        };
        let mut k = 0;
        for (i, p) in path.iter().enumerate() {
            let (mut d, mut err) = place(k, *p);
            // Move on while the next edge holds the vertex at least as closely.
            while k + 1 < edges.len() {
                let (dn, en) = place(k + 1, *p);
                if en < err || (en == err && dn >= d && i > 0 && d >= edges[k].start + edges[k].length) {
                    k += 1;
                    (d, err) = (dn, en);
                } else {
                    break;
                }
            }
            if err > far {
                return None;
            }
            out.push(d);
        }
    }
    // A whole circle's, ellipse's or ring's start may read as its end, its
    // closing vertex as its start again: the seam, not a step back.
    let n = out.len();
    if n >= 2 && out[0] - out[1] > 0.5 * total {
        out[0] = 0.;
    }
    if n >= 2 && out[n - 2] - out[n - 1] > 0.5 * total {
        out[n - 1] = total;
    }
    let ordered = out
        .windows(2)
        .all(|w| w[1] >= w[0] - 1e-9 * js_max(total, 1.));
    (ordered && out.iter().all(|d| d.is_finite())).then(|| {
        let mut top = 0.;
        out.into_iter()
            .map(|d| {
                top = js_max(top, js_min(d, total));
                top
            })
            .collect()
    })
}

/// Ink runs and dots (point, unit direction) of `dash` (world lengths, a dash
/// first) whose pattern position at the path's start is `offset`, the same rule
/// as a stroke's `dash_offset` (PDF's phase). A cut inside a chord is the
/// curve's own point at that distance (`exact`); a cut at a vertex is the vertex.
pub(super) struct Ink {
    pub(super) pieces: Vec<Vec<Vec2>>,
    pub(super) dots: Vec<(Vec2, Vec2)>,
}

fn lerp(a: Vec2, b: Vec2, f: f64) -> Vec2 {
    if f <= 0. {
        a
    } else if f >= 1. {
        b
    } else {
        Vec2::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f)
    }
}

pub(super) fn ink(
    path: &[Vec2],
    dist: &[f64],
    closed: bool,
    total: f64,
    dash: &[f64],
    offset: f64,
    exact: &dyn Fn(f64) -> Option<Vec2>,
) -> Option<Ink> {
    let cut = |a: Vec2, b: Vec2, s0: f64, s1: f64, t: f64| {
        let f = (t - s0) / (s1 - s0);
        if f <= 0. {
            a
        } else if f >= 1. {
            b
        } else {
            exact(t).unwrap_or_else(|| lerp(a, b, f))
        }
    };
    if dash.is_empty()
        || dash.iter().any(|d| !d.is_finite() || *d < 0.)
        || !offset.is_finite()
        || path.len() < 2
        || dist.len() != path.len()
    {
        return None;
    }
    let mut dash = dash.to_vec();
    if dash.len() % 2 == 1 {
        dash.extend_from_within(..);
    }
    let period: f64 = dash.iter().sum();
    if !period.is_finite() || period <= 0. || !total.is_finite() || total <= 0. {
        return None;
    }
    if (total / period + 2.) * dash.len() as f64 > MAX_PIECES as f64 {
        return None;
    }
    let mut segments: Vec<(Vec2, Vec2, f64, f64)> = path
        .windows(2)
        .zip(dist.windows(2))
        .map(|(p, d)| (p[0], p[1], d[0], d[1]))
        .collect();
    if closed {
        segments.push((path[path.len() - 1], path[0], dist[dist.len() - 1], total));
    }
    let end = segments.last().map_or(0., |s| s.3);
    let start = dist[0];
    // The pattern at the first vertex's own distance (a clipped part starts inside the curve).
    let mut u = (start + offset).rem_euclid(period);
    let mut di = 0;
    while di + 1 < dash.len() && u >= dash[di] {
        u -= dash[di];
        di += 1;
    }
    let mut left = dash[di] - u;
    let mut pieces = Vec::new();
    let mut piece: Vec<Vec2> = Vec::new();
    for &(a, b, s0, s1) in &segments {
        let len = s1 - s0;
        if len <= 0. {
            continue;
        }
        let mut at = 0.;
        while at < len {
            if left <= 0. {
                di = (di + 1) % dash.len();
                left = dash[di];
                continue;
            }
            let amount = js_min(left, len - at);
            let to = at + amount;
            if to <= at {
                return None;
            }
            if di % 2 == 0 {
                if piece.is_empty() {
                    piece.push(cut(a, b, s0, s1, s0 + at));
                }
                piece.push(if to >= len { b } else { cut(a, b, s0, s1, s0 + to) });
            }
            at = to;
            left -= amount;
            if left == 0. {
                let was_on = di % 2 == 0;
                di = (di + 1) % dash.len();
                left = dash[di];
                while left == 0. {
                    di = (di + 1) % dash.len();
                    left = dash[di];
                }
                if was_on && di % 2 == 1 && !piece.is_empty() {
                    pieces.push(std::mem::take(&mut piece));
                }
            }
        }
    }
    if piece.len() >= 2 {
        pieces.push(piece);
    }
    // A ring whose ink runs through its seam is one run.
    if closed
        && pieces.len() > 1
        && pieces[0].first() == path.first()
        && pieces.last().and_then(|p| p.last()) == path.first()
    {
        let first = pieces.remove(0);
        if let Some(last) = pieces.last_mut() {
            last.extend(first.into_iter().skip(1));
        }
    }
    // Dots at their pattern places, from the distances alone.
    let mut dots = Vec::new();
    let mut s = 0.;
    for (i, d) in dash.iter().enumerate() {
        if i % 2 == 0 && *d == 0. {
            let first = start + (s - offset - start).rem_euclid(period);
            let slack = period * 1e-9;
            for k in 0u32.. {
                let t = first + period * f64::from(k);
                if t >= end + if closed { -slack } else { slack } {
                    break;
                }
                let t = js_min(t, end);
                let seg = segments
                    .iter()
                    .position(|g| t <= g.3 && g.3 > g.2)
                    .or_else(|| segments.iter().rposition(|g| g.3 > g.2))?;
                let (a, b, s0, s1) = segments[seg];
                let len = js_hypot(b.x - a.x, b.y - a.y);
                if len > 0. {
                    dots.push((
                        cut(a, b, s0, s1, t),
                        Vec2::new((b.x - a.x) / len, (b.y - a.y) / len),
                    ));
                }
            }
        }
        s += d;
    }
    (pieces.len() + dots.len() <= MAX_PIECES).then_some(Ink { pieces, dots })
}

/// The stroke as its analytic ink, or false (nothing emitted) when the
/// carrier is not the route's or the pattern is past the budget.
pub(super) fn emit(
    route: &Route,
    path: &[Vec2],
    closed: bool,
    style: &StrokeStyle,
    sink: &mut dyn Sink,
) -> bool {
    let Some(dash) = style.dash.as_ref().filter(|d| !d.is_empty()) else {
        return false;
    };
    let Some(dist) = distances(route, path) else {
        return false;
    };
    let exact = |t: f64| route.point_at(t);
    let Some(ink) = ink(path, &dist, closed, route.total, dash, style.dash_offset, &exact) else {
        return false;
    };
    let mut solid = style.clone();
    solid.dash = None;
    solid.dash_offset = 0.;
    for p in &ink.pieces {
        sink.stroke(&solid, p, false);
    }
    let period: f64 = dash.iter().sum();
    let h = period * 1e-3;
    // The dot's pattern is a whole period long (ADR 1124): its one dot at the stroke's middle, and a pattern the
    // renderers measure in pixels as the line's own, so the GPU and the CPU draw the round dot of the line's weight
    // wherever the line's dashes are drawn, and PDF paints it round whatever the line's ends.
    for (p, d) in &ink.dots {
        let mut dot = style.clone();
        dot.dash = Some(vec![0., period]);
        dot.dash_offset = period - h;
        sink.stroke(
            &dot,
            &[
                Vec2::new(p.x - d.x * h, p.y - d.y * h),
                Vec2::new(p.x + d.x * h, p.y + d.y * h),
            ],
            false,
        );
    }
    true
}

#[cfg(test)]
#[path = "cad_dash_tests.rs"]
mod tests;
