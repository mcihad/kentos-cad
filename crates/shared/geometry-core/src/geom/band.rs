//! A wide polyline's band (docs/adr/0258 §2): what is drawn of a polyline
//! with widths, as AutoCAD draws one with its fill on. Each edge with a width
//! is a strip from its start width to its end width, flat at both ends: a
//! straight edge a trapezoid, an arc a ring sector whose width runs linearly
//! along its angle. Where two such edges meet, the gap on the outer side of
//! the turn is closed by a mitred wedge, bevelled when the mitre would reach
//! past [`MITRE`] times the wider half width. Edges of no width at either end
//! are the line itself: their runs come back as paths, drawn with the line's
//! weight. Every ring is counter-clockwise and filled on its own, all in the
//! object's colour; they overlap only where they meet.

use crate::geom::bulge::{bulge_arc, bulge_at, segment_tangent};
use crate::geometry::signed_area;
use crate::jsmath::{TAU, cos, js_hypot, js_max, sin};
use crate::vec2::Vec2;

/// How far a mitre may reach from its vertex, in the wider half width.
pub const MITRE: f64 = 4.0;

/// An arc's strip is drawn in steps of at most this turn (`entityOutline`'s 72 to the circle).
const STEP: f64 = TAU / 72.0;

/// What a wide polyline draws: the runs of edges without width as paths, and
/// the band's rings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Band {
    pub thin: Vec<Vec<Vec2>>,
    pub rings: Vec<Vec<Vec2>>,
}

fn edge_count(n: usize, closed: bool) -> usize {
    if closed { n } else { n.saturating_sub(1) }
}

/// The left of a travel direction.
fn left(t: Vec2) -> Vec2 {
    Vec2::new(-t.y, t.x)
}

fn ccw(mut ring: Vec<Vec2>) -> Vec<Vec2> {
    if signed_area(&ring) < 0.0 {
        ring.reverse();
    }
    ring
}

/// An edge's points from its start to its end: its two ends, or an arc's
/// steps between them.
fn edge_points(a: Vec2, b: Vec2, bulge: f64) -> Vec<Vec2> {
    let Some(arc) = bulge_arc(a, b, bulge) else {
        return vec![a, b];
    };
    let k = js_max(2.0, (arc.sweep.abs() / STEP).ceil()) as usize;
    let mut out = Vec::with_capacity(k + 1);
    out.push(a);
    for i in 1..k {
        let t = arc.a0 + arc.sweep * (i as f64) / (k as f64);
        out.push(Vec2::new(
            arc.c.x + arc.r * cos(t),
            arc.c.y + arc.r * sin(t),
        ));
    }
    out.push(b);
    out
}

/// An edge's strip from half width `h0` to `h1`.
fn strip(a: Vec2, b: Vec2, bulge: f64, h0: f64, h1: f64) -> Option<Vec<Vec2>> {
    if js_hypot(b.x - a.x, b.y - a.y) <= 0.0 {
        return None;
    }
    let Some(arc) = bulge_arc(a, b, bulge) else {
        let t = segment_tangent(a, b, 0.0, false);
        let n = left(t);
        return Some(ccw(vec![
            Vec2::new(a.x - n.x * h0, a.y - n.y * h0),
            Vec2::new(b.x - n.x * h1, b.y - n.y * h1),
            Vec2::new(b.x + n.x * h1, b.y + n.y * h1),
            Vec2::new(a.x + n.x * h0, a.y + n.y * h0),
        ]));
    };
    let k = js_max(2.0, (arc.sweep.abs() / STEP).ceil()) as usize;
    let mut outer = Vec::with_capacity(k + 1);
    let mut inner = Vec::with_capacity(k + 1);
    for i in 0..=k {
        let f = i as f64 / k as f64;
        let angle = arc.a0 + arc.sweep * f;
        let h = h0 + (h1 - h0) * f;
        let (c, s) = (cos(angle), sin(angle));
        outer.push(Vec2::new(
            arc.c.x + (arc.r + h) * c,
            arc.c.y + (arc.r + h) * s,
        ));
        let ri = js_max(arc.r - h, 0.0);
        inner.push(Vec2::new(arc.c.x + ri * c, arc.c.y + ri * s));
    }
    inner.reverse();
    outer.extend(inner);
    Some(ccw(outer))
}

/// The wedge closing the outer side where an edge arriving at `v` along `tin`
/// with half width `hin` meets one leaving along `tout` with `hout`.
fn wedge(v: Vec2, tin: Vec2, hin: f64, tout: Vec2, hout: f64) -> Option<Vec<Vec2>> {
    let cross = tin.x * tout.y - tin.y * tout.x;
    if cross.abs() < 1e-12 {
        return None;
    }
    // A left turn opens its right side.
    let side = if cross > 0.0 { -1.0 } else { 1.0 };
    let (nin, nout) = (left(tin), left(tout));
    let p1 = Vec2::new(v.x + nin.x * hin * side, v.y + nin.y * hin * side);
    let p2 = Vec2::new(v.x + nout.x * hout * side, v.y + nout.y * hout * side);
    // The mitre: p1 on along `tin`, p2 back along `tout`.
    let d = Vec2::new(p2.x - p1.x, p2.y - p1.y);
    let s = (d.x * tout.y - d.y * tout.x) / cross;
    let m = Vec2::new(p1.x + tin.x * s, p1.y + tin.y * s);
    let reach = js_hypot(m.x - v.x, m.y - v.y);
    let ring = if s >= 0.0 && reach <= MITRE * js_max(hin, hout) {
        vec![v, p1, m, p2]
    } else {
        vec![v, p1, p2]
    };
    Some(ccw(ring))
}

/// What a polyline of `pts`, `bulges` and `widths` (one pair for each
/// vertex's edge) draws.
pub fn band(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, widths: &[[f64; 2]]) -> Band {
    let n = pts.len();
    let count = edge_count(n, closed);
    let w = |i: usize| widths.get(i).copied().unwrap_or([0.0, 0.0]);
    let wide = |i: usize| {
        let [a, b] = w(i);
        a > 0.0 || b > 0.0
    };
    let mut out = Band::default();
    let mut run: Vec<Vec2> = Vec::new();
    for i in 0..count {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let bulge = bulge_at(bulges, i);
        if wide(i) {
            if run.len() >= 2 {
                out.thin.push(std::mem::take(&mut run));
            }
            run.clear();
            let [w0, w1] = w(i);
            out.rings.extend(strip(a, b, bulge, w0 / 2.0, w1 / 2.0));
        } else {
            let points = edge_points(a, b, bulge);
            let from = usize::from(!run.is_empty());
            run.extend_from_slice(&points[from..]);
        }
    }
    if run.len() >= 2 {
        out.thin.push(run);
    }
    // The joints: every inner vertex, and a closed line's first.
    let joints: Box<dyn Iterator<Item = usize>> = if closed {
        Box::new(0..n)
    } else {
        Box::new(1..n.saturating_sub(1))
    };
    for k in joints {
        let i = (k + n - 1) % n;
        let (hin, hout) = (w(i)[1] / 2.0, w(k)[0] / 2.0);
        if !(hin > 0.0 && hout > 0.0) {
            continue;
        }
        let tin = segment_tangent(pts[i], pts[k], bulge_at(bulges, i), true);
        let tout = segment_tangent(pts[k], pts[(k + 1) % n], bulge_at(bulges, k), false);
        out.rings.extend(wedge(pts[k], tin, hin, tout, hout));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn a_straight_edge_is_a_trapezoid_flat_at_its_ends() {
        let b = band(
            &[v(0.0, 0.0), v(10.0, 0.0)],
            None,
            false,
            &[[2.0, 4.0], [0.0, 0.0]],
        );
        assert!(b.thin.is_empty());
        assert_eq!(
            b.rings,
            vec![vec![v(0.0, -1.0), v(10.0, -2.0), v(10.0, 2.0), v(0.0, 1.0)]]
        );
    }

    #[test]
    fn a_right_angle_is_mitred_on_its_outer_side() {
        let b = band(
            &[v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0)],
            None,
            false,
            &[[2.0, 2.0], [2.0, 2.0], [0.0, 0.0]],
        );
        // Two strips and the wedge of the left turn, on its right (outer) side.
        assert_eq!(b.rings.len(), 3);
        let wedge = &b.rings[2];
        assert!(wedge.contains(&v(11.0, -1.0)), "{wedge:?}");
        assert!(wedge.contains(&v(10.0, -1.0)) && wedge.contains(&v(11.0, 0.0)));
    }

    #[test]
    fn edges_without_width_are_the_line_itself() {
        let b = band(
            &[v(0.0, 0.0), v(5.0, 0.0), v(10.0, 0.0), v(15.0, 0.0)],
            None,
            false,
            &[[0.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 0.0]],
        );
        assert_eq!(b.thin, vec![vec![v(0.0, 0.0), v(5.0, 0.0), v(10.0, 0.0)]]);
        assert_eq!(b.rings.len(), 1);
    }

    #[test]
    fn a_closed_square_is_mitred_at_every_corner() {
        let b = band(
            &[v(0.0, 0.0), v(4.0, 0.0), v(4.0, 4.0), v(0.0, 4.0)],
            None,
            true,
            &[[1.0, 1.0]; 4],
        );
        assert_eq!(b.rings.len(), 8);
        assert!(b.rings.iter().all(|r| signed_area(r) > 0.0));
    }

    #[test]
    fn an_arcs_strip_follows_its_circle() {
        // A half circle of radius 5 about (5, 0), counter-clockwise: bulge 1.
        let b = band(
            &[v(0.0, 0.0), v(10.0, 0.0)],
            Some(&[1.0, 0.0]),
            false,
            &[[2.0, 2.0], [0.0, 0.0]],
        );
        let ring = &b.rings[0];
        for p in ring {
            let r = js_hypot(p.x - 5.0, p.y);
            assert!(
                (r - 4.0).abs() < 1e-9 || (r - 6.0).abs() < 1e-9,
                "{p:?} {r}"
            );
        }
    }
}
