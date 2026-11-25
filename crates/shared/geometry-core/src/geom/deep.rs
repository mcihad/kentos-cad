//! Deep zoom (plan/render.md RM-24): a path's segments split where they cross a box round the view.
//!
//! The GPU takes a point's offset from the camera to float32 parts, good to about 2⁻²⁴ of its distance from the
//! camera (crates/render/wgpu/src/precision.rs). A segment's position near the camera is as good as its ends: a
//! segment through the view whose ends lie 64 m away is off by 4 µm, 4 pixels at 10⁶ px/m. Split where it crosses a
//! box round the view, the piece inside has both its ends in the box, and is as good as the box is small.
//!
//! The points put in lie on the segment, so the path draws the same: its length, a line type's dashes along it,
//! its marks, its area and the triangles of its fill (their edges stay shared) are unchanged.
//!
//! Curves (circles, arcs, a path's bulged edges, ellipses) are drawn round to the view's tolerance in and near the
//! box, as coarse as the drawing draws them away from it.

use crate::Vec2;
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::geometry::Bounds;
use crate::jsmath::{TAU, acos, atan2, cos, js_hypot, js_max, js_min, sin};

/// A deep view (plan/render.md RM-24, docs/adr/9220): the box round it and how far a chord may stray from its curve
/// in the box (a quarter pixel at the view's scale), world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Deep {
    pub bounds: Bounds,
    pub tolerance: f64,
}

/// The coarse step of a deep view's curve round its box: a full turn in 72 chords, as the drawing draws it.
pub const COARSE_STEP: f64 = TAU / 72.0;

/// Most points a deep view's curve takes: its steps near the box grow when a box far larger than its tolerance asks
/// for more.
pub const MOST_POINTS: usize = 65_536;

/// How much farther than a coarse chord strays from its curve the curve is drawn finely round the box: room for the
/// rounding of the box's and the chords' sums.
const NEAR: f64 = 1.25;

/// Where segment `p → q` enters and leaves `b`, as parameters along it (Liang–Barsky); None when it misses `b`.
fn crossing(p: Vec2, q: Vec2, b: &Bounds) -> Option<(f64, f64)> {
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (towards, room) in [
        (-dx, p.x - b.min_x),
        (dx, b.max_x - p.x),
        (-dy, p.y - b.min_y),
        (dy, b.max_y - p.y),
    ] {
        if towards == 0.0 {
            if room < 0.0 {
                return None;
            }
            continue;
        }
        let t = room / towards;
        if towards < 0.0 {
            if t > t1 {
                return None;
            }
            t0 = js_max(t0, t);
        } else {
            if t < t0 {
                return None;
            }
            t1 = js_min(t1, t);
        }
    }
    Some((t0, t1))
}

/// `points`, a path (or a ring, `closed`: its last point joins its first), with the points where a segment crosses
/// the edge of `b` put in, in order along it. None when no segment crosses the edge: the path is drawn as it is.
pub fn split_at_box(points: &[Vec2], closed: bool, b: &Bounds) -> Option<Vec<Vec2>> {
    let n = points.len();
    if n < 2 || !(b.min_x <= b.max_x && b.min_y <= b.max_y) {
        return None;
    }
    // A ring whose last point repeats its first is closed by that segment already.
    let closing = closed && points[0] != points[n - 1];
    let segments = if closing { n } else { n - 1 };
    let mut out: Option<Vec<Vec2>> = None;
    for i in 0..segments {
        let (p, q) = (points[i], points[(i + 1) % n]);
        let at = |t: f64| Vec2::new(p.x + t * (q.x - p.x), p.y + t * (q.y - p.y));
        let mut cuts = [None, None];
        if let Some((t0, t1)) = crossing(p, q, b) {
            if t0 > 0.0 && t0 < 1.0 {
                cuts[0] = Some(at(t0));
            }
            if t1 > t0 && t1 < 1.0 && t1 > 0.0 {
                cuts[1] = Some(at(t1));
            }
        }
        if out.is_none() && cuts.iter().any(Option::is_some) {
            let mut v = Vec::with_capacity(n + 8);
            v.extend_from_slice(&points[..=i]);
            out = Some(v);
        }
        if let Some(v) = out.as_mut() {
            v.extend(cuts.into_iter().flatten());
            // The closing segment's end is the first point, there already.
            if i + 1 < n {
                v.push(q);
            }
        }
    }
    out
}

/// The angle between points of a circle of radius `r` whose chords stay within `tol` of it, at most `most`.
pub fn step_for(r: f64, tol: f64, most: f64) -> f64 {
    if !(tol > 0.0) || !(r > 0.0) || tol >= r {
        return most;
    }
    js_min(2.0 * acos(1.0 - tol / r), most)
}

/// A curve `c + u·cos t + v·sin t`: a circle of radius r (u = (r, 0), v = (0, r)) or an ellipse (u and v its axes,
/// the drawing's form, `geom::ellipse`). Its chords over a step `dt` stray from it by at most
/// `reach` × (1 − cos dt/2): it is a circle of radius `reach` squeezed.
#[derive(Clone, Copy)]
struct Conic {
    c: Vec2,
    u: Vec2,
    v: Vec2,
}

impl Conic {
    /// Its point at `t`: the centre plus each axis term in turn, as the drawing adds an ellipse's (a circle's second
    /// term is nought).
    fn at(&self, t: f64) -> Vec2 {
        let (co, si) = (cos(t), sin(t));
        Vec2::new(
            self.c.x + self.u.x * co + self.v.x * si,
            self.c.y + self.u.y * co + self.v.y * si,
        )
    }

    /// Its longer half axis.
    fn reach(&self) -> f64 {
        js_max(js_hypot(self.u.x, self.u.y), js_hypot(self.v.x, self.v.y))
    }
}

/// Where `k` crosses the edge of `b`: its parameters from `t0` in its direction (`turn` ±1), within (0, `span`).
fn crossings(k: &Conic, t0: f64, turn: f64, span: f64, b: &Bounds, out: &mut Vec<f64>) {
    // On the edge x = e: u.x cos t + v.x sin t = e − c.x, that is R cos(t − φ) with R and φ the length and angle of
    // (u.x, v.x); the same for y.
    let edges = [
        (k.u.x, k.v.x, b.min_x - k.c.x, true),
        (k.u.x, k.v.x, b.max_x - k.c.x, true),
        (k.u.y, k.v.y, b.min_y - k.c.y, false),
        (k.u.y, k.v.y, b.max_y - k.c.y, false),
    ];
    for (cu, cv, room, upright) in edges {
        let r = js_hypot(cu, cv);
        if !(r > 0.0) || (room / r).abs() > 1.0 {
            continue;
        }
        let (phi, half) = (atan2(cv, cu), acos(room / r));
        for t in [phi + half, phi - half] {
            let p = k.at(t);
            let along = if upright {
                p.y >= b.min_y && p.y <= b.max_y
            } else {
                p.x >= b.min_x && p.x <= b.max_x
            };
            let s = norm_angle(turn * (t - t0));
            if along && s > 0.0 && s < span {
                out.push(s);
            }
        }
    }
}

/// Points of `k` from parameter `t0` over signed `sweep` (a full turn at most), both ends included: in and near `b`
/// at most `fine` apart, away from it at most `coarse`, and one at each crossing of `b`'s edge. Every point lies on
/// the curve.
///
/// Near means within (a little more than) a coarse chord's stray of the box: a curve passing beside the box, the box
/// on its inner side, would have its coarse chords cut across the box though the curve does not.
fn in_box(k: &Conic, t0: f64, sweep: f64, b: &Bounds, fine: f64, coarse: f64) -> Vec<Vec2> {
    let span = js_min(sweep.abs(), TAU);
    let turn = if sweep < 0.0 { -1.0 } else { 1.0 };
    let at = |s: f64| k.at(t0 + turn * s);
    if !(span > 0.0) || !(k.reach() > 0.0) {
        return vec![at(0.0)];
    }
    let (fine, coarse) = (js_max(fine, 1e-12), js_max(coarse, 1e-12));
    let stray = NEAR * k.reach() * (1.0 - cos(coarse / 2.0));
    let near = Bounds {
        min_x: b.min_x - stray,
        min_y: b.min_y - stray,
        max_x: b.max_x + stray,
        max_y: b.max_y + stray,
    };
    let mut cuts = vec![0.0, span];
    crossings(k, t0, turn, span, b, &mut cuts);
    crossings(k, t0, turn, span, &near, &mut cuts);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let close = |p: Vec2| p.x >= near.min_x && p.x <= near.max_x && p.y >= near.min_y && p.y <= near.max_y;
    // Each piece between two cuts lies wholly near the box or wholly away from it: its steps and whether it is near.
    let plan: Vec<(f64, f64, f64, bool)> = cuts
        .windows(2)
        .map(|w| {
            let near = close(at((w[0] + w[1]) / 2.0));
            let step = if near { fine } else { coarse };
            (w[0], w[1], js_max(((w[1] - w[0]) / step).ceil(), 1.0), near)
        })
        .collect();
    // Past the most, the near pieces' steps grow; the coarse ones keep theirs, or their chords could reach the box.
    let away: f64 = plan.iter().filter(|p| !p.3).map(|p| p.2).sum();
    let near_steps: f64 = plan.iter().filter(|p| p.3).map(|p| p.2).sum();
    let thin = js_max(near_steps / js_max(MOST_POINTS as f64 - away, 1.0), 1.0);
    let mut out = Vec::with_capacity((away + near_steps / thin) as usize + plan.len() + 1);
    out.push(at(0.0));
    for (u, v, n, near) in plan {
        let n = if near { js_max((n / thin).ceil(), 1.0) } else { n };
        for i in 1..=(n as usize) {
            out.push(at(u + (v - u) * i as f64 / n));
        }
    }
    out
}

/// Points of an arc (centre `c`, radius `r`, from angle `a0` over signed `sweep`; a full turn at most), both ends
/// included: in and near `b` its chords within `tol` of it, away from it 72 a turn, as the drawing draws it; one at
/// each crossing of `b`'s edge. Every point lies on the arc. The deep view's curve (plan/render.md RM-24): round to
/// a fraction of a pixel in the view.
pub fn arc_in_box(c: Vec2, r: f64, a0: f64, sweep: f64, b: &Bounds, tol: f64) -> Vec<Vec2> {
    if !(r > 0.0) {
        return vec![Vec2::new(c.x + r * cos(a0), c.y + r * sin(a0))];
    }
    let k = Conic {
        c,
        u: Vec2::new(r, 0.0),
        v: Vec2::new(0.0, r),
    };
    in_box(&k, a0, sweep, b, step_for(r, tol, COARSE_STEP), COARSE_STEP)
}

/// Points of an ellipse's arc `c + major·cos t + minor·sin t` (`geom::ellipse`'s form) from parameter `t0` over
/// signed `sweep`, both ends included, as [`arc_in_box`] takes an arc's: its chords within `tol` of it in and near
/// `b`, 72 steps a turn away from it.
pub fn ellipse_in_box(c: Vec2, major: Vec2, minor: Vec2, t0: f64, sweep: f64, b: &Bounds, tol: f64) -> Vec<Vec2> {
    let k = Conic {
        c,
        u: major,
        v: minor,
    };
    in_box(&k, t0, sweep, b, step_for(k.reach(), tol, COARSE_STEP), COARSE_STEP)
}

/// A bulged path's points (`tessellate::bulge_path`'s form: its vertices as they are, a closed one a ring whose first
/// point is not repeated), each arc's inner points those of [`arc_in_box`].
pub fn bulge_path_in_box(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, b: &Bounds, tol: f64) -> Vec<Vec2> {
    let n = pts.len();
    if n == 0 {
        return Vec::new();
    }
    let segments = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(n);
    out.push(pts[0]);
    for i in 0..segments {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        if let Some(arc) = bulge_arc(p, q, bulge_at(bulges, i)) {
            let inner = arc_in_box(arc.c, arc.r, arc.a0, arc.sweep, b, tol);
            if inner.len() > 2 {
                out.extend_from_slice(&inner[1..inner.len() - 1]);
            }
        }
        if !(closed && i + 1 == n) {
            out.push(q);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsmath::PI;

    const BOX: Bounds = Bounds {
        min_x: -1.0,
        min_y: -1.0,
        max_x: 1.0,
        max_y: 1.0,
    };

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn a_segment_through_the_box_gets_its_entry_and_its_exit() {
        let cut = split_at_box(&[v(-100.0, 0.5), v(100.0, 0.5)], false, &BOX).expect("cut");
        assert_eq!(cut, [v(-100.0, 0.5), v(-1.0, 0.5), v(1.0, 0.5), v(100.0, 0.5)]);
    }

    #[test]
    fn a_segment_from_inside_gets_its_exit_only() {
        let cut = split_at_box(&[v(0.0, 0.0), v(0.0, 50.0), v(30.0, 50.0)], false, &BOX).expect("cut");
        assert_eq!(cut, [v(0.0, 0.0), v(0.0, 1.0), v(0.0, 50.0), v(30.0, 50.0)]);
    }

    #[test]
    fn a_path_inside_or_beside_the_box_is_left_as_it_is() {
        assert_eq!(split_at_box(&[v(-0.5, 0.0), v(0.5, 0.2)], false, &BOX), None);
        assert_eq!(split_at_box(&[v(-100.0, 5.0), v(100.0, 5.0)], false, &BOX), None);
        assert_eq!(split_at_box(&[v(2.0, 0.0)], false, &BOX), None);
    }

    #[test]
    fn a_ring_s_closing_segment_is_split_once() {
        // Open ring: its last point joins its first across the box.
        let ring = [v(-10.0, 0.0), v(-10.0, -10.0), v(10.0, -10.0), v(10.0, 0.0)];
        let cut = split_at_box(&ring, true, &BOX).expect("cut");
        assert_eq!(cut[..4], ring);
        assert_eq!(cut[4..], [v(1.0, 0.0), v(-1.0, 0.0)]);
        // A ring closed by a repeated point: the same cut, no segment added.
        let mut shut = ring.to_vec();
        shut.push(ring[0]);
        let cut = split_at_box(&shut, true, &BOX).expect("cut");
        assert_eq!(cut, [&ring[..], &[v(1.0, 0.0), v(-1.0, 0.0), ring[0]]].concat());
    }

    #[test]
    fn the_points_put_in_lie_on_the_segment() {
        // Through (0.3, 0.2), far ends on both sides.
        let (p, q) = (v(-1234.5, -987.25), v(2099.46, 1678.865));
        let cut = split_at_box(&[p, q], false, &BOX).expect("cut");
        assert_eq!(cut.len(), 4);
        for c in &cut[1..3] {
            let cross = (q.x - p.x) * (c.y - p.y) - (q.y - p.y) * (c.x - p.x);
            let length = ((q.x - p.x).powi(2) + (q.y - p.y).powi(2)).sqrt();
            assert!((cross / length).abs() < 1e-9, "{c:?} is off the segment by {}", cross / length);
            assert!(c.x.abs() <= 1.0 + 1e-9 && c.y.abs() <= 1.0 + 1e-9, "{c:?} on the box's edge");
        }
    }

    #[test]
    fn touching_a_corner_puts_in_one_point() {
        let cut = split_at_box(&[v(-2.0, 0.0), v(0.0, 2.0)], false, &BOX).expect("cut");
        assert_eq!(cut, [v(-2.0, 0.0), v(-1.0, 1.0), v(0.0, 2.0)]);
    }

    /// The largest distance from the arc of the chords between `pts` (all on the circle): their sagittas.
    fn sagitta(c: Vec2, r: f64, pts: &[Vec2], within: Option<&Bounds>) -> f64 {
        pts.windows(2)
            .filter(|w| {
                within.is_none_or(|b| {
                    let m = v((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
                    m.x >= b.min_x && m.x <= b.max_x && m.y >= b.min_y && m.y <= b.max_y
                })
            })
            .map(|w| {
                let m = v((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
                r - ((m.x - c.x).powi(2) + (m.y - c.y).powi(2)).sqrt()
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_circle_through_the_box_is_fine_in_it_and_coarse_round_it() {
        // A 50 m circle whose right edge passes through a 2 m box; a tenth of a millimetre in it.
        let (c, r) = (v(-49.5, 0.3), 50.0);
        let (tol, b) = (1e-4, BOX);
        let pts = arc_in_box(c, r, 0.0, TAU, &b, tol);
        for p in &pts {
            let d = ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt();
            assert!((d - r).abs() < 1e-9, "{p:?} lies on the circle");
        }
        assert!(sagitta(c, r, &pts, Some(&b)) <= tol * 1.000_001, "round in the box");
        // Round it, as coarse as the 72 chords of a full turn; a handful of points in and near the box.
        let coarse = r * (1.0 - cos(COARSE_STEP / 2.0));
        assert!(sagitta(c, r, &pts, None) <= coarse * 1.000_001);
        assert!(pts.len() < 72 + 80, "{} points", pts.len());
        // The two crossings of the box's edge are points of it, on the edge.
        let on_edge = pts.iter().filter(|p| (p.y.abs() - 1.0).abs() < 1e-9 && p.x.abs() <= 1.0).count();
        assert_eq!(on_edge, 2);
        // A full turn: its last point is its first.
        let (first, last) = (pts[0], pts[pts.len() - 1]);
        assert!((first.x - last.x).abs() < 1e-9 && (first.y - last.y).abs() < 1e-9);
    }

    #[test]
    fn a_curve_passing_beside_the_box_never_has_a_chord_across_it() {
        // A 51.52 m circle whose top passes 2 cm over the box, the box inside it: its coarse chords (5 cm from it at
        // their middles) would cut across the box's upper corners, a straight line through the view where the
        // circle is not. Near the box it is drawn finely; no chord touches the box.
        let (c, r) = (v(0.0, -50.5), 51.52);
        let pts = arc_in_box(c, r, 0.0, TAU, &BOX, 1e-4);
        assert!(pts.iter().all(|p| !(p.x.abs() <= 1.0 && p.y.abs() <= 1.0)), "the circle misses the box");
        let across = pts.windows(2).filter(|w| crossing(w[0], w[1], &BOX).is_some()).count();
        assert_eq!(across, 0, "chords across the box");
        // The 72 chords of a full turn, the coarse ones that would cross it among them.
        let coarse: Vec<Vec2> = (0..=72).map(|i| v(c.x + r * cos(i as f64 * COARSE_STEP), c.y + r * sin(i as f64 * COARSE_STEP))).collect();
        assert!(coarse.windows(2).any(|w| crossing(w[0], w[1], &BOX).is_some()));
        // An ellipse the same: its top 1.5 cm over the box, 5 mm over its corners.
        let pts = ellipse_in_box(c, v(r, 0.0), v(0.0, r * 0.9999), 0.0, TAU, &BOX, 1e-4);
        assert!(pts.iter().all(|p| !(p.x.abs() <= 1.0 && p.y.abs() <= 1.0)), "the ellipse misses the box");
        assert_eq!(pts.windows(2).filter(|w| crossing(w[0], w[1], &BOX).is_some()).count(), 0, "chords across the box");
    }

    #[test]
    fn an_arc_keeps_its_ends_and_its_direction() {
        // A quarter turn clockwise from 90° to 0° on a circle round the origin of radius 1.5: through the box's corner
        // region.
        let pts = arc_in_box(v(0.0, 0.0), 1.5, PI / 2.0, -PI / 2.0, &BOX, 1e-5);
        assert!((pts[0].x).abs() < 1e-12 && (pts[0].y - 1.5).abs() < 1e-12);
        let end = pts[pts.len() - 1];
        assert!((end.x - 1.5).abs() < 1e-12 && end.y.abs() < 1e-12);
        // Clockwise: x grows along it.
        assert!(pts.windows(2).all(|w| w[1].x >= w[0].x - 1e-12));
    }

    #[test]
    fn a_circle_away_from_the_box_is_drawn_as_before() {
        let pts = arc_in_box(v(100.0, 100.0), 10.0, 0.0, TAU, &BOX, 1e-6);
        assert_eq!(pts.len(), 73, "72 chords, the first point again at the end");
    }

    #[test]
    fn a_box_asking_for_too_many_points_thins_them() {
        // A circle inside the box at a picometre: more than the most, thinned to it.
        let pts = arc_in_box(v(0.0, 0.0), 0.9, 0.0, TAU, &BOX, 1e-12);
        assert!(pts.len() <= MOST_POINTS + 2, "{}", pts.len());
        assert!(pts.len() > MOST_POINTS / 2);
    }

    #[test]
    fn an_ellipse_is_round_in_the_box_and_on_its_curve() {
        // A 40 × 10 m ellipse turned 30°, its end of the major axis through the box.
        let a = PI / 6.0;
        let major = v(40.0 * cos(a), 40.0 * sin(a));
        let minor = v(-major.y * 0.25, major.x * 0.25);
        let c = v(-major.x + 0.2, -major.y + 0.1);
        let tol = 1e-4;
        let pts = ellipse_in_box(c, major, minor, 0.0, TAU, &BOX, tol);
        // Every point on it: back in its own frame, on the unit circle.
        let det = major.x * minor.y - major.y * minor.x;
        let unit = |p: Vec2| {
            let (dx, dy) = (p.x - c.x, p.y - c.y);
            v((minor.y * dx - minor.x * dy) / det, (-major.y * dx + major.x * dy) / det)
        };
        for p in &pts {
            let q = unit(*p);
            assert!((q.x.hypot(q.y) - 1.0).abs() < 1e-12, "{p:?} on the ellipse");
        }
        // In the box its chords stray at most `tol`: the distance of a chord's middle from the curve, measured along
        // the curve's normal there, is below the sagitta of its circle of reach.
        let mut worst: f64 = 0.0;
        for w in pts.windows(2) {
            let m = v((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
            if m.x.abs() <= 1.0 && m.y.abs() <= 1.0 {
                let (t0, t1) = (atan2(unit(w[0]).y, unit(w[0]).x), atan2(unit(w[1]).y, unit(w[1]).x));
                let tm = t0 + norm_angle(t1 - t0) / 2.0;
                let on = v(c.x + major.x * cos(tm) + minor.x * sin(tm), c.y + major.y * cos(tm) + minor.y * sin(tm));
                worst = worst.max((on.x - m.x).hypot(on.y - m.y));
            }
        }
        assert!(worst > 0.0 && worst <= tol * 1.000_001, "{worst}");
        // Away from it 72 steps a turn: far fewer points than finely all round.
        assert!(pts.len() < 72 + 200, "{}", pts.len());
    }

    #[test]
    fn a_bulged_path_keeps_its_vertices_and_its_arcs_are_round_in_the_box() {
        // A half circle of radius 20 m bulging right through the box, then a straight edge back: closed.
        let pts = [v(-19.5, -20.0), v(-19.5, 20.0)];
        let bulges = [1.0, 0.0];
        let ring = bulge_path_in_box(&pts, Some(&bulges), true, &BOX, 1e-4);
        assert_eq!(ring[0], pts[0]);
        assert!(ring.contains(&pts[1]));
        assert_ne!(ring[ring.len() - 1], ring[0], "a ring: its first point not repeated");
        // Every inner point on the arc: centre (−19.5, 0), radius 20.
        let between = &ring[1..ring.iter().position(|p| *p == pts[1]).expect("second vertex")];
        assert!(between.len() > 36, "{}", between.len());
        for p in between {
            assert!(((p.x + 19.5).hypot(p.y) - 20.0).abs() < 1e-9, "{p:?}");
        }
        let c = v(-19.5, 0.0);
        assert!(sagitta(c, 20.0, between, Some(&BOX)) <= 1e-4 * 1.000_001);
        // Straight edges and a path without bulges: the vertices alone.
        assert_eq!(bulge_path_in_box(&pts, None, false, &BOX, 1e-4), pts);
    }
}
