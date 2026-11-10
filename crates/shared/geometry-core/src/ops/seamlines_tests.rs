//! The seamline geometry (docs/adr/4700) on footprints worked by hand: two strips side by side (the seam the overlap's
//! middle line, its ends on the shared edges), two staggered strips (Pan et al. 2009's AEFC: from one crossing along
//! the corners' bisectors to the other), the polygons' areas, the faults, and the grid path's bottleneck.

use super::*;
use crate::jsmath::js_min;

const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

fn at(x: f64, y: f64) -> Pt {
    [X0 + x, Y0 + y]
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Pt> {
    vec![at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)]
}

fn near(a: &[Pt], b: &[Pt]) {
    assert_eq!(a.len(), b.len(), "{a:?} {b:?}");
    for (p, q) in a.iter().zip(b) {
        assert!((p[0] - q[0]).abs() < 1e-7 && (p[1] - q[1]).abs() < 1e-7, "{a:?} {b:?}");
    }
}

#[test]
fn two_strips_side_by_side_meet_at_the_overlaps_middle() {
    let fp = Footprints::new(&rect(500.0, 108.0, 520.0, 120.0), &rect(510.0, 108.0, 530.0, 120.0)).expect("a seam");
    assert_eq!(fp.kinds(), &[EdgeKind::Shared, EdgeKind::SecondEnd, EdgeKind::Shared, EdgeKind::FirstEnd]);
    let [(s, sc), (e, ec)] = fp.ends();
    near(&[s, e], &[at(515.0, 108.0), at(515.0, 120.0)]);
    assert!(!sc && !ec, "both on shared edges");
    let seam = fp.voronoi(1.0);
    near(&seam, &[at(515.0, 108.0), at(515.0, 120.0)]);
    let [a, b] = fp.rings(&seam);
    near(&a, &[at(515.0, 120.0), at(500.0, 120.0), at(500.0, 108.0), at(515.0, 108.0)]);
    near(&b, &[at(515.0, 108.0), at(530.0, 108.0), at(530.0, 120.0), at(515.0, 120.0)]);
    assert!((ring_area(&a) - 180.0).abs() < 1e-6 && (ring_area(&b) - 180.0).abs() < 1e-6);
    assert!((fp.overlap_area() - 120.0).abs() < 1e-9);
    assert!(simple(&a) && simple(&b));
}

#[test]
fn staggered_strips_take_the_corners_bisectors() {
    // Pan et al. 2009: the quadrilateral overlap ABCD, A and C the crossings; A's bisector meets D's at E, C's meets
    // B's at F; AEFC is the seam.
    let fp = Footprints::new(&rect(500.0, 108.0, 520.0, 120.0), &rect(510.0, 105.0, 530.0, 117.0)).expect("a seam");
    let [(s, sc), (e, ec)] = fp.ends();
    near(&[s, e], &[at(510.0, 108.0), at(520.0, 117.0)]);
    assert!(sc && ec, "both where the boundaries cross");
    for step in [0.25, 1.0, 5.0, 50.0] {
        let seam = fp.voronoi(step);
        near(&seam, &[at(510.0, 108.0), at(514.5, 112.5), at(515.5, 112.5), at(520.0, 117.0)]);
    }
    let seam = fp.voronoi(1.0);
    let [a, b] = fp.rings(&seam);
    let total = fp.area(0) + fp.area(1) - fp.overlap_area();
    assert!((ring_area(&a) + ring_area(&b) - total).abs() < 1e-6);
    assert!(simple(&a) && simple(&b));
    // Every seam point is as far from the first's end as from the second's.
    for p in &seam[1..seam.len() - 1] {
        let q = [p[0] - X0, p[1] - Y0];
        let d1 = js_min(to_polyline(q, &[[510.0, 117.0], [510.0, 108.0]]), to_polyline(q, &[[520.0, 117.0], [510.0, 117.0]]));
        let d2 = js_min(to_polyline(q, &[[510.0, 108.0], [520.0, 108.0]]), to_polyline(q, &[[520.0, 108.0], [520.0, 117.0]]));
        assert!((d1 - d2).abs() < 1e-9, "{p:?}: {d1} {d2}");
    }
}

#[test]
fn faults() {
    let a = rect(500.0, 108.0, 520.0, 120.0);
    assert_eq!(Footprints::new(&a, &rect(530.0, 108.0, 540.0, 120.0)).err(), Some(SeamFault::Apart));
    assert_eq!(Footprints::new(&a, &rect(520.0, 108.0, 540.0, 120.0)).err(), Some(SeamFault::Apart), "touching");
    assert_eq!(Footprints::new(&a, &rect(505.0, 110.0, 515.0, 118.0)).err(), Some(SeamFault::Inside(1)));
    assert_eq!(Footprints::new(&rect(505.0, 110.0, 515.0, 118.0), &a).err(), Some(SeamFault::Inside(0)));
    assert_eq!(Footprints::new(&a, &a).err(), Some(SeamFault::Inside(0)), "the same");
    assert_eq!(
        Footprints::new(&rect(500.0, 110.0, 530.0, 120.0), &rect(510.0, 100.0, 520.0, 130.0)).err(),
        Some(SeamFault::Crossed)
    );
    let notch = vec![at(500.0, 108.0), at(520.0, 108.0), at(510.0, 112.0), at(520.0, 120.0), at(500.0, 120.0)];
    assert_eq!(Footprints::new(&notch, &a).err(), Some(SeamFault::NotConvex(0)));
    assert_eq!(Footprints::new(&a, &[at(500.0, 108.0), at(510.0, 108.0)]).err(), Some(SeamFault::Degenerate(1)));
}

#[test]
fn a_footprint_given_clockwise_with_its_closing_point_and_a_straight_corner_is_the_same() {
    let mut cw = vec![at(510.0, 108.0), at(510.0, 120.0), at(530.0, 120.0), at(530.0, 114.0), at(530.0, 108.0)];
    cw.push(cw[0]);
    let fp = Footprints::new(&rect(500.0, 108.0, 520.0, 120.0), &cw).expect("a seam");
    near(&fp.voronoi(1.0), &[at(515.0, 108.0), at(515.0, 120.0)]);
}

#[test]
fn the_grid_path_passes_the_walls_gap_at_its_bottleneck() {
    // 5 × 5: every cell 1 but column 2, a wall of 9 with a gap of 5 at the last row.
    let (w, h) = (5, 5);
    let mut d = vec![1.0; w * h];
    for r in 0..h {
        d[r * w + 2] = if r == 4 { 5.0 } else { 9.0 };
    }
    let p = grid_path(w, h, &d, 0, 4).expect("a path");
    assert_eq!(p.bottleneck, 5.0);
    assert_eq!(p.cells.first(), Some(&0));
    assert_eq!(p.cells.last(), Some(&4));
    assert!(p.cells.contains(&22), "{:?}", p.cells);
    assert!(p.cells.iter().all(|&c| d[c] <= 5.0));
    // A wall not searched: no path.
    for r in 0..h {
        d[r * w + 2] = f64::NAN;
    }
    assert_eq!(grid_path(w, h, &d, 0, 4), None);
    // Its turns.
    assert_eq!(turns(&[0, 1, 2, 7, 12], 5), vec![0, 2, 12]);
}
