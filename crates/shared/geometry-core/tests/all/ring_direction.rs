//! A small ring's direction at map coordinates (docs/adr/9401 §9.3): which way a ring runs is the sign of its area.
//! Summed from raw Transverse Mercator coordinates the products' rounding is larger than a small ring's area, and the
//! sign can come out wrong: the 12 mm square below runs counter-clockwise (its exact doubled area is +0.000288 m²)
//! and the raw sum gave −0.000488. The core's sum takes coordinates from the first corner.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::{ring_area, signed_area};
use kentos_geometry_core::ops::seam_network::signed2;

/// A 12 × 12 mm square, counter-clockwise, in a 3° Transverse Mercator zone.
const SQUARE: [[f64; 2]; 4] = [
    [486_280.859, 4_420_207.667],
    [486_280.871, 4_420_207.667],
    [486_280.871, 4_420_207.679],
    [486_280.859, 4_420_207.679],
];

#[test]
fn a_small_ring_runs_the_way_it_was_drawn() {
    // The doubled area as exact arithmetic gives it for these four corners.
    let exact = 0.000_287_999_979_861_080_67;
    assert!((ring_area(&SQUARE) * 2.0 - exact).abs() < 1e-12);
    let corners: Vec<Vec2> = SQUARE.iter().map(|p| Vec2::new(p[0], p[1])).collect();
    assert!(signed_area(&corners) > 0.0);
    // The seam network turns its rings by this sign.
    assert!(signed2(&SQUARE) > 0.0, "{}", signed2(&SQUARE));
    let mut clockwise = SQUARE;
    clockwise.reverse();
    assert!(signed2(&clockwise) < 0.0, "{}", signed2(&clockwise));
}
