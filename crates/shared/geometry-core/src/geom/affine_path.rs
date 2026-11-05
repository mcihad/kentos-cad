//! A vertex path under a non-similar affine map, kept exact (docs/adr/1061 §3, docs/adr/1008): its arcs as one
//! rational quadratic B-spline and its straight edges' widths measured again across their images. The DXF reader's
//! nonuniform block (`kentos_formats::dxf::emit`) and Patlat's (`block::Blocks::explode`) both use it: one rule for
//! what a drawing opens and what it explodes. `libm::hypot` (not `js_hypot`) because the reader's numbers came from
//! it before this rule moved here; the results stay bit for bit.

use crate::geom::affine::{Affine, apply};
use crate::vec2::Vec2;

fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    hypot(b.x - a.x, b.y - a.y)
}

fn finite(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The widths of a straight path's edges under `m`: edge `i` (from vertex `i` to the next) keeps its start and end
/// widths times |det M| / |M·t|, t its unit direction in the source (a strip between two parallel lines is again
/// one under an affine map). None when the map is singular or an edge has no length (its direction, so its width,
/// is not defined): the caller keeps its own way.
pub fn exact_widths(m: &Affine, pts: &[Vec2], widths: &[[f64; 2]], closed: bool) -> Option<Vec<[f64; 2]>> {
    let det = (m[0] * m[3] - m[1] * m[2]).abs();
    if !(det.is_finite() && det > 0.0) {
        return None;
    }
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    let mut out = Vec::with_capacity(n);
    for (i, w) in widths.iter().enumerate().take(n) {
        if i >= edges {
            // An open path's last vertex starts no edge: its widths draw nothing.
            out.push(*w);
            continue;
        }
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let len = dist(p, q);
        if !(len > 0.0 && len.is_finite()) {
            return None;
        }
        let (tx, ty) = ((q.x - p.x) / len, (q.y - p.y) / len);
        let image = hypot(m[0] * tx + m[2] * ty, m[1] * tx + m[3] * ty);
        if !(image > 0.0 && image.is_finite()) {
            return None;
        }
        let k = det / image;
        out.push([w[0] * k, w[1] * k]);
    }
    Some(out)
}

/// The point at the middle of the arc from `p` to `q` with bulge `b`: the chord's middle moved right of it by the
/// sagitta, c/2 · b.
fn bulge_point(p: Vec2, q: Vec2, b: f64) -> Vec2 {
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    Vec2::new((p.x + q.x) / 2.0 + dy * b / 2.0, (p.y + q.y) / 2.0 - dx * b / 2.0)
}

/// An open vertex path with bulges under `m` as one rational quadratic B-spline: each edge a rational Bézier piece
/// (a straight edge its ends and their middle, weight 1; an arc of sweep φ, split in halves while |bulge| ≥ 1, its
/// ends and the meeting of its end tangents, weight cos(φ/2) = (1 − b²)/(1 + b²) with b = tan(φ/4)), joined end to end
/// (double inner knots). The control points are mapped, the weights kept: a rational Bézier's image under an affine
/// map is the Bézier of the mapped points (Piegl & Tiller, The NURBS Book §7.3–7.4). Control points, weights and
/// knots; none when an edge has no length or a number is not finite.
pub fn arcs_as_nurbs(m: &Affine, pts: &[Vec2], bulges: &[f64]) -> Option<(Vec<Vec2>, Vec<f64>, Vec<f64>)> {
    if pts.len() < 2 {
        return None;
    }
    let mut pieces: Vec<(Vec2, Vec2, f64)> = Vec::new();
    let mut ctrl = vec![apply(m, pts[0])];
    let mut weights = vec![1.0];
    for i in 0..pts.len() - 1 {
        let (p, q) = (pts[i], pts[i + 1]);
        let c = dist(p, q);
        if !(c > 0.0 && c.is_finite()) {
            return None;
        }
        let b = bulges.get(i).copied().unwrap_or(0.0);
        if !b.is_finite() {
            return None;
        }
        // Halves of an arc as wide as a half circle or wider: tan(θ/8) = b / (1 + √(1 + b²)).
        let mut arcs: Vec<(Vec2, Vec2, f64)> = vec![(p, q, b)];
        while let Some(k) = arcs.iter().position(|a| a.2.abs() >= 1.0) {
            let (a0, a1, ab) = arcs.remove(k);
            let half = ab / (1.0 + hypot(1.0, ab));
            let mid = bulge_point(a0, a1, ab);
            arcs.insert(k, (mid, a1, half));
            arcs.insert(k, (a0, mid, half));
            if arcs.len() > 8 {
                return None;
            }
        }
        pieces.extend(arcs);
    }
    for (p, q, b) in pieces {
        let (apex, w) = if b == 0.0 {
            (Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0), 1.0)
        } else {
            // The end tangents meet at the chord's middle moved to the arc's side by c/2 · tan(φ/2), tan(φ/2) =
            // 2b/(1 − b²); a positive bulge turns counter-clockwise, its arc right of the chord.
            let (dx, dy) = (q.x - p.x, q.y - p.y);
            let rise = 2.0 * b / (1.0 - b * b) / 2.0;
            (
                Vec2::new((p.x + q.x) / 2.0 + dy * rise, (p.y + q.y) / 2.0 - dx * rise),
                (1.0 - b * b) / (1.0 + b * b),
            )
        };
        if !(w > 0.0 && w.is_finite() && finite(apex)) {
            return None;
        }
        ctrl.push(apply(m, apex));
        weights.push(w);
        ctrl.push(apply(m, q));
        weights.push(1.0);
    }
    let n = (ctrl.len() - 1) / 2;
    let mut knots = vec![0.0, 0.0, 0.0];
    for k in 1..n {
        knots.push(k as f64);
        knots.push(k as f64);
    }
    knots.extend([n as f64; 3]);
    ctrl.iter().all(|c| finite(*c)).then_some((ctrl, weights, knots))
}
