//! A surface's survey quality (src/surface/quality.rs, docs/adr/0916 §4): near pairs, spikes and long triangles found
//! by hand.

use kentos_road::surface::{self, SpotPoint, SurfaceInput};

fn spot(x: f64, y: f64, z: f64) -> SpotPoint {
    SpotPoint { x, y, z, source: None }
}

fn lattice(xs: &[f64], ys: &[f64], z: impl Fn(f64, f64) -> f64) -> Vec<SpotPoint> {
    ys.iter().flat_map(|y| xs.iter().map(move |x| (*x, *y))).map(|(x, y)| spot(x, y, z(x, y))).collect()
}

#[test]
fn a_shot_twice_near_one_place() {
    let mut points = lattice(&[0.0, 1.0, 2.0, 3.0], &[0.0, 1.0, 2.0, 3.0], |_, _| 10.0);
    points.push(spot(1.04, 1.03, 10.4));
    let s = surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds");
    let pairs = s.near_pairs(0.1);
    assert_eq!(pairs.len(), 1);
    assert_eq!((pairs[0].a, pairs[0].b), ([1.0, 1.0, 10.0], [1.04, 1.03, 10.4]));
    assert!((pairs[0].distance - 0.05).abs() < 1e-12 && (pairs[0].dz - 0.4).abs() < 1e-12);
    assert!(s.near_pairs(0.01).is_empty());
}

#[test]
fn a_spike_stands_off_its_neighbours() {
    let s = surface::build(&SurfaceInput {
        points: lattice(&[0.0, 1.0, 2.0, 3.0, 4.0], &[0.0, 1.0, 2.0, 3.0, 4.0], |x, y| if (x, y) == (2.0, 2.0) { 13.0 } else { 10.0 }),
        ..SurfaceInput::default()
    })
    .expect("builds");
    // Within 1 m: the four next to it at 10: off by 3. Each of them sees the spike and three at 10: off by −0.75.
    let spikes = s.spikes(1.0, 1.0);
    assert_eq!(spikes[0].at, [2.0, 2.0, 13.0]);
    assert!((spikes[0].difference - 3.0).abs() < 1e-12 && spikes[0].neighbours == 4);
    assert_eq!(spikes.len(), 1, "{spikes:?}");
    let all = s.spikes(1.0, 0.5);
    assert_eq!(all.len(), 5, "{all:?}");
    assert!(all[1..].iter().all(|p| (p.difference + 0.75).abs() < 1e-12));
    assert!(s.spikes(1.0, 3.0).is_empty());
}

#[test]
fn long_triangles_are_gaps() {
    let s = surface::build(&SurfaceInput {
        points: lattice(&[0.0, 1.0, 9.0, 10.0], &[0.0, 1.0, 2.0], |x, _| x * 0.1),
        ..SurfaceInput::default()
    })
    .expect("builds");
    let g = s.gaps(2.0);
    assert_eq!(g.triangles, 4);
    assert!((g.area - 16.0).abs() < 1e-9 && (g.longest - 8.0_f64.hypot(1.0)).abs() < 1e-12, "{g:?}");
}
