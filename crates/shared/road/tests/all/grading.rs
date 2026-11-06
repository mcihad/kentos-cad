//! Platform grading (src/grading.rs, docs/adr/0927): the daylight and the volumes on a plane terrain against their
//! closed forms, the fan at a corner, the open lines, the refusals, and the balance.

use kentos_road::grading::{self, BalanceError, GradingError, Platform, Side};
use kentos_road::surface::{self, SpotPoint, Surface, SurfaceInput};

/// A plane terrain z = z0 + gx·x + gy·y over [−100, 100]².
fn terrain(z0: f64, gx: f64, gy: f64) -> Surface {
    let points = [[-100.0, -100.0], [100.0, -100.0], [100.0, 100.0], [-100.0, 100.0]]
        .map(|[x, y]| SpotPoint { x, y, z: z0 + gx * x + gy * y, source: None })
        .to_vec();
    surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("the terrain builds")
}

fn pad(z: f64) -> Platform {
    Platform {
        corners: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
        base: [0.0, 0.0, z],
        grade: 0.0,
        direction: 0.0,
        cut: 1.0,
        fill: 2.0,
        step: 5.0,
        fan: 15.0,
        reach: 50.0,
    }
}

/// The embankment (or cutting) of a 10 × 10 pad `h` above (below) a flat terrain, slopes `s` horizontal per vertical:
/// the box, four side prisms and four corner pyramids on a six-segment fan.
fn closed_form(h: f64, s: f64) -> f64 {
    let r = h * s;
    let fan_area = 6.0 * 0.5 * r * r * (15f64).to_radians().sin();
    100.0 * h + 4.0 * 10.0 * 0.5 * r * h + 4.0 * fan_area * h / 3.0
}

#[test]
fn a_pad_above_a_flat_terrain_is_filled_to_its_closed_form() {
    let g = grading::grade(&pad(12.0), &terrain(10.0, 0.0, 0.0), 0.0).expect("grades");
    assert_eq!(g.open, 0);
    assert!(g.lines.iter().all(|l| l.side == Side::Fill));
    // Every line meets the terrain 4 m out (2 m fill at 2:1).
    for l in &g.lines {
        let d = l.daylight.expect("meets");
        assert!(((d[0] - l.from[0]).hypot(d[1] - l.from[1]) - 4.0).abs() < 1e-9, "{l:?}");
        assert!((d[2] - 10.0).abs() < 1e-9);
    }
    // A corner's fan: 90° in 15° steps is seven lines; the edges between corners one more at 5 m.
    assert_eq!(g.lines.len(), 4 * 7 + 4);
    let v = g.volumes.expect("volumes");
    assert!((v.fill - closed_form(2.0, 2.0)).abs() < 1e-6, "{} {}", v.fill, closed_form(2.0, 2.0));
    assert!(v.cut.abs() < 1e-9);
    assert!((g.top_area - 100.0).abs() < 1e-12);
}

#[test]
fn a_pad_below_a_flat_terrain_is_cut_to_its_closed_form() {
    let g = grading::grade(&pad(8.0), &terrain(10.0, 0.0, 0.0), 0.0).expect("grades");
    assert!(g.lines.iter().all(|l| l.side == Side::Cut));
    let v = g.volumes.expect("volumes");
    assert!((v.cut - closed_form(2.0, 1.0)).abs() < 1e-6, "{} {}", v.cut, closed_form(2.0, 1.0));
    assert!(v.fill.abs() < 1e-9);
}

#[test]
fn the_corners_given_clockwise_grade_the_same() {
    let mut p = pad(12.0);
    p.corners.reverse();
    let a = grading::grade(&p, &terrain(10.0, 0.0, 0.0), 0.0).expect("grades").volumes.expect("volumes");
    assert!((a.fill - closed_form(2.0, 2.0)).abs() < 1e-6);
}

#[test]
fn on_a_slope_the_uphill_side_is_cut_and_the_downhill_side_fill() {
    // The terrain rises 10 % to the east; the pad at 10.5 crosses it.
    let g = grading::grade(&pad(10.5), &terrain(10.0, 0.1, 0.0), 0.0).expect("grades");
    let east = g.lines.iter().find(|l| l.from[0] == 10.0 && l.from[1] == 5.0).expect("the east edge's middle line");
    let west = g.lines.iter().find(|l| l.from[0] == 0.0 && l.from[1] == 5.0).expect("the west edge's middle line");
    assert_eq!(east.side, Side::Cut);
    assert_eq!(west.side, Side::Fill);
    // East: terrain 11 at the edge, the cut rises 1 per 1 and the terrain 0.1: 0.5 = 0.9 u, u = 5/9.
    let d = east.daylight.expect("meets");
    assert!((d[0] - (10.0 + 5.0 / 9.0)).abs() < 1e-9, "{d:?}");
    // West: terrain 10 at the edge, the fill falls 0.5 per 1 and the terrain 0.1: 0.5 = 0.4 u, u = 1.25.
    let w = west.daylight.expect("meets");
    assert!((w[0] + 1.25).abs() < 1e-9, "{w:?}");
    let v = g.volumes.expect("volumes");
    assert!(v.cut > 0.0 && v.fill > 0.0);
}

#[test]
fn a_sloping_pad_follows_its_plane() {
    let mut p = pad(12.0);
    p.grade = 0.02;
    p.direction = 90.0;
    let g = grading::grade(&p, &terrain(10.0, 0.0, 0.0), 0.0).expect("grades");
    let north = g.top.iter().find(|c| c[1] == 10.0).expect("a north corner");
    assert!((north[2] - 12.2).abs() < 1e-12);
}

#[test]
fn a_line_beyond_the_terrain_or_the_reach_is_open_and_there_are_no_volumes() {
    let mut p = pad(40.0);
    p.reach = 20.0;
    let g = grading::grade(&p, &terrain(10.0, 0.0, 0.0), 0.0).expect("grades");
    // 30 m of fill at 2:1 meets 60 m out, past the 20 m reach.
    assert_eq!(g.open, g.lines.len());
    assert!(g.volumes.is_none() && g.why.is_some());
    let mut q = pad(12.0);
    q.corners = vec![[95.0, 0.0], [105.0, 0.0], [105.0, 10.0], [95.0, 10.0]];
    let h = grading::grade(&q, &terrain(10.0, 0.0, 0.0), 0.0).expect("grades");
    assert!(h.lines.iter().any(|l| l.side == Side::Open), "off the terrain");
}

#[test]
fn a_bad_boundary_or_value_is_refused() {
    let t = terrain(10.0, 0.0, 0.0);
    let mut p = pad(12.0);
    p.corners.truncate(2);
    assert_eq!(grading::grade(&p, &t, 0.0).unwrap_err(), GradingError::TooFewCorners);
    let mut p = pad(12.0);
    p.corners = vec![[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]];
    assert!(matches!(grading::grade(&p, &t, 0.0).unwrap_err(), GradingError::Crossing { .. }));
    let mut p = pad(12.0);
    p.corners[1] = p.corners[0];
    assert!(matches!(grading::grade(&p, &t, 0.0).unwrap_err(), GradingError::SamePlace { .. }));
    let mut p = pad(12.0);
    p.fill = 0.0;
    assert!(matches!(grading::grade(&p, &t, 0.0).unwrap_err(), GradingError::BadValue { .. }));
    let mut p = pad(12.0);
    p.fan = 120.0;
    assert!(matches!(grading::grade(&p, &t, 0.0).unwrap_err(), GradingError::BadValue { .. }));
    let mut p = pad(12.0);
    p.base[2] = f64::NAN;
    assert_eq!(grading::grade(&p, &t, 0.0).unwrap_err(), GradingError::NotFinite);
}

#[test]
fn the_balance_meets_cut_and_fill_on_a_slope() {
    let t = terrain(10.0, 0.1, 0.0);
    let p = pad(10.0);
    let b = grading::balance(&p, &t, -3.0, 3.0, 0.01).expect("balances");
    assert!(b.converged && b.net.abs() <= 0.01, "{b:?}");
    let v = grading::grade(&p, &t, b.offset).expect("grades").volumes.expect("volumes");
    assert!((v.cut - v.fill).abs() <= 0.01);
    // On a flat terrain with equal slopes the pad balances at the terrain's height.
    let mut q = pad(10.0);
    q.cut = 2.0;
    let flat = terrain(10.0, 0.0, 0.0);
    let c = grading::balance(&q, &flat, -1.0, 2.0, 1e-6).expect("balances");
    assert!(c.offset.abs() < 1e-6, "{c:?}");
}

#[test]
fn a_bracket_that_keeps_its_sign_is_no_balance() {
    let t = terrain(10.0, 0.0, 0.0);
    // All fill from +1 to +3.
    assert!(matches!(grading::balance(&pad(10.0), &t, 1.0, 3.0, 0.01), Err(BalanceError::NoSignChange { .. })));
    assert_eq!(grading::balance(&pad(10.0), &t, 3.0, 1.0, 0.01), Err(BalanceError::BadBracket));
}

/// A slope line along a terrain triangle's edge, its direction rounded off the edge's line: the section reads the
/// terrain's height at its start, not a crossing lost in rounding.
#[test]
fn a_line_along_a_triangles_edge_reads_the_terrain() {
    let points = [[-200.0, -200.0], [400.0, -200.0], [400.0, 400.0], [-200.0, 400.0]]
        .map(|[x, y]| SpotPoint { x, y, z: 100.0 + 0.05 * x + 0.02 * y, source: None })
        .to_vec();
    let t = surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds");
    let a = 1.25 * std::f64::consts::PI;
    let s = t.section(&[[0.0, 0.0], [100.0 * a.cos(), 100.0 * a.sin()]]);
    assert!((s[0].z.expect("on the terrain") - 100.0).abs() < 1e-9, "{s:?}");
    let mut p = pad(104.0);
    p.base = [0.0, 0.0, 104.0];
    let g = grading::grade(&p, &t, 0.0).expect("grades");
    for l in &g.lines {
        let d = l.daylight.expect("meets");
        let terrain = 100.0 + 0.05 * d[0] + 0.02 * d[1];
        assert!((d[2] - terrain).abs() < 1e-9, "{l:?}");
    }
}
