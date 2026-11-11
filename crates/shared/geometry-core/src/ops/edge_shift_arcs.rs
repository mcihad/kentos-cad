//! Paralel kaydır on arcs (docs/adr/1024; plan/map-cad.md MAP-06): the edge `a → b` of a ring moved by `d` along its
//! normal when the edge or a neighbour is an arc. Each moved vertex is where the moved edge's curve meets its
//! neighbour's own curve: a straight edge's line moved `d` along its normal, an arc's circle with its radius grown or
//! shrunk by `d` about the same centre; a straight neighbour keeps its line, an arc neighbour its circle (centre and
//! radius), its far vertex fixed. Every arc stays an arc of its own circle (its bulge is the new sweep's): nothing is
//! turned into chords. Of two meeting points the nearer to the old vertex is taken. An open polyline's free end moves
//! square to its edge (along the radius for an arc). The all-straight case is `edge_shift`'s own closed form.
//!
//! A target area: the area is no longer quadratic in `d`; it is bracketed from 0 outward and bisected to the last
//! bit (`for_area`), the same steps on every platform.

use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::jsmath::{atan2, js_hypot, js_max};
use crate::vec2::Vec2;

use super::edge_shift::{NO_AREA, NO_EDGE, PARALLEL, PASSED};

/// A neighbour is parallel to the edge when the sine of the angle between them is at most this (as `edge_shift`).
const PARALLEL_SINE: f64 = 1e-12;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Curve {
    Line { p: Vec2, dir: Vec2 },
    Circle { c: Vec2, r: f64 },
}

fn sub(p: Vec2, q: Vec2) -> Vec2 {
    Vec2::new(p.x - q.x, p.y - q.y)
}

fn dot(p: Vec2, q: Vec2) -> f64 {
    p.x * q.x + p.y * q.y
}

fn cross(p: Vec2, q: Vec2) -> f64 {
    p.x * q.y - p.y * q.x
}

fn dist(p: Vec2, q: Vec2) -> f64 {
    js_hypot(p.x - q.x, p.y - q.y)
}

/// The curve of the segment `a → b` with `bulge`: its circle, else its line.
pub(crate) fn curve_of(a: Vec2, b: Vec2, bulge: f64) -> Option<Curve> {
    if let Some(arc) = bulge_arc(a, b, bulge) {
        return Some(Curve::Circle { c: arc.c, r: arc.r });
    }
    let t = sub(b, a);
    let l = js_hypot(t.x, t.y);
    (l > 0.0).then(|| Curve::Line { p: a, dir: Vec2::new(t.x / l, t.y / l) })
}

/// Where `u` meets `v`, the point nearer `near`; none when they do not meet (parallel lines, a line missing a
/// circle, circles apart or one inside the other, a tangent counts as meeting).
pub(crate) fn meet(u: Curve, v: Curve, near: Vec2) -> Option<Vec2> {
    let nearer = |p: Vec2, q: Vec2| if dist(p, near) <= dist(q, near) { p } else { q };
    match (u, v) {
        (Curve::Line { p, dir: d }, Curve::Line { p: q, dir: e }) => {
            let x = cross(d, e);
            if x.abs() <= PARALLEL_SINE {
                return None;
            }
            let t = cross(sub(q, p), e) / x;
            Some(Vec2::new(p.x + d.x * t, p.y + d.y * t))
        }
        (Curve::Line { p, dir: d }, Curve::Circle { c, r }) | (Curve::Circle { c, r }, Curve::Line { p, dir: d }) => {
            // |p + t·d − c|² = r², d a unit vector.
            let w = sub(p, c);
            let b = dot(d, w);
            let disc = b * b - (dot(w, w) - r * r);
            if !(disc >= 0.0) {
                return None;
            }
            let s = disc.sqrt();
            let at = |t: f64| Vec2::new(p.x + d.x * t, p.y + d.y * t);
            Some(nearer(at(-b - s), at(-b + s)))
        }
        (Curve::Circle { c: c1, r: r1 }, Curve::Circle { c: c2, r: r2 }) => {
            let k = dist(c1, c2);
            if !(k > 0.0) || k > r1 + r2 || k < (r1 - r2).abs() {
                return None;
            }
            let a = (r1 * r1 - r2 * r2 + k * k) / (2.0 * k);
            let h = js_max(r1 * r1 - a * a, 0.0).sqrt();
            let e = Vec2::new((c2.x - c1.x) / k, (c2.y - c1.y) / k);
            let m = Vec2::new(c1.x + e.x * a, c1.y + e.y * a);
            Some(nearer(Vec2::new(m.x - e.y * h, m.y + e.x * h), Vec2::new(m.x + e.y * h, m.y - e.x * h)))
        }
    }
}

/// The sweep from `p` to `q` about `c` turning as `sweep` turns (its sign), in (0, 2π) that way; none when it is
/// gone (the two points one) or jumped more than half a turn from `sweep` (the arc turned round).
pub(crate) fn sweep_to(c: Vec2, p: Vec2, q: Vec2, sweep: f64) -> Option<f64> {
    let (u, v) = (sub(p, c), sub(q, c));
    let mut a = atan2(cross(u, v), dot(u, v));
    let tau = 2.0 * std::f64::consts::PI;
    if sweep > 0.0 && a <= 0.0 {
        a += tau;
    } else if sweep < 0.0 && a >= 0.0 {
        a -= tau;
    }
    (a != 0.0 && a.abs() < tau && (a - sweep).abs() < std::f64::consts::PI).then_some(a)
}

/// A ring after the shift: its points and bulges.
pub(crate) struct Moved {
    pub pts: Vec<Vec2>,
    pub bulges: Vec<f64>,
}

/// The edge `edge` of the ring (`pts`, `bulges`; `closed` for an area's ring or a closed polyline) moved `d` along
/// `normal` (unit: away from an area's inside, a polyline's right). For an edge or a neighbour that is an arc;
/// the refusals are `edge_shift`'s.
pub(crate) fn shifted(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    edge: usize,
    normal: Vec2,
    d: f64,
) -> Result<Moved, &'static str> {
    let n = pts.len();
    let (i, j) = (edge, (edge + 1) % n);
    let (a, b) = (pts[i], pts[j]);
    let own_bulge = bulge_at(bulges, i);
    let own = curve_of(a, b, own_bulge).ok_or(NO_EDGE)?;
    // The moved edge's curve: the line moved along the normal, the circle grown away from (or towards) its centre.
    let moved = match own {
        Curve::Line { p, dir } => Curve::Line { p: Vec2::new(p.x + normal.x * d, p.y + normal.y * d), dir },
        Curve::Circle { c, r } => {
            let mid = crate::geom::bulge::segment_mid(a, b, own_bulge);
            let away = if dot(normal, sub(mid, c)) > 0.0 { 1.0 } else { -1.0 };
            let r2 = r + away * d;
            if !(r2 > 0.0) {
                return Err(PASSED);
            }
            Curve::Circle { c, r: r2 }
        }
    };
    // A free end: square to the edge (along the radius for an arc).
    let free = |v: Vec2| -> Result<Vec2, &'static str> {
        match moved {
            Curve::Line { .. } => Ok(Vec2::new(v.x + normal.x * d, v.y + normal.y * d)),
            Curve::Circle { c, r } => {
                let w = sub(v, c);
                let l = js_hypot(w.x, w.y);
                Ok(Vec2::new(c.x + w.x / l * r, c.y + w.y / l * r))
            }
        }
    };
    let mut new_pts = pts.to_vec();
    let mut new_bulges: Vec<f64> = (0..n).map(|k| bulge_at(bulges, k)).collect();
    // Each end: the start's neighbour is the segment before it (h → a), the end's the one after it (b → k).
    let ends = [(i, true), (j, false)];
    for (v, before) in ends {
        let at = pts[v];
        let neighbour = if before {
            (closed || v > 0).then(|| ((v + n - 1) % n, (v + n - 1) % n))
        } else {
            (closed || v + 1 < n).then_some((v, (v + 1) % n))
        };
        let Some((segment, far)) = neighbour else {
            new_pts[v] = free(at)?;
            continue;
        };
        let nb_bulge = bulge_at(bulges, segment);
        let (s0, s1) = if before { (pts[far], at) } else { (at, pts[far]) };
        let nb = curve_of(s0, s1, nb_bulge).ok_or(PASSED)?;
        if let (Curve::Line { dir: e, .. }, Curve::Line { dir: f, .. }) = (moved, nb)
            && cross(e, f).abs() <= PARALLEL_SINE
        {
            return Err(PARALLEL);
        }
        let to = meet(moved, nb, at).ok_or(PASSED)?;
        // The neighbour keeps its far vertex: a straight one must not pass it, an arc one keeps its turn.
        match nb {
            Curve::Line { .. } => {
                let far_p = pts[far];
                if dot(sub(to, far_p), sub(at, far_p)) <= 0.0 {
                    return Err(PASSED);
                }
            }
            Curve::Circle { c, .. } => {
                let old = bulge_arc(s0, s1, nb_bulge).map_or(0.0, |a| a.sweep);
                let (p, q) = if before { (pts[far], to) } else { (to, pts[far]) };
                let sweep = sweep_to(c, p, q, old).ok_or(PASSED)?;
                new_bulges[segment] = bulge_of_sweep(sweep);
            }
        }
        new_pts[v] = to;
    }
    let (a2, b2) = (new_pts[i], new_pts[j]);
    match moved {
        Curve::Line { dir, .. } => {
            if dot(sub(b2, a2), dir) <= 0.0 {
                return Err(PASSED);
            }
        }
        Curve::Circle { c, .. } => {
            let old = bulge_arc(a, b, own_bulge).map_or(0.0, |x| x.sweep);
            let sweep = sweep_to(c, a2, b2, old).ok_or(PASSED)?;
            new_bulges[i] = bulge_of_sweep(sweep);
        }
    }
    Ok(Moved { pts: new_pts, bulges: new_bulges })
}

/// Whether the edge or a neighbour of it is an arc: the case this module takes.
pub(crate) fn involves_arcs(n: usize, bulges: Option<&[f64]>, closed: bool, edge: usize) -> bool {
    let j = (edge + 1) % n;
    let before = (closed || edge > 0).then(|| (edge + n - 1) % n);
    let after = (closed || j + 1 < n).then_some(j);
    [Some(edge), before, after].into_iter().flatten().any(|k| bulge_at(bulges, k) != 0.0)
}

/// The edge's length: its chord's, else its arc's.
pub(crate) fn edge_length(a: Vec2, b: Vec2, bulge: f64) -> f64 {
    match bulge_arc(a, b, bulge) {
        Some(arc) => arc.r * arc.sweep.abs(),
        None => dist(a, b),
    }
}

/// The distance `d` at which `area(d)` (an area's size after the shift, `Err` where the shift is refused) reaches
/// `target`, starting from `area0` at 0: outward from 0 by doubling a first guess (`|area0 − target| / length`) until
/// the size passes the target, then halving the bracket to the last bit. Refused when the shift is refused before
/// the target is passed (that refusal), or when no shift reaches it.
pub(crate) fn for_area(
    area0: f64,
    target: f64,
    length: f64,
    area: &dyn Fn(f64) -> Result<f64, &'static str>,
) -> Result<f64, &'static str> {
    let f0 = area0 - target;
    if f0 == 0.0 {
        return Ok(0.0);
    }
    if !(length > 0.0) || !f0.is_finite() {
        return Err(NO_AREA);
    }
    // The size grows with d (the normal points away from the inside): go against the sign of f0.
    let dir = if f0 > 0.0 { -1.0 } else { 1.0 };
    let guess = f0.abs() / length;
    let (mut lo, mut flo) = (0.0, f0);
    let mut hi = None;
    for k in 0..64 {
        let d = dir * guess * f64::from(1u32 << k.min(31)) * if k > 31 { f64::from(1u32 << (k - 31)) } else { 1.0 };
        let f = area(d)? - target;
        if f == 0.0 {
            return Ok(d);
        }
        if (f > 0.0) != (f0 > 0.0) {
            hi = Some((d, f));
            break;
        }
        (lo, flo) = (d, f);
    }
    let Some((mut hi, mut fhi)) = hi else {
        return Err(NO_AREA);
    };
    for _ in 0..200 {
        let mid = lo + (hi - lo) / 2.0;
        if mid == lo || mid == hi {
            break;
        }
        let f = area(mid)? - target;
        if f == 0.0 {
            return Ok(mid);
        }
        if (f > 0.0) == (flo > 0.0) {
            (lo, flo) = (mid, f);
        } else {
            (hi, fhi) = (mid, f);
        }
    }
    Ok(if flo.abs() <= fhi.abs() { lo } else { hi })
}

