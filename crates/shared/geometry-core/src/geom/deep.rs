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
use crate::geometry::Bounds;

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
}
