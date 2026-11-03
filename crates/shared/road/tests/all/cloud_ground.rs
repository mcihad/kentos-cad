//! A terrain's points from a point cloud (src/cloud_ground.rs, docs/adr/0913): the thinning's three rules, the order
//! they do not depend on, the coverage of an area and of the cell box, the error against the kept points' surface, and
//! the refusals.

use kentos_road::cloud_ground::{self, Thinner, Thinning, ThinningError};
use kentos_road::surface::{self, SpotPoint, SurfaceInput};

fn rule(cell: f64) -> Thinning {
    Thinning { cell, z_tolerance: None, near: None }
}

/// A 10 × 10 lattice at 0.5 m on [0.25, 4.75]², z = 100 + 0.1 x.
fn lattice() -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for j in 0..10 {
        for i in 0..10 {
            let (x, y) = (0.25 + 0.5 * f64::from(i), 0.25 + 0.5 * f64::from(j));
            out.push([x, y, 100.0 + 0.1 * x]);
        }
    }
    out
}

fn thin(rule: Thinning, lines: &[Vec<[f64; 2]>], points: &[[f64; 3]]) -> cloud_ground::Thinned {
    let mut t = Thinner::new(rule, lines).expect("a rule");
    for p in points {
        t.feed(*p);
    }
    t.finish()
}

#[test]
fn a_cell_keeps_the_point_nearest_its_centre() {
    let got = thin(rule(1.0), &[], &lattice());
    // 25 cells of 1 m, each holding four points at 0.25 m from its centre: the first read is kept.
    assert_eq!((got.read, got.cells, got.points.len()), (100, 25, 25));
    assert_eq!(got.by_cell, 25);
    assert_eq!(got.points[0].1, [0.25, 0.25, 100.025]);
    assert!(got.points.windows(2).all(|w| w[0].0 < w[1].0));
    assert_eq!(got.bounds, Some([0.25, 0.25, 4.75, 4.75]));
}

#[test]
fn the_kept_points_do_not_depend_on_the_order_but_ties_go_to_the_first() {
    let mut pts: Vec<[f64; 3]> = lattice();
    // Move each cell's third point to its centre: it wins wherever it comes.
    for p in pts.iter_mut().skip(10).step_by(20) {
        p[0] = (p[0] / 1.0).floor() + 0.5;
        p[1] = (p[1] / 1.0).floor() + 0.5;
    }
    let ahead = thin(rule(1.0), &[], &pts);
    let mut back = pts.clone();
    back.reverse();
    let behind = thin(rule(1.0), &[], &back);
    let set = |t: &cloud_ground::Thinned| {
        let mut v: Vec<[i64; 3]> = t.points.iter().map(|(_, p)| p.map(|c| (c * 1e6).round() as i64)).collect();
        v.sort_unstable();
        v
    };
    assert_eq!(set(&ahead).len(), 25);
    let moved: Vec<[f64; 3]> = pts.iter().skip(10).step_by(20).copied().collect();
    for m in &moved {
        assert!(ahead.points.iter().any(|(_, p)| p == m), "{m:?}");
        assert!(behind.points.iter().any(|(_, p)| p == m), "{m:?}");
    }
}

#[test]
fn a_steep_cell_keeps_its_foot_and_top() {
    let mut pts = lattice();
    // A 0.6 m step at x = 2.5: the cells across it spread more than 0.5 m.
    for p in &mut pts {
        if p[0] > 2.5 {
            p[2] += 0.6;
        }
    }
    let flat = thin(Thinning { z_tolerance: Some(0.5), ..rule(1.0) }, &[], &lattice());
    assert_eq!((flat.points.len(), flat.by_slope), (25, 0));
    let stepped = thin(Thinning { z_tolerance: Some(0.5), ..rule(1.0) }, &[], &pts);
    // The five cells of the column x ∈ [2, 3) keep their highest besides their nearest (their lowest, the first read).
    assert_eq!(stepped.by_slope, 5, "{stepped:?}");
    assert_eq!(stepped.points.len(), 30);
    assert!(stepped.points.iter().any(|(_, p)| *p == [2.75, 0.25, 100.875]));
}

#[test]
fn every_point_beside_a_breakline_is_kept() {
    let line = vec![vec![[0.0, 2.5], [5.0, 2.5]]];
    let got = thin(Thinning { near: Some(0.3), ..rule(1.0) }, &line, &lattice());
    // The rows y = 2.25 and 2.75 are 0.25 m from the line: 20 points, 15 of them not a cell's own.
    assert_eq!(got.by_line, 15, "{got:?}");
    assert_eq!(got.points.len(), 40);
    let none = thin(rule(1.0), &line, &lattice());
    assert_eq!(none.points.len(), 25);
}

#[test]
fn the_coverage_of_an_area_and_of_the_cell_box() {
    let mut pts = lattice();
    pts.retain(|p| !(p[0] > 3.0 && p[1] > 3.0));
    let got = thin(rule(1.0), &[], &pts);
    let boxed = cloud_ground::coverage(&got, 1.0, None).expect("points");
    // The box is 5 × 5 cells; four of them are empty.
    assert_eq!((boxed.cells, boxed.occupied), (25, 21));
    assert!((boxed.share - 0.84).abs() < 1e-12);
    assert!((boxed.area - 25.0).abs() < 1e-12);
    assert!((boxed.density - 84.0 / 25.0).abs() < 1e-12);
    let ring = [[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0]];
    let area = cloud_ground::coverage(&got, 1.0, Some(&ring)).expect("an area");
    assert_eq!((area.cells, area.occupied), (50, 21));
    assert!((area.area - 50.0).abs() < 1e-12);
    assert!(cloud_ground::coverage(&thin(rule(1.0), &[], &[]), 1.0, None).is_none());
}

#[test]
fn the_error_of_the_points_left_out() {
    // On a plane every left out point is on the kept points' surface, where it has one.
    let pts = lattice();
    let got = thin(rule(1.0), &[], &pts);
    let kept: std::collections::HashSet<u64> = got.points.iter().map(|x| x.0).collect();
    let points = got.points.iter().map(|(_, p)| SpotPoint { x: p[0], y: p[1], z: p[2], source: None }).collect();
    let s = surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds");
    let left = pts.iter().enumerate().filter(|(i, _)| !kept.contains(&(*i as u64))).map(|(_, p)| *p);
    let r = cloud_ground::residuals(&s, left);
    assert_eq!(r.count + r.outside, 75);
    assert!(r.outside > 0 && r.count > 0, "{r:?}");
    assert!(r.max < 1e-9 && r.rms < 1e-9, "{r:?}");
    // A bump the kept points miss: its height is the error.
    let mut bumped = pts.clone();
    bumped[44][2] += 0.3;
    let r = cloud_ground::residuals(&s, [bumped[44]]);
    assert_eq!(r.count, 1);
    assert!((r.max - 0.3).abs() < 1e-9 && (r.rms - 0.3).abs() < 1e-9, "{r:?}");
}

#[test]
fn the_refusals() {
    assert_eq!(Thinner::new(rule(0.0), &[]).err(), Some(ThinningError::Cell));
    assert_eq!(Thinner::new(rule(f64::NAN), &[]).err(), Some(ThinningError::Cell));
    assert_eq!(Thinner::new(Thinning { z_tolerance: Some(-1.0), ..rule(1.0) }, &[]).err(), Some(ThinningError::ZTolerance));
    assert_eq!(Thinner::new(Thinning { near: Some(0.0), ..rule(1.0) }, &[]).err(), Some(ThinningError::Near));
}
