//! A surface's analysis (src/surface/analysis.rs, docs/adr/0916 §1): on planes and on a lattice of planar squares
//! (z = g(x) + h(y): every square's four corners on one plane, so the answers do not depend on the diagonals), the
//! ranges' areas, the roughness and the raster's slope against the triangles' in closed form.

use kentos_road::surface::{self, AnalysisError, Measure, SpotPoint, Surface, SurfaceInput};

fn surface_of(f: impl Fn(f64, f64) -> f64, n: i32, step: f64) -> Surface {
    let mut points = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (f64::from(i) * step, f64::from(j) * step);
            points.push(SpotPoint { x, y, z: f(x, y), source: None });
        }
    }
    surface::build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds")
}

#[test]
fn a_plane_s_elevation_ranges_are_its_strips() {
    // z = 100 + 0.1 x over 10 × 10: z runs 100 to 101 across x; the range [100.2, 100.5) is the strip 2 ≤ x < 5.
    let s = surface_of(|x, _| 100.0 + 0.1 * x, 10, 1.0);
    let (b, parts) = s.bands(Measure::Elevation, &[99.0, 100.2, 100.5, 100.9], true).expect("bands");
    let k = 1.01_f64.sqrt();
    let want = [20.0, 30.0, 40.0];
    for (band, w) in b.bands.iter().zip(want) {
        assert!((band.area - w).abs() < 1e-9, "{band:?}");
        assert!((band.surface_area - w * k).abs() < 1e-9, "{band:?}");
    }
    assert!((b.outside_area - 10.0).abs() < 1e-9, "{b:?}");
    assert_eq!(parts.len(), 3);
    let (plan, surf) = s.areas();
    assert!((plan - 100.0).abs() < 1e-9 && (surf - 100.0 * k).abs() < 1e-9);
}

#[test]
fn slopes_and_aspects_by_square() {
    // z = g(x) + h(y): g rises 0.2 per metre for x < 5 and is flat after; h falls 0.05 for y < 5, flat after.
    let g = |x: f64| if x <= 5.0 { 0.2 * x } else { 1.0 };
    let h = |y: f64| if y <= 5.0 { -0.05 * y } else { -0.25 };
    let s = surface_of(|x, y| 50.0 + g(x) + h(y), 10, 1.0);
    // Slopes: √(0.2² + 0.05²) = 20.6 %, 20 %, 5 %, 0 (flat).
    let (b, _) = s.bands(Measure::Slope, &[1.0, 10.0, 20.3, 30.0], false).expect("bands");
    assert!((b.bands[0].area - 25.0).abs() < 1e-9 && b.bands[0].triangles == 50, "{b:?}");
    assert!((b.bands[1].area - 25.0).abs() < 1e-9, "{b:?}");
    assert!((b.bands[2].area - 25.0).abs() < 1e-9, "{b:?}");
    assert!((b.outside_area - 25.0).abs() < 1e-9 && b.flat_area == 0.0, "{b:?}");
    // Aspects: falling west (270°) where only g slopes, north (0°) where only h, between where both; flat apart.
    let (a, _) = s.bands(Measure::Aspect, &[0.0, 45.0, 315.0, 360.0], false).expect("bands");
    assert!((a.flat_area - 25.0).abs() < 1e-9, "{a:?}");
    let west_south = (-0.2_f64).atan2(0.05).to_degrees() + 360.0;
    assert!(west_south > 270.0 && west_south < 315.0);
    assert!((a.bands[0].area - 25.0).abs() < 1e-9, "{a:?}");
    assert!((a.bands[1].area - 50.0).abs() < 1e-9, "{a:?}");
}

#[test]
fn a_plane_is_smooth_and_a_ridge_is_rough() {
    let s = surface_of(|x, y| 10.0 + 0.3 * x - 0.1 * y, 6, 1.0);
    let r = s.roughness().expect("corners");
    assert!(r.max < 1e-12 && r.mean < 1e-12, "{r:?}");
    // A ridge along x = 3: the corners on it have two planes ±45° round them.
    let ridge = surface_of(|x, _| 10.0 - (x - 3.0).abs(), 6, 1.0);
    let r = ridge.roughness().expect("corners");
    // Half the angle each way: |Σ| / Σα = cos 45°.
    assert!((r.max - (1.0 - std::f64::consts::FRAC_1_SQRT_2)).abs() < 1e-12, "{r:?}");
    assert_eq!(r.at, [3.0, 0.0]);
}

#[test]
fn a_raster_flattens_a_ridge_and_keeps_a_plane() {
    let s = surface_of(|x, y| 10.0 + 0.3 * x - 0.1 * y, 30, 1.0);
    let g = s.grid_slope(1.5).expect("a grid");
    assert!(g.cells > 0 && g.max_absolute < 1e-9, "{g:?}");
    let ridge = surface_of(|x, _| 10.0 - 0.5 * (x - 15.0).abs(), 30, 1.0);
    let g = ridge.grid_slope(1.5).expect("a grid");
    // A centre within a cell of the ridge sees both sides: Horn's x difference over the ridge is smaller.
    assert!(g.max_absolute > 1.0 && g.mean < 0.0, "{g:?}");
}

#[test]
fn the_refusals() {
    let s = surface_of(|x, _| x, 2, 1.0);
    assert_eq!(s.bands(Measure::Slope, &[1.0], false).err(), Some(AnalysisError::Breaks));
    assert_eq!(s.bands(Measure::Slope, &[2.0, 1.0], false).err(), Some(AnalysisError::Breaks));
    assert_eq!(s.grid_slope(0.0).err(), Some(AnalysisError::Cell));
    assert_eq!(s.grid_slope(1e-6).err(), Some(AnalysisError::TooMany));
}
