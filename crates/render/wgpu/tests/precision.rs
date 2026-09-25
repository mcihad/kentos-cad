//! Float64 world → float32 GPU at large Turkish TM coordinates (TODOS.md
//! REN-07). The GPU side is measured with the shader's own float32 steps
//! (`precision::world_px`, `line_corners`), the reference is the camera's
//! float64 arithmetic. `tests/gpu.rs` repeats the pan test on a real device.
//!
//! Units: world metres; device pixels from the view's centre, y up.

use kentos_render_wgpu::camera::{MAX_SCALE, MIN_SCALE};
use kentos_render_wgpu::layout::FrameUniform;
use kentos_render_wgpu::precision::{
    MAX_SEGMENT_LENGTH, join, line_corners, split, split_offset, world_px,
};
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8, Vec2};

/// Around a parcel in TUREF / TM36 (EPSG:5256): E ≈ 487 000 m, N ≈ 4 420 000 m.
const E: f64 = 487_012.346;
const N: f64 = 4_420_187.521;

fn camera(center: Vec2, scale: f64) -> Camera {
    Camera {
        center,
        scale,
        width: 1600.0,
        height: 1000.0,
    }
}

fn frame(camera: &Camera, origin: Vec2, scale_factor: f64) -> FrameUniform {
    camera.frame_uniform(
        origin,
        [
            (camera.width * scale_factor) as f32,
            (camera.height * scale_factor) as f32,
        ],
        scale_factor,
        &RenderSettings::new(Rgba8::rgb(0, 0, 0)),
        false,
    )
}

/// The exact position (float64) of `p` in device pixels from the view's centre, y up.
fn exact_px(camera: &Camera, p: Vec2, scale_factor: f64) -> [f64; 2] {
    let k = camera.scale * scale_factor;
    [(p.x - camera.center.x) * k, (p.y - camera.center.y) * k]
}

/// Where the GPU puts `p` of a scene with this origin.
fn gpu_px(camera: &Camera, p: Vec2, origin: Vec2, scale_factor: f64) -> [f64; 2] {
    let (hi, lo) = split_offset(p, origin);
    let px = world_px(hi, lo, &frame(camera, origin, scale_factor));
    [f64::from(px[0]), f64::from(px[1])]
}

fn error(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// A few origins: the document's anchor beside the data, one kilometres away, and none at all.
fn origins() -> [Vec2; 3] {
    [
        Vec2::new(486_512.34, 4_420_187.52),
        Vec2::new(480_000.0, 4_410_000.0),
        Vec2::new(0.0, 0.0),
    ]
}

#[test]
fn the_split_keeps_float64_to_far_below_a_micrometre() {
    for origin in origins() {
        for (dx, dy) in [
            (0.0, 0.0),
            (0.0001, -0.0003),
            (12.3456789, 98.7654321),
            (-4321.123, 555.5),
        ] {
            let p = Vec2::new(E + dx, N + dy);
            let (hi, lo) = split_offset(p, origin);
            let back = join(hi, lo);
            let exact = [p.x - origin.x, p.y - origin.y];
            // Two float32 parts carry about 48 bits: a few nanometres at 4 400 km.
            assert!(
                error(back, exact) < 1e-8,
                "{origin:?} {dx},{dy}: {back:?} vs {exact:?}"
            );
        }
    }
    let (hi, lo) = split(4_420_187.521);
    assert!((f64::from(hi) + f64::from(lo) - 4_420_187.521).abs() < 1e-8);
}

#[test]
fn the_gpu_places_points_where_the_cpu_does_at_every_zoom_and_origin() {
    for origin in origins() {
        let mut scale = MIN_SCALE;
        while scale <= MAX_SCALE {
            let cam = camera(Vec2::new(E, N), scale);
            // Points spread over the view: up to its edges.
            let half_w = cam.width / 2.0 / scale;
            let half_h = cam.height / 2.0 / scale;
            for (fx, fy) in [
                (0.0, 0.0),
                (0.37, -0.21),
                (-0.99, 0.99),
                (0.5, 0.5),
                (1.0, -1.0),
            ] {
                let p = Vec2::new(E + fx * half_w, N + fy * half_h);
                for scale_factor in [1.0, 1.25, 2.0] {
                    let gpu = gpu_px(&cam, p, origin, scale_factor);
                    let exact = exact_px(&cam, p, scale_factor);
                    // Float32 keeps 2⁻²⁴ of the distance from the centre: under 0.001 px for 2000 px.
                    let err = error(gpu, exact);
                    assert!(
                        err < 1e-3,
                        "origin {origin:?}, {scale} px/m, ×{scale_factor}: {err} px off"
                    );
                }
            }
            scale *= 3.7;
        }
    }
}

#[test]
fn panning_by_hundredths_of_a_millimetre_does_not_jitter_at_the_deepest_zoom() {
    // 0.2 mm per pixel; the view moves 0.01 mm (0.05 px) at a time.
    let scale = MAX_SCALE;
    let step = 0.000_01;
    let fixed = Vec2::new(E + 0.0371, N - 0.0213);
    for origin in origins() {
        let mut last: Option<[f64; 2]> = None;
        for i in 0..2000 {
            let cam = camera(
                Vec2::new(E + step * f64::from(i), N + step * 0.5 * f64::from(i)),
                scale,
            );
            let gpu = gpu_px(&cam, fixed, origin, 1.0);
            let exact = exact_px(&cam, fixed, 1.0);
            assert!(
                error(gpu, exact) < 1e-3,
                "origin {origin:?}, step {i}: {gpu:?} vs {exact:?}"
            );
            if let Some(last) = last {
                // Each step moves the point by exactly the pan, in pixels.
                let dx = gpu[0] - last[0];
                let dy = gpu[1] - last[1];
                assert!((dx + step * scale).abs() < 2e-3, "step {i}: moved {dx} px");
                assert!(
                    (dy + 0.5 * step * scale).abs() < 2e-3,
                    "step {i}: moved {dy} px"
                );
            }
            last = Some(gpu);
        }
    }
}

#[test]
fn screen_and_world_round_trip_in_float64() {
    for scale in [MIN_SCALE, 0.01, 1.0, 37.5, 800.0, MAX_SCALE] {
        let cam = camera(Vec2::new(E, N), scale);
        for (x, y) in [
            (0.0, 0.0),
            (800.0, 500.0),
            (1599.5, 0.25),
            (123.456, 987.654),
        ] {
            let w = cam.screen_to_world(x, y);
            let s = cam.world_to_screen(w);
            // Bounded by float64 itself: a northing near 4.4e6 m is kept to 9.3e-10 m,
            // 4.7e-6 px at 5000 px/m.
            assert!(
                (s[0] - x).abs() < 1e-5 && (s[1] - y).abs() < 1e-5,
                "{scale}: {s:?}"
            );
        }
        let p = Vec2::new(E + 1.234_567, N - 7.654_321);
        let s = cam.world_to_screen(p);
        let back = cam.screen_to_world(s[0], s[1]);
        // A picosecond's worth of rounding at these magnitudes (ulp of 4.4e6 is 1e-9 m).
        assert!(
            (back.x - p.x).abs() < 1e-8 && (back.y - p.y).abs() < 1e-8,
            "{scale}: {back:?}"
        );
    }
}

#[test]
fn zooming_keeps_the_world_point_under_the_cursor() {
    let mut cam = camera(Vec2::new(E, N), 1.0);
    let cursor = (1234.5, 321.25);
    let anchor = cam.screen_to_world(cursor.0, cursor.1);
    // A hundred wheel notches in, down to the deepest zoom, and back out.
    for factor in
        std::iter::repeat_n(0.15f64.exp(), 100).chain(std::iter::repeat_n((-0.15f64).exp(), 100))
    {
        cam.zoom_at(factor, cursor.0, cursor.1);
        let under = cam.screen_to_world(cursor.0, cursor.1);
        assert!(
            (under.x - anchor.x).abs() < 1e-7 && (under.y - anchor.y).abs() < 1e-7,
            "{} px/m: {under:?} vs {anchor:?}",
            cam.scale
        );
    }
    assert!(cam.scale <= MAX_SCALE && cam.scale >= MIN_SCALE);
}

#[test]
fn panning_moves_the_world_with_the_pointer() {
    let mut cam = camera(Vec2::new(E, N), 250.0);
    let before = cam.screen_to_world(100.0, 100.0);
    cam.pan_by(40.0, -25.0);
    let after = cam.screen_to_world(140.0, 75.0);
    assert!((after.x - before.x).abs() < 1e-9 && (after.y - before.y).abs() < 1e-9);
}

/// Distance in pixels from the view's centre to the line through a segment's
/// box, as the GPU places the box's corners: its centre line runs between the
/// midpoints of the corner pairs.
fn centre_line_offset(corners: [[f32; 2]; 4]) -> f64 {
    let mid = |a: [f32; 2], b: [f32; 2]| {
        [
            (f64::from(a[0]) + f64::from(b[0])) / 2.0,
            (f64::from(a[1]) + f64::from(b[1])) / 2.0,
        ]
    };
    let p = mid(corners[0], corners[1]);
    let q = mid(corners[2], corners[3]);
    let d = [q[0] - p[0], q[1] - p[1]];
    // Signed distance from (0, 0) to the line p → q.
    (d[0] * -p[1] - d[1] * -p[0]) / d[0].hypot(d[1])
}

#[test]
fn a_long_line_cut_into_pieces_stays_on_its_exact_place_at_the_deepest_zoom() {
    // 20 km survey lines in every direction, passing 0.1 mm (half a pixel)
    // beside the view's centre at the deepest zoom.
    let cam = camera(Vec2::new(E, N), MAX_SCALE);
    let origin = origins()[0];
    let f = frame(&cam, origin, 1.0);
    let dist = 0.0001;
    let expected = dist * MAX_SCALE;
    let (mut worst_cut, mut worst_uncut) = (0.0f64, 0.0f64);
    for k in 0..16 {
        let angle = 0.1 + f64::from(k) * std::f64::consts::PI / 16.0;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let near = Vec2::new(E - dir.y * dist, N + dir.x * dist);
        let at = |t: f64| Vec2::new(near.x + dir.x * t, near.y + dir.y * t);
        let (a, b) = (at(-10_000.0), at(10_000.0));

        // As the scene cuts it: pieces of at most MAX_SEGMENT_LENGTH; the one under the camera.
        let pieces = (20_000.0 / MAX_SEGMENT_LENGTH).ceil();
        let i = (pieces / 2.0).floor();
        let piece = |t: f64| Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
        let (p0, p1) = (piece(i / pieces), piece((i + 1.0) / pieces));
        let (a_hi, a_lo) = split_offset(p0, origin);
        let (b_hi, b_lo) = split_offset(p1, origin);
        let cut = centre_line_offset(line_corners(a_hi, a_lo, b_hi, b_lo, &f));
        worst_cut = worst_cut.max((cut.abs() - expected).abs());

        let (a_hi, a_lo) = split_offset(a, origin);
        let (b_hi, b_lo) = split_offset(b, origin);
        let uncut = centre_line_offset(line_corners(a_hi, a_lo, b_hi, b_lo, &f));
        worst_uncut = worst_uncut.max((uncut.abs() - expected).abs());
    }
    assert!(
        worst_cut < 0.05,
        "cut into pieces: up to {worst_cut} px off"
    );
    // Uncut, the ends 10 km away cost whole pixels: why the scene cuts long segments.
    assert!(worst_uncut > 0.25, "uncut: up to {worst_uncut} px off");
}
