//! Deep zoom (plan/render.md RM-24): a path's segments split where they cross a box round the view.
//!
//! The GPU takes a point's offset from the camera to float32 parts, good to about 2⁻²⁴ of its distance from the
//! camera (crates/render/wgpu/src/precision.rs). A segment's position near the camera is as good as its ends: a
//! segment through the view whose ends lie 64 m away is off by 4 µm, 4 pixels at 10⁶ px/m. Split where it crosses a
//! box round the view, the piece inside has both its ends in the box, and is as good as the box is small.
//!
//! The points put in lie on the segment, so the path draws the same: its length, a line type's dashes along it,
//! its marks, its area and the triangles of its fill (their edges stay shared) are unchanged.

use crate::Vec2;
use crate::geom::arc::norm_angle;
use crate::geometry::Bounds;
use crate::jsmath::{PI, TAU, acos, asin, cos, sin};

/// The coarse step of a deep view's curve round its box: a full turn in 72 chords, as the drawing draws it.
pub const COARSE_STEP: f64 = TAU / 72.0;

/// Most points a deep view's curve takes: its steps grow when a box far larger than its tolerance asks for more.
pub const MOST_POINTS: usize = 65_536;

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
            t0 = t0.max(t);
        } else {
            if t < t0 {
                return None;
            }
            t1 = t1.min(t);
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
    (2.0 * acos(1.0 - tol / r)).min(most)
}

/// Where the arc crosses the edge of `b`: its angles from `a0` in the arc's direction (`turn` ±1), within (0, `span`).
fn crossings(c: Vec2, r: f64, a0: f64, turn: f64, span: f64, b: &Bounds, out: &mut Vec<f64>) {
    let mut put = |theta: f64| {
        let s = norm_angle(turn * (theta - a0));
        if s > 0.0 && s < span {
            out.push(s);
        }
    };
    for x in [b.min_x, b.max_x] {
        let d = (x - c.x) / r;
        if d.abs() <= 1.0 {
            let t = acos(d);
            for theta in [t, -t] {
                let y = c.y + r * sin(theta);
                if y >= b.min_y && y <= b.max_y {
                    put(theta);
                }
            }
        }
    }
    for y in [b.min_y, b.max_y] {
        let d = (y - c.y) / r;
        if d.abs() <= 1.0 {
            let t = asin(d);
            for theta in [t, PI - t] {
                let x = c.x + r * cos(theta);
                if x >= b.min_x && x <= b.max_x {
                    put(theta);
                }
            }
        }
    }
}

/// Points of an arc (centre `c`, radius `r`, from angle `a0` over signed `sweep`; a full turn at most), both ends
/// included: in `b` at most `fine` apart in angle, out of it at most `coarse`, and one at each crossing of `b`'s edge.
/// Every point lies on the arc. The deep view's curve (plan/render.md RM-24): round to a fraction of a pixel in the
/// view, as coarse as the drawing draws it round the view.
pub fn arc_in_box(c: Vec2, r: f64, a0: f64, sweep: f64, b: &Bounds, fine: f64, coarse: f64) -> Vec<Vec2> {
    let span = sweep.abs().min(TAU);
    let turn = if sweep < 0.0 { -1.0 } else { 1.0 };
    let at = |s: f64| {
        let a = a0 + turn * s;
        Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
    };
    if !(span > 0.0) || !(r > 0.0) {
        return vec![at(0.0)];
    }
    let mut cuts = vec![0.0, span];
    crossings(c, r, a0, turn, span, b, &mut cuts);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let inside = |p: Vec2| p.x >= b.min_x && p.x <= b.max_x && p.y >= b.min_y && p.y <= b.max_y;
    let (fine, coarse) = (fine.max(1e-12), coarse.max(1e-12));
    let plan: Vec<(f64, f64, f64)> = cuts
        .windows(2)
        .map(|w| {
            let step = if inside(at((w[0] + w[1]) / 2.0)) { fine } else { coarse };
            (w[0], w[1], ((w[1] - w[0]) / step).ceil().max(1.0))
        })
        .collect();
    let total: f64 = plan.iter().map(|p| p.2).sum();
    let thin = (total / MOST_POINTS as f64).max(1.0);
    let mut out = Vec::with_capacity(total.min(MOST_POINTS as f64) as usize + plan.len() + 1);
    out.push(at(0.0));
    for (u, v, n) in plan {
        let n = (n / thin).ceil().max(1.0);
        for k in 1..=(n as usize) {
            out.push(at(u + (v - u) * k as f64 / n));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let pts = arc_in_box(c, r, 0.0, TAU, &b, step_for(r, tol, COARSE_STEP), COARSE_STEP);
        for p in &pts {
            let d = ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt();
            assert!((d - r).abs() < 1e-9, "{p:?} lies on the circle");
        }
        assert!(sagitta(c, r, &pts, Some(&b)) <= tol * 1.000_001, "round in the box");
        // Round it, as coarse as the 72 chords of a full turn; a handful of points in the box.
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
    fn an_arc_keeps_its_ends_and_its_direction() {
        // A quarter turn clockwise from 90° to 0° on a circle round the origin of radius 1.5: through the box's corner
        // region.
        let pts = arc_in_box(v(0.0, 0.0), 1.5, PI / 2.0, -PI / 2.0, &BOX, 0.01, COARSE_STEP);
        assert!((pts[0].x).abs() < 1e-12 && (pts[0].y - 1.5).abs() < 1e-12);
        let end = pts[pts.len() - 1];
        assert!((end.x - 1.5).abs() < 1e-12 && end.y.abs() < 1e-12);
        // Clockwise: x grows along it.
        assert!(pts.windows(2).all(|w| w[1].x >= w[0].x - 1e-12));
    }

    #[test]
    fn a_circle_away_from_the_box_is_drawn_as_before() {
        let pts = arc_in_box(v(100.0, 100.0), 10.0, 0.0, TAU, &BOX, 1e-6, COARSE_STEP);
        assert_eq!(pts.len(), 73, "72 chords, the first point again at the end");
    }

    #[test]
    fn a_box_asking_for_too_many_points_thins_them() {
        // A circle inside the box at a nanometre: more than the most, thinned to it.
        let pts = arc_in_box(v(0.0, 0.0), 0.9, 0.0, TAU, &BOX, step_for(0.9, 1e-12, COARSE_STEP), COARSE_STEP);
        assert!(pts.len() <= MOST_POINTS + 2, "{}", pts.len());
        assert!(pts.len() > MOST_POINTS / 2);
    }
}
