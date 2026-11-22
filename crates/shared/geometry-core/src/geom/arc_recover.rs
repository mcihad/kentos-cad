//! Arcs taken back from a flattened outline (docs/adr/8071 §1). A wall's body is built from its lines, whose arcs are
//! exact, but its rings are those lines flattened at [`DEFAULT_STEP`] (`bulge_path_outline`: at least two chords an
//! arc, none over 5°). A writer that can draw arcs (an IFC profile's `IfcArcIndex`) takes each arc back from the ring:
//! a run of consecutive vertices on one of the drawing's circles, each step a chord of at most the step turning the same
//! way, is one arc through its middle vertex. Three points on a circle are never collinear, so a run is never a straight
//! line's; a straight cut across a circle is longer than the step and breaks the run.

use crate::Vec2;
use crate::geom::arc::DEFAULT_STEP;
use crate::geom::bulge::bulge_via;
use crate::jsmath::{atan2, js_hypot, tan};

/// A circle an outline's arcs may lie on: its centre and radius.
pub type Circle = (Vec2, f64);

/// How near a vertex must lie to a circle to be on it, relative to the radius (at least 1 m): the flattening's own
/// rounding, far below a drawing's tolerance.
pub const ON_CIRCLE: f64 = 1e-9;

fn on(p: Vec2, c: &Circle) -> bool {
    (js_hypot(p.x - c.0.x, p.y - c.0.y) - c.1).abs() <= ON_CIRCLE * crate::jsmath::js_max(c.1, 1.0)
}

/// The edge from `a` to `b` as a chord of one of `circles`: the circle and the way it turns (+1 counter-clockwise);
/// none when it is no chord of at most the step.
fn chord(a: Vec2, b: Vec2, circles: &[Circle]) -> Option<(usize, i8)> {
    circles.iter().enumerate().find_map(|(k, c)| {
        if !(on(a, c) && on(b, c)) {
            return None;
        }
        let mut d = atan2(b.y - c.0.y, b.x - c.0.x) - atan2(a.y - c.0.y, a.x - c.0.x);
        while d > std::f64::consts::PI {
            d -= std::f64::consts::TAU;
        }
        while d < -std::f64::consts::PI {
            d += std::f64::consts::TAU;
        }
        (d.abs() > 1e-12 && d.abs() <= DEFAULT_STEP * (1.0 + 1e-6))
            .then_some((k, if d > 0.0 { 1 } else { -1 }))
    })
}

/// The ring `pts` (an outline flattened at [`DEFAULT_STEP`] or finer, its first point not repeated at its end) with each
/// run of two or more chords of one of `circles`, turning the same way, taken back as one arc through the run's middle
/// vertex: the ring's points and each edge's bulge (to the next point). A run round the whole circle is two arcs. A ring
/// with no such run comes back as it was, its bulges 0.
pub fn recover(pts: &[Vec2], circles: &[Circle]) -> (Vec<Vec2>, Vec<f64>) {
    let n = pts.len();
    if n < 3 || circles.is_empty() {
        return (pts.to_vec(), vec![0.0; n]);
    }
    let edges: Vec<Option<(usize, i8)>> = (0..n)
        .map(|i| chord(pts[i], pts[(i + 1) % n], circles))
        .collect();
    if edges.iter().all(Option::is_none) {
        return (pts.to_vec(), vec![0.0; n]);
    }
    // Start where a run starts (or at any edge when one run goes round the whole ring): no run is split at the end.
    let whole = edges.iter().all(|e| e.is_some() && *e == edges[0]);
    let start = if whole {
        0
    } else {
        (0..n)
            .find(|&i| edges[i] != edges[(i + n - 1) % n])
            .unwrap_or(0)
    };
    let mut runs: Vec<(usize, usize)> = Vec::new(); // (first edge, edge count), along the ring from `start`
    let mut k = 0;
    while k < n {
        let i = (start + k) % n;
        let mut len = 1;
        if edges[i].is_some() {
            while k + len < n && edges[(start + k + len) % n] == edges[i] {
                len += 1;
            }
        }
        runs.push((i, len));
        k += len;
    }
    if whole {
        // Round the whole circle: two arcs.
        runs = vec![(0, n / 2), (n / 2, n - n / 2)];
    }
    let mut out_pts = Vec::new();
    let mut out_b = Vec::new();
    for (first, len) in runs {
        let a = pts[first];
        if len >= 2 && edges[first].is_some() {
            let b = pts[(first + len) % n];
            let m = pts[(first + len / 2) % n];
            match bulge_via(a, m, b) {
                Some(bulge) => {
                    out_pts.push(a);
                    out_b.push(bulge);
                }
                None => {
                    for j in 0..len {
                        out_pts.push(pts[(first + j) % n]);
                        out_b.push(0.0);
                    }
                }
            }
        } else {
            for j in 0..len {
                out_pts.push(pts[(first + j) % n]);
                out_b.push(0.0);
            }
        }
    }
    (out_pts, out_b)
}

/// The bulges of a ring whose corners may lie on `circles` (docs/adr/8071 §1): an edge between two points of one circle
/// is that circle's shorter arc between them, every other edge straight (0). For an outline whose corners are on a
/// curved wall's faces (an opening's jambs on both): its edges along the faces are their arcs, its jambs straight.
pub fn along(pts: &[Vec2], circles: &[Circle]) -> Vec<f64> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            circles
                .iter()
                .find(|c| on(a, c) && on(b, c))
                .map_or(0.0, |c| {
                    let mut d = atan2(b.y - c.0.y, b.x - c.0.x) - atan2(a.y - c.0.y, a.x - c.0.x);
                    while d > std::f64::consts::PI {
                        d -= std::f64::consts::TAU;
                    }
                    while d <= -std::f64::consts::PI {
                        d += std::f64::consts::TAU;
                    }
                    if d.abs() <= 1e-12 { 0.0 } else { tan(d / 4.0) }
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::bulge::{bulge_path_outline, bulge_ring_area};

    #[test]
    fn a_flattened_quarter_annulus_comes_back_with_its_two_arcs() {
        // A 90° piece of a ring between radii 5.85 and 6.15 round the origin: its outer arc counter-clockwise from
        // (6.15, 0) to (0, 6.15), its inner arc back, the ends straight. Exact area π(6.15² − 5.85²)/4 = 0.9π.
        let (r1, r2) = (6.15, 5.85);
        let b = crate::jsmath::tan(std::f64::consts::PI / 8.0);
        let pts = vec![
            Vec2::new(r1, 0.0),
            Vec2::new(0.0, r1),
            Vec2::new(0.0, r2),
            Vec2::new(r2, 0.0),
        ];
        let bulges = vec![b, 0.0, -b, 0.0];
        let flat = bulge_path_outline(&pts, Some(&bulges), true, DEFAULT_STEP);
        assert!(flat.len() > 30, "flattened: {}", flat.len());
        let circles = [(Vec2::new(0.0, 0.0), r1), (Vec2::new(0.0, 0.0), r2)];
        let (back, bulges_back) = recover(&flat, &circles);
        assert_eq!(back.len(), 4, "{back:?}");
        let exact = 0.9 * std::f64::consts::PI;
        assert!((bulge_ring_area(&back, Some(&bulges_back)).abs() - exact).abs() < 1e-12);
        // The flattened ring's own area falls short by the chords' segments.
        assert!(exact - bulge_ring_area(&flat, None).abs() > 1e-4);
    }

    #[test]
    fn an_opening_through_a_curved_wall_runs_along_its_faces_arcs() {
        // Jambs at 30° and 40° through a wall between radii 5.85 and 6.15: its faces' edges are 10° arcs, the jambs
        // straight; its area the ring's piece, (6.15² − 5.85²)/2 · 10°.
        let (r1, r2) = (6.15, 5.85);
        let at =
            |r: f64, deg: f64| Vec2::new(r * crate::jsmath::cos(deg.to_radians()), r * crate::jsmath::sin(deg.to_radians()));
        let ring = vec![at(r1, 30.0), at(r2, 30.0), at(r2, 40.0), at(r1, 40.0)];
        let circles = [(Vec2::new(0.0, 0.0), r1), (Vec2::new(0.0, 0.0), r2)];
        let b = along(&ring, &circles);
        assert_eq!(b[0], 0.0);
        assert_eq!(b[2], 0.0);
        assert!(b[1] > 0.0 && b[3] < 0.0, "{b:?}");
        let exact = (r1 * r1 - r2 * r2) / 2.0 * 10f64.to_radians();
        assert!((bulge_ring_area(&ring, Some(&b)).abs() - exact).abs() < 1e-12);
    }

    #[test]
    fn a_whole_circle_is_two_arcs_and_a_straight_ring_stays() {
        let r = 2.0;
        let flat = bulge_path_outline(
            &[Vec2::new(r, 0.0), Vec2::new(-r, 0.0)],
            Some(&[1.0, 1.0]),
            true,
            DEFAULT_STEP,
        );
        let (back, b) = recover(&flat, &[(Vec2::new(0.0, 0.0), r)]);
        assert_eq!(back.len(), 2);
        assert!(
            (bulge_ring_area(&back, Some(&b)).abs() - std::f64::consts::PI * r * r).abs() < 1e-12
        );
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let (same, zeros) = recover(&square, &[(Vec2::new(0.5, 0.5), 0.5_f64.sqrt())]);
        assert_eq!(
            same, square,
            "a square's corners are on its circumcircle, but its sides are no 5° chords"
        );
        assert_eq!(zeros, vec![0.0; 4]);
    }
}
