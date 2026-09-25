//! Cubic Bézier helpers for the SVG editor's path operations
//! (`apps/web/src/style/svg/bezier.ts`): segments of a sub-path, points and
//! tangents, de Casteljau splitting, flattening to a tolerance, lengths and
//! nearest points. A straight segment is a cubic whose controls sit on its
//! ends, so every segment has one form.

use kentos_geometry_core::jsmath::{js_hypot, js_max, js_min, or};

use crate::shape::{PathNode, Pt, SubPath};

pub type Cubic = [Pt; 4];

/// Segment i of a sub-path runs from node i to node i+1 (the last one of a closed sub-path back to node 0).
pub fn segment_count(sp: &SubPath) -> usize {
    let n = sp.nodes.len();
    if n < 2 {
        return if sp.closed && n == 1 && (sp.nodes[0].in_.is_some() || sp.nodes[0].out.is_some()) {
            1
        } else {
            0
        };
    }
    if sp.closed { n } else { n - 1 }
}

pub fn is_line_seg(a: &PathNode, b: &PathNode) -> bool {
    a.out.is_none() && b.in_.is_none()
}

/// The cubic of segment i (controls on the ends for a straight segment).
pub fn segment_cubic(sp: &SubPath, i: usize) -> Cubic {
    let a = &sp.nodes[i];
    let b = &sp.nodes[(i + 1) % sp.nodes.len()];
    [
        [a.x, a.y],
        a.out.unwrap_or([a.x, a.y]),
        b.in_.unwrap_or([b.x, b.y]),
        [b.x, b.y],
    ]
}

pub fn segment_is_line(sp: &SubPath, i: usize) -> bool {
    is_line_seg(&sp.nodes[i], &sp.nodes[(i + 1) % sp.nodes.len()])
}

pub fn bez(c: &Cubic, t: f64) -> Pt {
    let u = 1.0 - t;
    let a = u * u * u;
    let b = 3.0 * u * u * t;
    let d = 3.0 * u * t * t;
    let e = t * t * t;
    [
        a * c[0][0] + b * c[1][0] + d * c[2][0] + e * c[3][0],
        a * c[0][1] + b * c[1][1] + d * c[2][1] + e * c[3][1],
    ]
}

/// First derivative at t.
pub fn bez_deriv(c: &Cubic, t: f64) -> Pt {
    let u = 1.0 - t;
    let a = 3.0 * u * u;
    let b = 6.0 * u * t;
    let d = 3.0 * t * t;
    [
        a * (c[1][0] - c[0][0]) + b * (c[2][0] - c[1][0]) + d * (c[3][0] - c[2][0]),
        a * (c[1][1] - c[0][1]) + b * (c[2][1] - c[1][1]) + d * (c[3][1] - c[2][1]),
    ]
}

/// Second derivative at t (the Newton steps of nearest points and fits).
pub fn bez_deriv2(c: &Cubic, t: f64) -> Pt {
    let u = 1.0 - t;
    [
        6.0 * u * (c[2][0] - 2.0 * c[1][0] + c[0][0])
            + 6.0 * t * (c[3][0] - 2.0 * c[2][0] + c[1][0]),
        6.0 * u * (c[2][1] - 2.0 * c[1][1] + c[0][1])
            + 6.0 * t * (c[3][1] - 2.0 * c[2][1] + c[1][1]),
    ]
}

/// Unit tangent at t. At an end whose control coincides with it the first
/// derivative vanishes; the direction then comes from the next control.
pub fn bez_tangent(c: &Cubic, t: f64) -> Pt {
    let mut d = bez_deriv(c, t);
    if js_hypot(d[0], d[1]) < 1e-12 {
        let q = if t < 0.5 {
            if js_hypot(c[2][0] - c[0][0], c[2][1] - c[0][1]) > 1e-12 {
                c[2]
            } else {
                c[3]
            }
        } else if js_hypot(c[3][0] - c[1][0], c[3][1] - c[1][1]) > 1e-12 {
            c[1]
        } else {
            c[0]
        };
        let p = if t < 0.5 { c[0] } else { c[3] };
        d = if t < 0.5 {
            [q[0] - p[0], q[1] - p[1]]
        } else {
            [p[0] - q[0], p[1] - q[1]]
        };
    }
    let l = or(js_hypot(d[0], d[1]), 1.0);
    [d[0] / l, d[1] / l]
}

fn lerp(a: Pt, b: Pt, t: f64) -> Pt {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// de Casteljau split at t: the part before and the part after.
pub fn split_cubic(c: &Cubic, t: f64) -> (Cubic, Cubic) {
    let q0 = lerp(c[0], c[1], t);
    let q1 = lerp(c[1], c[2], t);
    let q2 = lerp(c[2], c[3], t);
    let r0 = lerp(q0, q1, t);
    let r1 = lerp(q1, q2, t);
    let s = lerp(r0, r1, t);
    ([c[0], q0, r0, s], [s, r1, q2, c[3]])
}

/// The part of the curve between t0 and t1 (t0 > t1 gives it reversed).
pub fn sub_cubic(c: &Cubic, t0: f64, t1: f64) -> Cubic {
    if t0 > t1 {
        return reverse_cubic(&sub_cubic(c, t1, t0));
    }
    if t0 <= 0.0 && t1 >= 1.0 {
        return *c;
    }
    let right = if t0 > 0.0 { split_cubic(c, t0).1 } else { *c };
    if t1 >= 1.0 {
        return right;
    }
    let u = (t1 - t0) / (1.0 - t0);
    split_cubic(&right, u).0
}

pub fn reverse_cubic(c: &Cubic) -> Cubic {
    [c[3], c[2], c[1], c[0]]
}

/// Number of equal parameter steps that keep the chords within `tol` of the
/// curve (Wang's bound for cubics), capped for degenerate input.
pub fn flatten_steps(c: &Cubic, tol: f64, max: f64) -> f64 {
    let m = js_max(
        js_hypot(
            c[0][0] - 2.0 * c[1][0] + c[2][0],
            c[0][1] - 2.0 * c[1][1] + c[2][1],
        ),
        js_hypot(
            c[1][0] - 2.0 * c[2][0] + c[3][0],
            c[1][1] - 2.0 * c[2][1] + c[3][1],
        ),
    );
    if !(m > 0.0) || !(tol > 0.0) {
        return 1.0;
    }
    js_max(1.0, js_min(max, ((0.75 * m) / tol).sqrt().ceil()))
}

/// Chord ends along a cubic (both ends included) with their parameters.
pub fn flatten_cubic(c: &Cubic, tol: f64) -> (Vec<Pt>, Vec<f64>) {
    let n = flatten_steps(c, tol, 256.0);
    let mut pts = vec![c[0]];
    let mut ts = vec![0.0];
    let mut k = 1.0;
    while k < n {
        pts.push(bez(c, k / n));
        ts.push(k / n);
        k += 1.0;
    }
    pts.push(c[3]);
    ts.push(1.0);
    (pts, ts)
}

/// A sub-path as a polyline within `tol`; a closed one does not repeat its start (the closing chord is implied).
pub fn flatten_sub_path_tol(sp: &SubPath, tol: f64) -> Vec<Pt> {
    let mut out = Vec::new();
    let n = segment_count(sp);
    if sp.nodes.is_empty() {
        return out;
    }
    out.push(sp.nodes[0].pt());
    for i in 0..n {
        if segment_is_line(sp, i) {
            out.push(sp.nodes[(i + 1) % sp.nodes.len()].pt());
            continue;
        }
        let (pts, _) = flatten_cubic(&segment_cubic(sp, i), tol);
        out.extend_from_slice(&pts[1..]);
    }
    // A closed sub-path came back to its start: the ring does not repeat it.
    if sp.closed && out.len() > 1 {
        let a = out[0];
        let b = out[out.len() - 1];
        if (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12 {
            out.pop();
        }
    }
    out
}

// Gauss–Legendre nodes on [0, 1] (8 points): exact enough for symbol-sized curves.
// The TypeScript's literals, digit for digit (they read to the same numbers).
#[allow(clippy::excessive_precision)]
const GX: [f64; 8] = [
    0.0198550717512319,
    0.1016667612931866,
    0.2372337950418355,
    0.4082826787521751,
    0.5917173212478249,
    0.7627662049581645,
    0.8983332387068134,
    0.9801449282487681,
];
#[allow(clippy::excessive_precision)]
const GW: [f64; 8] = [
    0.0506142681451881,
    0.1111905172266872,
    0.1568533229389436,
    0.1813418916891810,
    0.1813418916891810,
    0.1568533229389436,
    0.1111905172266872,
    0.0506142681451881,
];

/// Arc length between t0 and t1 (split in quarters for accuracy on tight curves).
pub fn cubic_length(c: &Cubic, t0: f64, t1: f64) -> f64 {
    if c[0][0] == c[1][0] && c[0][1] == c[1][1] && c[2][0] == c[3][0] && c[2][1] == c[3][1] {
        let a = bez(c, t0);
        let b = bez(c, t1);
        return js_hypot(b[0] - a[0], b[1] - a[1]);
    }
    let mut s = 0.0;
    let parts = 4.0;
    for p in 0..4 {
        let p = p as f64;
        let a = t0 + ((t1 - t0) * p) / parts;
        let b = t0 + ((t1 - t0) * (p + 1.0)) / parts;
        for k in 0..GX.len() {
            let d = bez_deriv(c, a + (b - a) * GX[k]);
            s += GW[k] * (b - a) * js_hypot(d[0], d[1]);
        }
    }
    s
}

/// Length of segment i of a sub-path.
pub fn segment_length(sp: &SubPath, i: usize) -> f64 {
    cubic_length(&segment_cubic(sp, i), 0.0, 1.0)
}

/// The nearest point of a curve: its parameter, the point and the distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nearest {
    pub t: f64,
    pub p: Pt,
    pub d: f64,
}

kentos_geometry_core::json_struct!(Nearest { t, p, d });

/// Parameter on the curve nearest to p (sampling, then Newton steps).
pub fn nearest_on_cubic(c: &Cubic, p: Pt) -> Nearest {
    let mut best_t = 0.0;
    let mut best_d = f64::INFINITY;
    let n = 32.0;
    for k in 0..=32 {
        let k = k as f64;
        let q = bez(c, k / n);
        let d = (q[0] - p[0]) * (q[0] - p[0]) + (q[1] - p[1]) * (q[1] - p[1]);
        if d < best_d {
            best_d = d;
            best_t = k / n;
        }
    }
    let mut t = best_t;
    for _ in 0..8 {
        let q = bez(c, t);
        let d1 = bez_deriv(c, t);
        // Second derivative for the Newton step on (B(t) − p)·B'(t) = 0.
        let d2 = bez_deriv2(c, t);
        let f = (q[0] - p[0]) * d1[0] + (q[1] - p[1]) * d1[1];
        let df = d1[0] * d1[0] + d1[1] * d1[1] + (q[0] - p[0]) * d2[0] + (q[1] - p[1]) * d2[1];
        if df.abs() < 1e-18 {
            break;
        }
        let next = js_min(1.0, js_max(0.0, t - f / df));
        if (next - t).abs() < 1e-12 {
            t = next;
            break;
        }
        t = next;
    }
    let q = bez(c, t);
    let d = js_hypot(q[0] - p[0], q[1] - p[1]);
    if d * d > best_d + 1e-18 {
        let b = bez(c, best_t);
        return Nearest {
            t: best_t,
            p: b,
            d: best_d.sqrt(),
        };
    }
    Nearest { t, p: q, d }
}

/// The parameter where the curve is `dist` away (straight line) from its start (`from_end`: from its end).
pub fn param_at_distance(c: &Cubic, dist: f64, from_end: bool) -> Option<f64> {
    let o = if from_end { c[3] } else { c[0] };
    let f = |t: f64| js_hypot(bez(c, t)[0] - o[0], bez(c, t)[1] - o[1]) - dist;
    let n = 64.0;
    let mut prev_t = if from_end { 1.0 } else { 0.0 };
    let mut prev_f = f(prev_t);
    for k in 1..=64 {
        let k = k as f64;
        let t = if from_end { 1.0 - k / n } else { k / n };
        let v = f(t);
        if prev_f <= 0.0 && v >= 0.0 {
            let mut lo = prev_t;
            let mut hi = t;
            for _ in 0..60 {
                let m = (lo + hi) / 2.0;
                if f(m) < 0.0 {
                    lo = m;
                } else {
                    hi = m;
                }
            }
            return Some((lo + hi) / 2.0);
        }
        prev_t = t;
        prev_f = v;
    }
    None
}

/// Nodes of a sub-path in the opposite direction (handles swap sides).
pub fn reverse_sub_path(sp: &SubPath) -> SubPath {
    let mut nodes: Vec<PathNode> = sp
        .nodes
        .iter()
        .rev()
        .map(|n| PathNode {
            in_: n.out,
            out: n.in_,
            ..n.clone()
        })
        .collect();
    if sp.closed && nodes.len() > 1 {
        // Node 0 stays first so a closed ring keeps its start.
        if let Some(last) = nodes.pop() {
            nodes.insert(0, last);
        }
    }
    SubPath {
        closed: sp.closed,
        nodes,
    }
}

/// Signed area of a ring (shoelace, y down: positive is clockwise on screen).
pub fn ring_signed_area(pts: &[Pt]) -> f64 {
    let mut a = 0.0;
    let n = pts.len();
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        a += p[0] * q[1] - q[0] * p[1];
    }
    a / 2.0
}

/// Winding number of a closed polyline around p.
pub fn winding_of(ring: &[Pt], p: Pt) -> i32 {
    let mut w = 0;
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        let cross = (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]);
        if a[1] <= p[1] {
            if b[1] > p[1] && cross > 0.0 {
                w += 1;
            }
        } else if b[1] <= p[1] && cross < 0.0 {
            w -= 1;
        }
    }
    w
}
