//! Görünüme sığdır (docs/adr/0206 §1): the scale a fixed map's frame needs
//! to show the drawing area's view, worked out by hand: a frame 200 × 150 mm
//! shows 0.2 × 0.15 m of paper.

use kentos_sheet::atlas::fit_view_scale;
use kentos_sheet::units::RectUm;

const FRAME: RectUm = RectUm {
    left: 10_000,
    top: 20_000,
    width: 200_000,
    height: 150_000,
};

#[test]
fn the_largest_standard_scale_that_holds_the_view() {
    // 300 × 150 m: 1/1500 across (300 / 0.2), 1/1000 down; 1/2000 is the first standard scale to hold both.
    assert_eq!(fit_view_scale(300.0, 150.0, 0, &FRAME), Some(2000));
    // Exactly 1/1000 across: 1/1000 holds it.
    assert_eq!(fit_view_scale(200.0, 100.0, 0, &FRAME), Some(1000));
    // Turned a quarter: 150 m across (1/750), 300 m down (1/2000).
    assert_eq!(fit_view_scale(300.0, 150.0, 90_000, &FRAME), Some(2000));
    // Turned 45°: (300 + 150)·√½ ≈ 318.2 m each way, 1/2121.3 down: 1/2500.
    assert_eq!(fit_view_scale(300.0, 150.0, 45_000, &FRAME), Some(2500));
    assert_eq!(fit_view_scale(300.0, 150.0, -45_000, &FRAME), Some(2500));
}

#[test]
fn past_the_standard_scales_the_smallest_whole_one() {
    // 50 km × 10 km: 50 000 / 0.2 = 250 000, past 1/100 000.
    assert_eq!(fit_view_scale(50_000.0, 10_000.0, 0, &FRAME), Some(250_000));
    // 50 001 m: 250 005.
    assert_eq!(fit_view_scale(50_001.0, 10_000.0, 0, &FRAME), Some(250_005));
}

#[test]
fn no_ground_no_scale() {
    for (w, h) in [
        (0.0, 10.0),
        (10.0, 0.0),
        (-5.0, 10.0),
        (f64::NAN, 10.0),
        (f64::INFINITY, 10.0),
    ] {
        assert_eq!(fit_view_scale(w, h, 0, &FRAME), None, "{w} × {h}");
    }
}
