//! The styled drawing far from the drawing's anchor (docs/adr/0157): the
//! shader's float32 steps (`shaders/wgsl/styled/common.wgsl` `camera` and
//! `toPx`) walked on the CPU, against the camera's float64 arithmetic.
//! Positions are packed from their tile as the style core packs them
//! (`style::batch::TILE`, the tile of the primitive's first point), the
//! camera goes as a float32 high and low part, each batch carries its tile's
//! origin. Before tiles every position and the camera were one float32 from
//! the anchor.
//!
//! Units: world metres; device pixels from the view's centre, y up.

/// The tiles' side (the style core's `TILE`).
const TILE: f64 = 65536.0;

/// A project's anchor in TUREF / TM36 (`fixtures/interaction/v1/vector-fit.kcad`).
const ANCHOR: [f64; 2] = [487_100.0, 4_420_200.0];

/// A local survey's corner in that project: 4 400 km from the anchor.
const LOCAL: [f64; 2] = [1062.5, 2003.25];

/// `Math.round`: the nearest whole, halves up (the core's `js_round`).
fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// The tile's origin from the anchor and the position from the tile, as
/// the style core packs a primitive whose first point is `p`.
fn pack(p: [f64; 2]) -> ([f32; 2], [f32; 2]) {
    let tile = [
        js_round((p[0] - ANCHOR[0]) / TILE),
        js_round((p[1] - ANCHOR[1]) / TILE),
    ];
    let origin = [tile[0] * TILE, tile[1] * TILE];
    let base = [ANCHOR[0] + origin[0], ANCHOR[1] + origin[1]];
    (
        [origin[0] as f32, origin[1] as f32],
        [(p[0] - base[0]) as f32, (p[1] - base[1]) as f32],
    )
}

/// The frame's camera centre from the anchor in two float32 parts.
fn camera_parts(center: [f64; 2]) -> ([f32; 2], [f32; 2]) {
    let c = [center[0] - ANCHOR[0], center[1] - ANCHOR[1]];
    let hi = [c[0] as f32, c[1] as f32];
    let lo = [
        (c[0] - f64::from(hi[0])) as f32,
        (c[1] - f64::from(hi[1])) as f32,
    ];
    (hi, lo)
}

/// The shader, step for step in float32: the position less the camera's
/// high part from the batch's tile, less its low part, in device pixels.
fn styled_px(p: [f64; 2], center: [f64; 2], px_per_m: f32) -> [f64; 2] {
    let (origin, local) = pack(p);
    let (hi, lo) = camera_parts(center);
    let near = [hi[0] - origin[0], hi[1] - origin[1]];
    [
        f64::from(((local[0] - near[0]) - lo[0]) * px_per_m),
        f64::from(((local[1] - near[1]) - lo[1]) * px_per_m),
    ]
}

/// What the styled drawing did before tiles: the position and the camera
/// each one float32 from the anchor.
fn single_px(p: [f64; 2], center: [f64; 2], px_per_m: f32) -> [f64; 2] {
    let local = [(p[0] - ANCHOR[0]) as f32, (p[1] - ANCHOR[1]) as f32];
    let cam = [
        (center[0] - ANCHOR[0]) as f32,
        (center[1] - ANCHOR[1]) as f32,
    ];
    [
        f64::from((local[0] - cam[0]) * px_per_m),
        f64::from((local[1] - cam[1]) * px_per_m),
    ]
}

fn exact_px(p: [f64; 2], center: [f64; 2], px_per_m: f64) -> [f64; 2] {
    [(p[0] - center[0]) * px_per_m, (p[1] - center[1]) * px_per_m]
}

fn error(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The bound docs/adr/0157 §1 gives: a position is its float32 from its
/// tile, at most 46 km from the tile's centre: 2 mm.
const BOUND_M: f64 = 0.002;

/// Zooms from an overview to the deepest (5000 device px a metre), and
/// cameras a little off the point so that it stays on a 1600 × 1000 screen.
const ZOOMS: [f64; 6] = [0.25, 4.0, 76.0, 500.0, 2000.0, 5000.0];

#[test]
fn a_corner_4400_km_from_the_anchor_is_off_by_no_more_than_its_float32_from_the_tile() {
    let mut worst: f64 = 0.0;
    for k in ZOOMS {
        for (dx, dy) in [(0.0, 0.0), (0.0371, -0.0213), (-0.1234, 0.0789)] {
            // Within ±600 px of the centre at this zoom.
            let reach = 600.0 / k;
            let center = [LOCAL[0] + dx * reach, LOCAL[1] + dy * reach];
            let got = styled_px(LOCAL, center, k as f32);
            let want = exact_px(LOCAL, center, k);
            // In metres: the same at every zoom.
            worst = worst.max(error(got, want) / k);
        }
    }
    // The corner lies 27 km from its tile's centre (the tiles' grid is the
    // anchor's): its float32 is good to 1 mm.
    assert!(worst < BOUND_M, "{worst} m");
}

#[test]
fn a_far_circle_is_round_at_a_close_zoom() {
    // A 4 m circle in the local survey at 100 px/m (the whole circle on
    // screen): every vertex within 0.2 px; before tiles they sat on a half
    // metre grid, 25 px apart.
    let c = [1046.0, 2028.0];
    let k = 100.0;
    let mut before: f64 = 0.0;
    for i in 0..64 {
        let a = f64::from(i) * std::f64::consts::TAU / 64.0;
        let p = [c[0] + 4.0 * a.cos(), c[1] + 4.0 * a.sin()];
        // A circle's vertices are packed from the tile of its first point.
        assert_eq!(pack(c).0, pack(p).0, "one tile");
        let e = error(styled_px(p, c, k as f32), exact_px(p, c, k));
        assert!(e < BOUND_M * k, "vertex {i}: {e} px");
        before = before.max(error(single_px(p, c, k as f32), exact_px(p, c, k)));
    }
    assert!(before > 10.0, "{before} px before tiles");
}

#[test]
fn panning_by_hundredths_of_a_millimetre_far_from_the_anchor_moves_smoothly() {
    // The camera's two parts follow every step: at the deepest zoom 0.01 mm
    // is 0.05 px a step, never a jump (before tiles the view moved in half
    // metre leaps).
    let k = 5000.0;
    let mut last: Option<[f64; 2]> = None;
    for step in 0..200 {
        let center = [
            LOCAL[0] + f64::from(step) * 1e-5,
            LOCAL[1] - f64::from(step) * 1e-5,
        ];
        let got = styled_px(LOCAL, center, k as f32);
        assert!(
            error(got, exact_px(LOCAL, center, k)) / k < BOUND_M,
            "step {step}"
        );
        if let Some(prev) = last {
            let moved = error(got, prev);
            assert!(
                (moved - 0.0707).abs() < 0.01,
                "step {step}: moved {moved} px"
            );
        }
        last = Some(got);
    }
}

#[test]
fn around_the_anchor_positions_pack_as_before() {
    // Within ±32 km of the anchor the tile is the anchor's own: the numbers
    // are the very float32 the single origin gave.
    for p in [
        [487_012.346, 4_420_187.521],
        [487_100.0 + 32_000.0, 4_420_200.0 - 31_000.0],
        [487_100.0 - 32_767.0, 4_420_200.0 + 32_767.0],
    ] {
        let (origin, local) = pack(p);
        assert_eq!(origin, [0.0, 0.0]);
        assert_eq!(
            local,
            [(p[0] - ANCHOR[0]) as f32, (p[1] - ANCHOR[1]) as f32]
        );
        let center = [p[0] + 0.05, p[1] - 0.03];
        let e = error(styled_px(p, center, 76.0), exact_px(p, center, 76.0)) / 76.0;
        assert!(e < BOUND_M, "{e} m");
    }
}
