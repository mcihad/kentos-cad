//! Fits cubic Béziers through a polyline within a tolerance
//! (`apps/web/src/style/svg/fitCurve.ts`; Schneider, "An Algorithm for
//! Automatically Fitting Digitized Curves", Graphics Gems 1990):
//! chord-length parameters, least-squares handles along fixed end tangents,
//! Newton re-parameterisation, and a split at the worst point when one
//! curve is not enough. Straight runs stay lines and sharp turns stay
//! corners. Simplify, stroke to path and offsets use it.

use kentos_geometry_core::jsmath::{PI, cos, js_floor, js_hypot, js_max, js_min, or};

use crate::bezier::{Cubic, bez, bez_deriv, bez_deriv2};
use crate::shape::{PathNode, Pt, SubPath};

fn sub(a: Pt, b: Pt) -> Pt {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: Pt, b: Pt) -> Pt {
    [a[0] + b[0], a[1] + b[1]]
}
fn mul(a: Pt, k: f64) -> Pt {
    [a[0] * k, a[1] * k]
}
fn dot(a: Pt, b: Pt) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn len(a: Pt) -> f64 {
    js_hypot(a[0], a[1])
}
fn unit(a: Pt) -> Pt {
    let l = len(a);
    if l > 1e-15 {
        [a[0] / l, a[1] / l]
    } else {
        [0.0, 0.0]
    }
}

/// Distance from p to the segment a–b.
pub fn dist_to_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        js_max(
            0.0,
            js_min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2),
        )
    } else {
        0.0
    };
    js_hypot(p[0] - a[0] - dx * t, p[1] - a[1] - dy * t)
}

fn chord_params(d: &[Pt], first: usize, last: usize) -> Vec<f64> {
    let mut u = vec![0.0];
    for i in first + 1..=last {
        let prev = u[u.len() - 1];
        u.push(prev + len(sub(d[i], d[i - 1])));
    }
    let total = or(u[u.len() - 1], 1.0);
    u.iter().map(|v| v / total).collect()
}

fn generate(d: &[Pt], first: usize, last: usize, u: &[f64], t1: Pt, t2: Pt) -> Cubic {
    let p0 = d[first];
    let p3 = d[last];
    let mut c00 = 0.0;
    let mut c01 = 0.0;
    let mut c11 = 0.0;
    let mut x0 = 0.0;
    let mut x1 = 0.0;
    for (i, &t) in u.iter().enumerate() {
        let s = 1.0 - t;
        let b0 = s * s * s;
        let b1 = 3.0 * t * s * s;
        let b2 = 3.0 * t * t * s;
        let b3 = t * t * t;
        let a1 = mul(t1, b1);
        let a2 = mul(t2, b2);
        c00 += dot(a1, a1);
        c01 += dot(a1, a2);
        c11 += dot(a2, a2);
        let tmp = sub(d[first + i], add(mul(p0, b0 + b1), mul(p3, b2 + b3)));
        x0 += dot(a1, tmp);
        x1 += dot(a2, tmp);
    }
    let det = c00 * c11 - c01 * c01;
    let mut alpha1 = if det != 0.0 {
        (x0 * c11 - x1 * c01) / det
    } else {
        0.0
    };
    let mut alpha2 = if det != 0.0 {
        (c00 * x1 - c01 * x0) / det
    } else {
        0.0
    };
    let seg_len = len(sub(p3, p0));
    let eps = 1e-6 * seg_len;
    if alpha1 < eps || alpha2 < eps {
        // Fall back to the Wu/Barsky heuristic: a third of the chord.
        alpha1 = seg_len / 3.0;
        alpha2 = alpha1;
    }
    [p0, add(p0, mul(t1, alpha1)), add(p3, mul(t2, alpha2)), p3]
}

fn max_error(d: &[Pt], first: usize, last: usize, c: &Cubic, u: &[f64]) -> (f64, usize) {
    let mut err = 0.0;
    let mut at = js_floor((first + last) as f64 / 2.0) as usize;
    for i in first + 1..last {
        let p = bez(c, u[i - first]);
        let e = (p[0] - d[i][0]) * (p[0] - d[i][0]) + (p[1] - d[i][1]) * (p[1] - d[i][1]);
        if e >= err {
            err = e;
            at = i;
        }
    }
    (err, at)
}

fn reparameterize(d: &[Pt], first: usize, c: &Cubic, u: &[f64]) -> Vec<f64> {
    u.iter()
        .enumerate()
        .map(|(i, &t)| {
            let p = d[first + i];
            let q = bez(c, t);
            let q1 = bez_deriv(c, t);
            let q2 = bez_deriv2(c, t);
            let num = (q[0] - p[0]) * q1[0] + (q[1] - p[1]) * q1[1];
            let den = q1[0] * q1[0] + q1[1] * q1[1] + (q[0] - p[0]) * q2[0] + (q[1] - p[1]) * q2[1];
            if den.abs() < 1e-18 {
                return t;
            }
            js_min(1.0, js_max(0.0, t - num / den))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn fit_range(
    d: &[Pt],
    first: usize,
    last: usize,
    t1: Pt,
    t2: Pt,
    tol2: f64,
    out: &mut Vec<Cubic>,
    depth: u32,
) {
    if last - first == 1 || depth > 40 {
        let dist = len(sub(d[last], d[first])) / 3.0;
        out.push([
            d[first],
            add(d[first], mul(t1, dist)),
            add(d[last], mul(t2, dist)),
            d[last],
        ]);
        return;
    }
    let mut u = chord_params(d, first, last);
    let mut c = generate(d, first, last, &u, t1, t2);
    let (mut err, mut at) = max_error(d, first, last, &c, &u);
    if err < tol2 {
        out.push(c);
        return;
    }
    if err < tol2 * 16.0 {
        for _ in 0..12 {
            u = reparameterize(d, first, &c, &u);
            c = generate(d, first, last, &u, t1, t2);
            (err, at) = max_error(d, first, last, &c, &u);
            if err < tol2 {
                out.push(c);
                return;
            }
        }
    }
    // Far off (a long smooth run): halve it, which keeps the pieces even;
    // close: split at the worst point, where the detail is.
    if err > tol2 * 256.0 {
        at = js_floor((first + last) as f64 / 2.0) as usize;
    }
    if at <= first {
        at = first + 1;
    }
    if at >= last {
        at = last - 1;
    }
    let centre = unit(sub(d[at - 1], d[at + 1]));
    let tc = if len(centre) > 0.0 {
        centre
    } else {
        unit(sub(d[at - 1], d[at]))
    };
    fit_range(d, first, at, t1, tc, tol2, out, depth + 1);
    fit_range(d, at, last, [-tc[0], -tc[1]], t2, tol2, out, depth + 1);
}

/// One run of points as cubics (`None`: a line, when the run is straight within `tol`).
pub fn fit_run(pts: &[Pt], tol: f64, t1: Option<Pt>, t2: Option<Pt>) -> Vec<Option<Cubic>> {
    let d = dedupe(pts);
    if d.len() < 2 {
        return Vec::new();
    }
    let a = d[0];
    let b = d[d.len() - 1];
    if d.iter().all(|&p| dist_to_segment(p, a, b) <= tol) {
        return vec![None];
    }
    let mut out = Vec::new();
    let s1 = t1.unwrap_or_else(|| unit(sub(d[1], d[0])));
    let s2 = t2.unwrap_or_else(|| unit(sub(d[d.len() - 2], d[d.len() - 1])));
    fit_range(&d, 0, d.len() - 1, s1, s2, tol * tol, &mut out, 0);
    out.into_iter().map(Some).collect()
}

/// The one cubic that best fits the points with these end tangents (node deletion keeping the shape).
pub fn fit_one(pts: &[Pt], t1: Pt, t2: Pt) -> Option<Cubic> {
    let d = dedupe(pts);
    if d.len() < 2 {
        return None;
    }
    let last = d.len() - 1;
    let mut u = chord_params(&d, 0, last);
    let mut c = generate(&d, 0, last, &u, t1, t2);
    for _ in 0..8 {
        u = reparameterize(&d, 0, &c, &u);
        c = generate(&d, 0, last, &u, t1, t2);
    }
    Some(c)
}

fn dedupe(pts: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    for &p in pts {
        match out.last() {
            Some(q) if !(js_hypot(p[0] - q[0], p[1] - q[1]) > 1e-12) => {}
            _ => out.push(p),
        }
    }
    out
}

/// Indices of points where the polyline turns by more than `deg` (ends of an open line always).
pub fn corners_of(pts: &[Pt], closed: bool, deg: f64) -> Vec<usize> {
    let n = pts.len();
    let limit = cos((deg * PI) / 180.0);
    let mut out = Vec::new();
    for i in 0..n {
        if !closed && (i == 0 || i == n - 1) {
            out.push(i);
            continue;
        }
        let a = pts[(i + n - 1) % n];
        let b = pts[i];
        let c = pts[(i + 1) % n];
        let u = unit(sub(b, a));
        let v = unit(sub(c, b));
        if dot(u, v) < limit {
            out.push(i);
        }
    }
    out
}

/// How a polyline is fitted.
pub struct FitOptions<'a> {
    pub closed: bool,
    /// Points that must stay corners (nodes of the source), besides sharp turns.
    pub corners: &'a [f64],
    /// Turns sharper than this (degrees) are corners.
    pub corner_deg: f64,
}

/// A polyline as a sub-path of fitted curves: split at corners, each run
/// fitted within `tol`. A closed polyline without corners is fitted as one
/// smooth loop (the seam's tangents match).
pub fn fit_polyline(input: &[Pt], tol: f64, opts: &FitOptions) -> SubPath {
    // Repeated points go; the caller's corner indices follow the kept ones.
    let mut pts: Vec<Pt> = Vec::new();
    let mut kept_at: Vec<isize> = Vec::with_capacity(input.len());
    for &p in input {
        match pts.last() {
            Some(q) if !(js_hypot(p[0] - q[0], p[1] - q[1]) > 1e-12) => {}
            _ => pts.push(p),
        }
        kept_at.push(pts.len() as isize - 1);
    }
    if opts.closed && pts.len() > 2 {
        let a = pts[0];
        let b = pts[pts.len() - 1];
        if js_hypot(a[0] - b[0], a[1] - b[1]) <= 1e-12 {
            pts.pop();
            let n = pts.len() as isize;
            for k in kept_at.iter_mut() {
                if *k == n {
                    *k = 0;
                }
            }
        }
    }
    let n = pts.len();
    if n < 2 {
        return SubPath {
            closed: opts.closed,
            nodes: pts.iter().map(|p| PathNode::at(p[0], p[1])).collect(),
        };
    }
    // `new Set([...corners.map(i => keptAt[i]), ...cornersOf(…)])`, then in range and sorted.
    let mut set: Vec<f64> = Vec::new();
    let mut put = |v: f64| {
        // A Set tells values apart as SameValueZero does (NaN is one value).
        if !set.iter().any(|&w| w == v || (w.is_nan() && v.is_nan())) {
            set.push(v);
        }
    };
    for &i in opts.corners {
        // `keptAt[i]`: an index past the end (or not a whole number) is undefined.
        let at = if i >= 0.0 && i.fract() == 0.0 && (i as usize) < kept_at.len() {
            kept_at[i as usize] as f64
        } else {
            f64::NAN
        };
        put(at);
    }
    for i in corners_of(&pts, opts.closed, opts.corner_deg) {
        put(i as f64);
    }
    let mut corners: Vec<usize> = Vec::new();
    for v in set {
        if v >= 0.0 && v < n as f64 {
            corners.push(v as usize);
        }
    }
    corners.sort_unstable();
    let at = |i: isize| -> Pt {
        let m = n as isize;
        pts[(((i % m) + m) % m) as usize]
    };
    struct Piece {
        to: isize,
        curves: Vec<Option<Cubic>>,
    }
    let mut pieces: Vec<Piece> = Vec::new();
    let mut first: isize = 0;
    if opts.closed && corners.is_empty() {
        // A smooth loop: start at 0 with the tangent across the seam.
        let t = unit(sub(at(1), at(-1)));
        let mut ring = pts.clone();
        ring.push(pts[0]);
        pieces.push(Piece {
            to: n as isize,
            curves: fit_run(&ring, tol, Some(t), Some([-t[0], -t[1]])),
        });
    } else {
        let cs: Vec<isize> = if opts.closed || corners.contains(&0) {
            corners.iter().map(|&c| c as isize).collect()
        } else {
            std::iter::once(0)
                .chain(corners.iter().map(|&c| c as isize))
                .collect()
        };
        first = cs[0];
        let count = if opts.closed { cs.len() } else { cs.len() - 1 };
        for k in 0..count {
            let i0 = cs[k];
            let i1 = if opts.closed && k == cs.len() - 1 {
                cs[0] + n as isize
            } else {
                cs[k + 1]
            };
            let run: Vec<Pt> = (i0..=i1).map(at).collect();
            pieces.push(Piece {
                to: i1,
                curves: fit_run(&run, tol, None, None),
            });
        }
    }
    let p0 = at(first);
    let mut nodes: Vec<PathNode> = vec![PathNode::at(p0[0], p0[1])];
    for piece in &pieces {
        for c in &piece.curves {
            match c {
                Some(c) => {
                    if let Some(cur) = nodes.last_mut() {
                        cur.out = Some([c[1][0], c[1][1]]);
                    }
                    nodes.push(PathNode {
                        in_: Some([c[2][0], c[2][1]]),
                        ..PathNode::at(c[3][0], c[3][1])
                    });
                }
                None => {
                    let p = at(piece.to);
                    nodes.push(PathNode::at(p[0], p[1]));
                }
            }
        }
    }
    if opts.closed && nodes.len() > 1 {
        // The walk came back to the first node: its incoming handle moves there.
        if let Some(last) = nodes.pop()
            && let Some(h) = last.in_
        {
            nodes[0].in_ = Some(h);
        }
    }
    SubPath {
        closed: opts.closed,
        nodes,
    }
}
