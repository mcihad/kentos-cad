//! Surface drainage (src/drainage.rs, docs/adr/0929): a water drop down a plane and along a valley, a pond's level,
//! depth, area and volume in closed form, a catchment's area and flow path, and Manning's and the Rational method's
//! numbers by hand.

use kentos_road::drainage::{self, Channel, DropEnd, Tin};
use kentos_road::surface::{self, SpotPoint, Surface, SurfaceInput};

fn surface_of(f: impl Fn(f64, f64) -> f64, from: f64, to: f64, step: f64) -> Surface {
    let mut points = Vec::new();
    let n = ((to - from) / step).round() as i32;
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (from + f64::from(i) * step, from + f64::from(j) * step);
            points.push(SpotPoint { x, y, z: f(x, y), source: None });
        }
    }
    surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds")
}

#[test]
fn a_drop_runs_down_a_plane_to_its_edge() {
    let s = surface_of(|x, y| 100.0 + 0.1 * x + 0.02 * y, 0.0, 10.0, 1.0);
    let tin = Tin::new(&s);
    let d = drainage::water_drop(&tin, [5.3, 5.1]);
    assert_eq!(d.end, DropEnd::Edge, "{d:?}");
    // Down the gradient (−0.1, −0.02): every step falls and keeps that heading.
    for w in d.path.windows(2) {
        assert!(w[1][2] < w[0][2] + 1e-12, "{w:?}");
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        assert!((dx * 0.02 - dy * 0.1).abs() < 1e-9 * dx.hypot(dy).max(1.0), "{w:?}");
    }
    let last = d.path.last().expect("a path");
    assert!(last[0].abs() < 1e-9, "{last:?}");
    assert!((d.length - 5.3_f64.hypot(5.3 * 0.2)).abs() < 1e-9, "{}", d.length);
}

#[test]
fn a_drop_follows_a_valley_to_its_low_end() {
    // A V valley along y = 5, falling to the west: z = 100 + 0.05 x + 0.2 |y − 5|.
    let s = surface_of(|x, y| 100.0 + 0.05 * x + 0.2 * (y - 5.0).abs(), 0.0, 10.0, 1.0);
    let tin = Tin::new(&s);
    let d = drainage::water_drop(&tin, [7.3, 8.2]);
    let last = *d.path.last().expect("a path");
    assert!((last[1] - 5.0).abs() < 1e-9 && last[0].abs() < 1e-9, "{d:?}");
    assert_eq!(d.end, DropEnd::Edge);
    assert!(d.path.iter().any(|p| (p[1] - 5.0).abs() < 1e-9 && p[0] > 6.0), "it reaches the valley early: {d:?}");
}

#[test]
fn a_bowl_is_a_pond_up_to_its_rim() {
    // A cone bowl z = |r| on a lattice, its rim the lowest edge corner: (±5, 0) and (0, ±5) at 5.
    let s = surface_of(|x, y| x.hypot(y), -5.0, 5.0, 1.0);
    let tin = Tin::new(&s);
    let ponds = drainage::ponds(&tin, 0.0);
    assert_eq!(ponds.len(), 1, "{ponds:?}");
    let p = &ponds[0];
    assert_eq!(p.bottom, [0.0, 0.0, 0.0]);
    assert!((p.level - 5.0).abs() < 1e-12 && (p.depth - 5.0).abs() < 1e-12, "{p:?}");
    assert!(p.area > 0.0 && p.volume > 0.0 && p.volume < p.area * p.depth);
    assert!(drainage::ponds(&tin, 5.5).is_empty());
    // A plane holds no water.
    let flat = surface_of(|x, y| 100.0 + 0.1 * x + 0.02 * y, 0.0, 10.0, 1.0);
    assert!(drainage::ponds(&Tin::new(&flat), 0.0).is_empty());
}

#[test]
fn a_pond_volume_in_closed_form() {
    // A square pit: the centre corner 1 m below a level square of 3 × 3 corners at 2 m spacing, the rim one higher at
    // its middles: the water fills the four triangles around the centre to the level 0 (a square pyramid of base
    // 4 × 4 m and height 1 m: volume 16/3, area 16 m²... the triangulation's diagonals cut the base into eight triangles).
    let mut points = Vec::new();
    for j in -2i32..=2 {
        for i in -2i32..=2 {
            let (x, y) = (f64::from(i) * 2.0, f64::from(j) * 2.0);
            let z = if i == 0 && j == 0 { -1.0 } else if i.abs() == 2 || j.abs() == 2 { 1.0 } else { 0.0 };
            points.push(SpotPoint { x, y, z, source: None });
        }
    }
    let s = surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds");
    let tin = Tin::new(&s);
    let ponds = drainage::ponds(&tin, 0.0);
    assert_eq!(ponds.len(), 1, "{ponds:?}");
    let p = &ponds[0];
    // The water stands up to the rim (1 m): the inner corners at 0 are under it too.
    assert!((p.level - 1.0).abs() < 1e-12 && (p.depth - 2.0).abs() < 1e-12, "{p:?}");
    // Up to the rim the water covers the 8 × 8 m square: its volume is the box 64 × 1 above the inner ring's level
    // (the 4 × 4 inner square's pyramid and the ring falling from 1 to 0) — each triangle's area times its centroid depth.
    let mut want = 0.0;
    for t in &s.triangles {
        let v = t.map(|i| s.vertices[i as usize]);
        let area = ((v[1][0] - v[0][0]) * (v[2][1] - v[0][1]) - (v[2][0] - v[0][0]) * (v[1][1] - v[0][1])).abs() / 2.0;
        let mean = (3.0 - v[0][2] - v[1][2] - v[2][2]) / 3.0;
        want += area * mean;
    }
    // Its area is the triangles with a wet corner: the square less its four corner triangles on the rim (2 m² each).
    assert!((p.area - 56.0).abs() < 1e-9, "{p:?}");
    assert!((p.volume - want).abs() < 1e-9, "{} {want}", p.volume);
}

#[test]
fn a_catchment_of_a_valley_outlet() {
    let s = surface_of(|x, y| 100.0 + 0.05 * x + 0.2 * (y - 5.0).abs(), 0.0, 10.0, 1.0);
    let tin = Tin::new(&s);
    let c = drainage::catchment(&tin, [0.0, 5.0], 0.5, 0.3).expect("a catchment");
    // The cells whose drop reaches the valley, or the west edge within 0.3 m of the outlet: 4 x₀ ≥ |y₀ − 5| − 0.3.
    let want = (0..20)
        .flat_map(|i| (0..20).map(move |j| (0.25 + 0.5 * f64::from(i), 0.25 + 0.5 * f64::from(j))))
        .filter(|(x, y)| 4.0 * x >= (y - 5.0).abs() - 0.3)
        .count();
    assert_eq!(c.cells, want);
    assert!((c.area - want as f64 * 0.25).abs() < 1e-9);
    assert_eq!(c.outline.len(), 1);
    let first = c.path[0];
    assert!(first[0] > 9.0 && (first[1] - 5.0).abs() > 4.0, "the most distant cell is a far corner: {first:?}");
    assert!(c.fall > 0.0 && c.length > 10.0);
    assert!(drainage::catchment(&tin, [5.0, 9.5], 0.5, 0.01).is_none_or(|c| c.cells < 20));
}

#[test]
fn manning_and_the_rational_method_by_hand() {
    // A 1 m pipe at 1 % with n = 0.013: A = π/4, R = 0.25.
    let f = drainage::manning(Channel::Pipe { diameter: 1.0 }, 0.01, 0.013).expect("flows");
    let v = 0.25_f64.powf(2.0 / 3.0) * 0.1 / 0.013;
    assert!((f.velocity - v).abs() < 1e-12 && (f.discharge - v * std::f64::consts::FRAC_PI_4).abs() < 1e-12);
    let b = drainage::manning(Channel::Box { width: 2.0, height: 1.0 }, 0.004, 0.015).expect("flows");
    assert!((b.radius - 2.0 / 6.0).abs() < 1e-15);
    let t = drainage::manning(Channel::Trapezoid { bottom: 0.5, depth: 0.4, left: 1.5, right: 2.0 }, 0.02, 0.03).expect("flows");
    assert!((t.area - (0.2 + 0.5 * 3.5 * 0.16)).abs() < 1e-15);
    assert!((t.perimeter - (0.5 + 0.4 * (3.25_f64.sqrt() + 5.0_f64.sqrt()))).abs() < 1e-12);
    assert!(drainage::manning(Channel::Pipe { diameter: 1.0 }, 0.0, 0.013).is_none());
    // 2 ha, C = 0.7, 80 mm/h: Q = 0.7 · 80 · 0.02 / 3.6.
    assert!((drainage::rational(0.7, 80.0, 20_000.0) - 0.7 * 80.0 * 0.02 / 3.6).abs() < 1e-15);
}

#[test]
fn every_cell_of_a_valley_drains_as_its_closed_form_says() {
    // z = 100 + 0.05 x + 0.2 |y − 5|: a drop heads (−0.05, ∓0.2), so from (x₀, y₀) it reaches the valley y = 5 before
    // the west edge when 4 x₀ ≥ |y₀ − 5|, and then runs along it to (0, 5); else it ends at the edge, |y₀ − 5| − 4 x₀
    // from (0, 5). Starts on a triangle's edge are among them.
    let s = surface_of(|x, y| 100.0 + 0.05 * x + 0.2 * (y - 5.0).abs(), 0.0, 10.0, 1.0);
    let tin = Tin::new(&s);
    for i in 0..20 {
        for j in 0..20 {
            let (x, y) = (0.25 + 0.5 * f64::from(i), 0.25 + 0.5 * f64::from(j));
            let d = drainage::water_drop(&tin, [x, y]);
            let last = *d.path.last().expect("a path");
            assert_eq!(d.end, DropEnd::Edge, "{x} {y}: {d:?}");
            assert!(last[0].abs() < 1e-9, "{x} {y}: {last:?}");
            let want = if 4.0 * x >= (y - 5.0).abs() { 5.0 } else { y - 4.0 * x * (y - 5.0).signum() };
            assert!((last[1] - want).abs() < 1e-9, "{x} {y}: {last:?} not {want}");
        }
    }
}
