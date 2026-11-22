//! Alanla böl (docs/adr/1069): the line that cuts a closed area into a piece of a target size beside a chosen edge,
//! the line parallel to that edge. Geometry only: a parcel's legal subdivision (ifraz) is recorded apart. The
//! independent reference is `scripts/fixtures/area_cut_cases.py` (`fixtures/area-cut/v1/cases.json`).
//!
//! In the edge's own frame (s along the edge from its first vertex, h square to it into the area) the piece is the
//! area's part with h ≤ d. Its size A(d) grows with d; between two vertices' heights the cut's length w(h) is linear,
//! so A is quadratic there: A(d) = A(Hⱼ) + w₀·x + ½·k·x², x = d − Hⱼ, solved in closed form on the span the target
//! falls in. Worked in the edge's frame so that large map coordinates do not round the small ones away.
//!
//! Stage one: one outer ring of straight edges, the cut one straight chord. A line that crosses the boundary more than
//! twice would leave more than two pieces and is refused.

use crate::jsmath::js_hypot;
use crate::vec2::Vec2;

/// Why no cut is given, in the tool's words.
pub const TOO_FEW: &str = "Alanla bölmek için en az üç köşeli, alanı olan kapalı bir alan gerekir.";
pub const NO_EDGE: &str = "Bu alanda böyle bir kenar yok.";
pub const ZERO_EDGE: &str = "Referans kenarın uzunluğu yok; başka bir kenar seçin.";
pub const TARGET: &str = "Ayrılacak alan sıfırdan büyük ve alanın tamamından küçük olmalı.";
pub const MORE_PIECES: &str =
    "Bu alanı veren kesim çizgisi alanı ikiden çok parçaya böler; başka bir kenar seçin ya da Alan böl ile elle kesin.";

/// The cut: its two ends on the boundary (in the edge's direction), how far it lies from the edge, and the two
/// pieces' areas, the one beside the edge first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    pub a: Vec2,
    pub b: Vec2,
    pub distance: f64,
    pub near: f64,
    pub far: f64,
}

/// A vertex in the edge's frame.
#[derive(Clone, Copy)]
struct Local {
    s: f64,
    h: f64,
}

/// The ring's area by the shoelace, positive counter-clockwise.
fn signed_area(pts: &[Local]) -> f64 {
    let n = pts.len();
    let mut twice = 0.0;
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        twice += p.s * q.h - q.s * p.h;
    }
    twice / 2.0
}

/// The area of the ring's part with h ≤ d (the ring clipped by the half-plane).
fn below(ring: &[Local], d: f64) -> f64 {
    let n = ring.len();
    let mut out: Vec<Local> = Vec::with_capacity(n + 4);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let (pin, qin) = (p.h <= d, q.h <= d);
        if pin {
            out.push(p);
        }
        if pin != qin {
            out.push(crossing(p, q, d));
        }
    }
    if out.len() < 3 { 0.0 } else { signed_area(&out).abs() }
}

/// Where the edge p → q meets the line h = d.
fn crossing(p: Local, q: Local, d: f64) -> Local {
    let t = (d - p.h) / (q.h - p.h);
    Local {
        s: p.s + t * (q.s - p.s),
        h: d,
    }
}

/// Where the line h = d crosses the boundary, along s (half-open: a vertex on the line counts once).
fn crossings(ring: &[Local], d: f64) -> Vec<f64> {
    let n = ring.len();
    let mut at: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (p, q) = (ring[i], ring[(i + 1) % n]);
            ((p.h <= d) != (q.h <= d)).then(|| crossing(p, q, d).s)
        })
        .collect();
    at.sort_by(f64::total_cmp);
    at
}

/// The cut's length at height t: the inside stretches of the line h = t.
fn width(ring: &[Local], t: f64) -> f64 {
    crossings(ring, t).chunks(2).map(|c| if c.len() == 2 { c[1] - c[0] } else { 0.0 }).sum()
}

/// The cut parallel to edge `edge` (from vertex `edge` to the next) that leaves `target` square metres beside it.
/// `ring` is the outer ring, either way round, closed or not.
pub fn cut_for_area(ring: &[Vec2], edge: usize, target: f64) -> Result<Cut, &'static str> {
    let mut pts = ring.to_vec();
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    let n = pts.len();
    if n < 3 {
        return Err(TOO_FEW);
    }
    if edge >= n {
        return Err(NO_EDGE);
    }
    let (p0, p1) = (pts[edge], pts[(edge + 1) % n]);
    let length = js_hypot(p1.x - p0.x, p1.y - p0.y);
    if length == 0.0 || !length.is_finite() {
        return Err(ZERO_EDGE);
    }
    let u = Vec2::new((p1.x - p0.x) / length, (p1.y - p0.y) / length);
    // The ring's turn, worked about the edge's first vertex.
    let around: Vec<Local> = pts.iter().map(|v| Local { s: v.x - p0.x, h: v.y - p0.y }).collect();
    let turn = signed_area(&around);
    if turn == 0.0 || !turn.is_finite() {
        return Err(TOO_FEW);
    }
    // Into the area: the left of the edge on a counter-clockwise ring, its right on a clockwise one. The frame is then
    // counter-clockwise either way.
    let normal = if turn > 0.0 { Vec2::new(-u.y, u.x) } else { Vec2::new(u.y, -u.x) };
    let local: Vec<Local> = pts
        .iter()
        .map(|v| {
            let (x, y) = (v.x - p0.x, v.y - p0.y);
            Local {
                s: x * u.x + y * u.y,
                h: x * normal.x + y * normal.y,
            }
        })
        .collect();
    let total = signed_area(&local).abs();
    if !(target.is_finite() && target > 0.0 && target < total) {
        return Err(TARGET);
    }
    let mut heights: Vec<f64> = local.iter().map(|p| p.h).collect();
    heights.sort_by(f64::total_cmp);
    heights.dedup();
    // The span whose ends' areas hold the target.
    let mut d = None;
    for pair in heights.windows(2) {
        let (lo, hi) = (pair[0], pair[1]);
        let (a_lo, a_hi) = (below(&local, lo), below(&local, hi));
        if target < a_lo || target > a_hi {
            continue;
        }
        let span = hi - lo;
        let (w1, w2) = (width(&local, lo + span / 3.0), width(&local, lo + 2.0 * span / 3.0));
        let k = (w2 - w1) * 3.0 / span;
        let w0 = w1 - k * span / 3.0;
        let c = target - a_lo;
        let root = crate::jsmath::js_max(w0 * w0 + 2.0 * k * c, 0.0).sqrt();
        let x = if w0 + root > 0.0 { 2.0 * c / (w0 + root) } else { 0.0 };
        d = Some(lo + x.clamp(0.0, span));
        break;
    }
    let d = d.ok_or(TARGET)?;
    let at = crossings(&local, d);
    let [sa, sb] = at[..] else {
        return Err(MORE_PIECES);
    };
    let world = |s: f64| Vec2::new(p0.x + u.x * s + normal.x * d, p0.y + u.y * s + normal.y * d);
    let near = below(&local, d);
    Ok(Cut {
        a: world(sa),
        b: world(sb),
        distance: d,
        near,
        far: total - near,
    })
}
