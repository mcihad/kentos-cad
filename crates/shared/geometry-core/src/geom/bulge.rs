//! Polyline arc segments in the DXF "bulge" form (`apps/web/src/model/geom/bulge.ts`):
//! segment pts[i] → pts[i+1] carries bulge = tan(θ/4), θ its included
//! angle, positive counter-clockwise; 0 is a straight segment.

use crate::api::Op;
use crate::geom::arc::{DEFAULT_STEP, circle_through, norm_angle};
use crate::geom::intersect::Edge;
use crate::geometry::{path_length, signed_area};
use crate::jsmath::{PI, atan, atan2, cos, js_hypot, js_max, or, sin, tan};
use crate::op;
use crate::vec2::Vec2;

const EPS: f64 = 1e-12;

/// Bulge of segment `i`; missing entries are straight.
pub fn bulge_at(bulges: Option<&[f64]>, i: usize) -> f64 {
    bulges.and_then(|b| b.get(i).copied()).unwrap_or(0.0)
}

pub fn is_arc_bulge(b: f64) -> bool {
    b.abs() > EPS
}

pub fn has_bulges(bulges: Option<&[f64]>) -> bool {
    bulges.is_some_and(|b| b.iter().any(|&v| is_arc_bulge(v)))
}

/// Circle, start angle and signed sweep of an arc segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BulgeArc {
    pub c: Vec2,
    pub r: f64,
    pub a0: f64,
    pub sweep: f64,
}

crate::json_struct!(BulgeArc { c, r, a0, sweep });

/// The arc of a bulged segment; `None` when straight or degenerate.
pub fn bulge_arc(a: Vec2, b: Vec2, bulge: f64) -> Option<BulgeArc> {
    if !is_arc_bulge(bulge) {
        return None;
    }
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let chord = js_hypot(dx, dy);
    if chord < EPS {
        return None;
    }
    // Centre on the chord's left normal at chord·(1−b²)/(4b); r = chord·(1+b²)/(4|b|).
    let k = (1.0 - bulge * bulge) / (4.0 * bulge);
    let c = Vec2::new((a.x + b.x) / 2.0 - dy * k, (a.y + b.y) / 2.0 + dx * k);
    let r = (chord * (1.0 + bulge * bulge)) / (4.0 * bulge.abs());
    Some(BulgeArc {
        c,
        r,
        a0: atan2(a.y - c.y, a.x - c.x),
        sweep: 4.0 * atan(bulge),
    })
}

pub fn bulge_of_sweep(sweep: f64) -> f64 {
    tan(sweep / 4.0)
}

/// Middle of a segment: the chord midpoint, or the arc's midpoint when bulged.
pub fn segment_mid(a: Vec2, b: Vec2, bulge: f64) -> Vec2 {
    Vec2::new(
        (a.x + b.x) / 2.0 + ((b.y - a.y) * bulge) / 2.0,
        (a.y + b.y) / 2.0 - ((b.x - a.x) * bulge) / 2.0,
    )
}

/// Bulge of the arc from `a` through `m` to `b`; 0 when collinear.
pub fn bulge_through(a: Vec2, m: Vec2, b: Vec2) -> f64 {
    let Some(circle) = circle_through(a, m, b) else {
        return 0.0;
    };
    let c = circle.c;
    let a_s = atan2(a.y - c.y, a.x - c.x);
    let a_m = atan2(m.y - c.y, m.x - c.x);
    let a_e = atan2(b.y - c.y, b.x - c.x);
    let ccw = norm_angle(a_m - a_s) < norm_angle(a_e - a_s);
    bulge_of_sweep(if ccw {
        norm_angle(a_e - a_s)
    } else {
        -norm_angle(a_s - a_e)
    })
}

/// Bulge of the arc leaving `a` along `dir` and ending at `b`; None when `b`
/// lies straight behind `a` (a full circle).
pub fn tangent_bulge(a: Vec2, dir: Vec2, b: Vec2) -> Option<f64> {
    let cx = b.x - a.x;
    let cy = b.y - a.y;
    if js_hypot(cx, cy) < EPS {
        return None;
    }
    let alpha = atan2(dir.x * cy - dir.y * cx, dir.x * cx + dir.y * cy);
    if PI - alpha.abs() < 1e-6 {
        return None;
    }
    // The included angle is twice the angle between tangent and chord.
    Some(tan(alpha / 2.0))
}

/// Unit travel direction at the end (`at_end`) or start of a segment.
pub fn segment_tangent(a: Vec2, b: Vec2, bulge: f64, at_end: bool) -> Vec2 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let l = or(js_hypot(dx, dy), 1.0);
    let half = 2.0 * atan(bulge); // θ/2
    let rot = if at_end { half } else { -half };
    let c = cos(rot);
    let s = sin(rot);
    Vec2::new((dx * c - dy * s) / l, (dx * s + dy * c) / l)
}

fn seg_count(n: usize, closed: bool) -> usize {
    if closed { n } else { n.saturating_sub(1) }
}

/// Primitive edges of a bulged path (arc edges keep the travel direction).
pub fn bulge_path_edges(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Vec<Edge> {
    let n = pts.len();
    (0..seg_count(n, closed))
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            match bulge_arc(a, b, bulge_at(bulges, i)) {
                Some(arc) => Edge::Arc {
                    c: arc.c,
                    r: arc.r,
                    a0: arc.a0,
                    sweep: arc.sweep,
                },
                None => Edge::Seg { a, b },
            }
        })
        .collect()
}

/// Points with arc segments tessellated (step ≤ `max_step` radians); a closed
/// path returns a ring without repeating its first point.
pub fn bulge_path_outline(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    max_step: f64,
) -> Vec<Vec2> {
    let n = pts.len();
    if !has_bulges(bulges) {
        return pts.to_vec();
    }
    let mut out = Vec::new();
    for i in 0..seg_count(n, closed) {
        let a = pts[i];
        out.push(a);
        let Some(arc) = bulge_arc(a, pts[(i + 1) % n], bulge_at(bulges, i)) else {
            continue;
        };
        let steps = js_max(2.0, (arc.sweep.abs() / max_step).ceil());
        let mut k = 1.0;
        while k < steps {
            let t = arc.a0 + (arc.sweep * k) / steps;
            out.push(Vec2::new(
                arc.c.x + cos(t) * arc.r,
                arc.c.y + sin(t) * arc.r,
            ));
            k += 1.0;
        }
    }
    if !closed && n > 0 {
        out.push(pts[n - 1]);
    }
    out
}

/// Length along the path, arcs measured exactly.
pub fn bulge_path_length(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> f64 {
    if !has_bulges(bulges) {
        return path_length(pts, closed);
    }
    let n = pts.len();
    let mut l = 0.0;
    for i in 0..seg_count(n, closed) {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        l += match bulge_arc(a, b, bulge_at(bulges, i)) {
            Some(arc) => arc.r * arc.sweep.abs(),
            None => js_hypot(b.x - a.x, b.y - a.y),
        };
    }
    l
}

/// Signed area of a closed bulged ring: shoelace plus each arc's circular segment.
pub fn bulge_ring_area(pts: &[Vec2], bulges: Option<&[f64]>) -> f64 {
    let mut area = signed_area(pts);
    if !has_bulges(bulges) {
        return area;
    }
    let n = pts.len();
    for i in 0..n {
        if let Some(arc) = bulge_arc(pts[i], pts[(i + 1) % n], bulge_at(bulges, i)) {
            area += ((arc.r * arc.r) / 2.0) * (arc.sweep - sin(arc.sweep));
        }
    }
    area
}

/// A path with its bulges (`reverseBulgePath`, `cleanBulgePath`).
#[derive(Clone, Debug, PartialEq)]
pub struct BulgePath {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
}

crate::json_struct!(BulgePath { pts, bulges });

/// Path reversed: vertex order flips and every arc turns the other way.
pub fn reverse_bulge_path(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> BulgePath {
    let n = pts.len();
    let rp: Vec<Vec2> = pts.iter().rev().copied().collect();
    let mut rb = vec![0.0; n];
    if closed {
        // Segment i of the reversed ring runs pts[n-1-i] → pts[n-2-i].
        for (i, b) in rb.iter_mut().enumerate() {
            *b = -bulge_at(bulges, (2 * n - 2 - i) % n);
        }
    } else {
        for i in 0..n.saturating_sub(1) {
            rb[i] = -bulge_at(bulges, n - 2 - i);
        }
    }
    BulgePath {
        pts: rp,
        bulges: Some(rb),
    }
}

/// Drops zero-length segments, keeping the surviving segment's bulge;
/// bulges only when one is an arc.
pub fn clean_bulge_path(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, tol: f64) -> BulgePath {
    let mut out_p: Vec<Vec2> = Vec::new();
    let mut out_b: Vec<f64> = Vec::new();
    for (i, &p) in pts.iter().enumerate() {
        if let Some(&last) = out_p.last()
            && js_hypot(p.x - last.x, p.y - last.y) <= tol
        {
            if let Some(b) = out_b.last_mut() {
                *b = bulge_at(bulges, i);
            }
            continue;
        }
        out_p.push(p);
        out_b.push(bulge_at(bulges, i));
    }
    if closed && out_p.len() > 1 {
        let f = out_p[0];
        let l = out_p[out_p.len() - 1];
        if js_hypot(f.x - l.x, f.y - l.y) <= tol {
            out_p.pop();
            out_b.pop();
        }
    }
    if !closed && let Some(b) = out_b.last_mut() {
        *b = 0.0;
    }
    if has_bulges(Some(&out_b)) {
        BulgePath {
            pts: out_p,
            bulges: Some(out_b),
        }
    } else {
        BulgePath {
            pts: out_p,
            bulges: None,
        }
    }
}

type Bulges = Option<Vec<f64>>;

pub(crate) static OPS: &[Op] = &[
    op!("bulgeAt", |b: Bulges, i: usize| bulge_at(b.as_deref(), i)),
    op!("isArcBulge", |b: f64| is_arc_bulge(b)),
    op!("hasBulges", |b: Bulges| has_bulges(b.as_deref())),
    op!("bulgeArc", |a: Vec2, b: Vec2, bulge: f64| bulge_arc(
        a, b, bulge
    )),
    op!("bulgeOfSweep", |s: f64| bulge_of_sweep(s)),
    op!("segmentMid", |a: Vec2, b: Vec2, bulge: f64| segment_mid(
        a, b, bulge
    )),
    op!("bulgeThrough", |a: Vec2, m: Vec2, b: Vec2| {
        bulge_through(a, m, b)
    }),
    op!("tangentBulge", |a: Vec2, dir: Vec2, b: Vec2| {
        tangent_bulge(a, dir, b)
    }),
    op!("segmentTangent", |a: Vec2,
                           b: Vec2,
                           bulge: f64,
                           at_end: bool| {
        segment_tangent(a, b, bulge, at_end)
    }),
    op!("bulgePathEdges", |pts: Vec<Vec2>,
                           b: Bulges,
                           closed: bool| {
        bulge_path_edges(&pts, b.as_deref(), closed)
    }),
    op!(
        "bulgePathOutline",
        |pts: Vec<Vec2>, b: Bulges, closed: bool, step: Option<f64>| bulge_path_outline(
            &pts,
            b.as_deref(),
            closed,
            step.unwrap_or(DEFAULT_STEP)
        )
    ),
    op!(
        "bulgePathLength",
        |pts: Vec<Vec2>, b: Bulges, closed: bool| bulge_path_length(&pts, b.as_deref(), closed)
    ),
    op!("bulgeRingArea", |pts: Vec<Vec2>, b: Bulges| {
        bulge_ring_area(&pts, b.as_deref())
    }),
    op!(
        "reverseBulgePath",
        |pts: Vec<Vec2>, b: Bulges, closed: bool| reverse_bulge_path(&pts, b.as_deref(), closed)
    ),
    op!(
        "cleanBulgePath",
        |pts: Vec<Vec2>, b: Bulges, closed: bool, tol: Option<f64>| clean_bulge_path(
            &pts,
            b.as_deref(),
            closed,
            tol.unwrap_or(1e-9)
        )
    ),
];
