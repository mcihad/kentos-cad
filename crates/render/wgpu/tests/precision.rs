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
        map: None,
        map_ratio: 1.0,
        metres_per_unit: 1.0,
        center,
        scale,
        width: 1600.0,
        height: 1000.0,
        twist: 0.0,
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

/// How far the GPU may put `p` from its exact place, device pixels: a thousandth of a pixel, and what the numbers
/// carry. The hi/lo pair keeps 2⁻⁴⁸ of `p`'s and of the camera's distance from the origin (a near origin, the
/// document's anchor beside its data, keeps nanometres; an origin 4 400 km away 16 nm, 0.3 px at the deepest zoom),
/// and the pixel itself is a float32 (2⁻²⁴ of its distance from the view's centre).
fn bound(camera: &Camera, p: Vec2, origin: Vec2, scale_factor: f64) -> f64 {
    let far = |q: Vec2| (q.x - origin.x).hypot(q.y - origin.y);
    let from_centre = (p.x - camera.center.x).hypot(p.y - camera.center.y);
    let k = camera.scale * scale_factor;
    1e-3 + ((far(p) + far(camera.center)) * 2f64.powi(-48) + from_centre * 2f64.powi(-23)) * k
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
                    // Float32 keeps 2⁻²⁴ of the distance from the centre: under 0.001 px for 2000 px; deep, the hi/lo
                    // pair's 2⁻⁴⁸ of the distance from the origin.
                    let err = error(gpu, exact);
                    let most = bound(&cam, p, origin, scale_factor);
                    assert!(
                        err < most,
                        "origin {origin:?}, {scale} px/m, ×{scale_factor}: {err} px off (at most {most})"
                    );
                }
            }
            scale *= 3.7;
        }
    }
}

#[test]
fn panning_by_hundredths_of_a_millimetre_does_not_jitter() {
    // The view moves 0.01 mm at a time at 0.1 mm a pixel, the deepest zoom of the 128 m pieces (`PIECES_DEEPEST`).
    pan_without_jitter(PIECES_DEEPEST, 0.000_01, Vec2::new(E + 0.0371, N - 0.0213));
}

#[test]
fn panning_by_a_twentieth_of_a_pixel_does_not_jitter_at_the_deepest_zoom() {
    // 5 nm at a time at 0.1 µm a pixel, a point 430 px from the centre.
    pan_without_jitter(
        MAX_SCALE,
        0.05 / MAX_SCALE,
        Vec2::new(E + 371.0 / MAX_SCALE, N - 213.0 / MAX_SCALE),
    );
}

/// 2000 pans by (`step`, `step` / 2) at `scale`: the GPU keeps `fixed` where the camera's float64 arithmetic puts it,
/// and moves it with each pan as that does (the camera's own float64 steps at a 4.4e6 m northing are 1 nm).
fn pan_without_jitter(scale: f64, step: f64, fixed: Vec2) {
    for origin in origins() {
        let mut last: Option<([f64; 2], [f64; 2])> = None;
        for i in 0..2000 {
            let cam = camera(
                Vec2::new(E + step * f64::from(i), N + step * 0.5 * f64::from(i)),
                scale,
            );
            let gpu = gpu_px(&cam, fixed, origin, 1.0);
            let exact = exact_px(&cam, fixed, 1.0);
            let most = bound(&cam, fixed, origin, 1.0);
            assert!(
                error(gpu, exact) < most,
                "origin {origin:?}, step {i}: {gpu:?} vs {exact:?}"
            );
            if let Some((was, was_exact)) = last {
                // Each step moves the point by exactly the pan, in pixels.
                let moved = [gpu[0] - was[0], gpu[1] - was[1]];
                let truth = [exact[0] - was_exact[0], exact[1] - was_exact[1]];
                assert!(
                    error(moved, truth) < 2.0 * most,
                    "origin {origin:?}, step {i}: moved {moved:?}, not {truth:?}"
                );
                assert!(
                    (truth[0] + step * scale).abs() < 1e-3 + 2e-9 * scale,
                    "step {i}: the camera moved {truth:?}"
                );
            }
            last = Some((gpu, exact));
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
            // One float64 rounding at a 4.4e6 m northing; the pixel bound grows with
            // zoom, while the permitted source-coordinate error stays unchanged.
            let pixel_bound = (f64::EPSILON * N.abs() * scale).max(1e-5);
            assert!(
                (s[0] - x).abs() < pixel_bound && (s[1] - y).abs() < pixel_bound,
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
fn a_two_millimetre_detail_is_inspectable_without_changing_its_source() {
    let a = Vec2::new(E, N);
    let b = Vec2::new(E + 0.002, N);
    let mut cam = camera(a, 1.0);
    cam.zoom_at(1e9, 800., 500.);
    let span = cam.world_to_screen(b)[0] - cam.world_to_screen(a)[0];
    // 20 000 logical pixels at the deepest zoom (0.1 µm a pixel, docs/adr/9220), to the float64 the 2 mm are at an
    // easting of 487 km (its last digit is 0.06 nm, 0.0006 px here).
    assert!(
        (span - 0.002 * MAX_SCALE).abs() < 1e-4 + f64::EPSILON * E * MAX_SCALE,
        "2 mm must span {} logical pixels, got {span}",
        0.002 * MAX_SCALE
    );
    for origin in origins() {
        let err = error(gpu_px(&cam, b, origin, 2.), exact_px(&cam, b, 2.));
        assert!(
            err < bound(&cam, b, origin, 2.),
            "origin {origin:?}: {err} px off"
        );
    }
    // Beside the data, to 0.01 px at 40 000 device pixels from the centre (a float32's 2⁻²⁴ of it is 0.002 px).
    assert!(error(gpu_px(&cam, b, origins()[0], 2.), exact_px(&cam, b, 2.)) < 0.01);
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

/// The deepest zoom the plain scene's 128 m pieces are cut for, and a highlight's outside a deep view. The drawing
/// area goes deeper, to [`MAX_SCALE`], through the deep view's scene and highlight, split at a box round the view
/// (plan/render.md RM-24, docs/adr/9220; `a_deep_view_s_highlight_stays_on_its_exact_place_at_the_deepest_zoom`).
const PIECES_DEEPEST: f64 = 1e4;

#[test]
fn a_long_line_cut_into_pieces_stays_on_its_exact_place_at_the_deepest_zoom() {
    // 20 km survey lines in every direction, passing 0.1 mm (a pixel)
    // beside the view's centre at the pieces' deepest zoom.
    const MAX_SCALE: f64 = PIECES_DEEPEST;
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

#[test]
fn a_deep_view_s_highlight_stays_on_its_exact_place_at_the_deepest_zoom() {
    use kentos_geometry_core::geom::deep::Deep;
    use kentos_render_wgpu::Bounds;
    use kentos_render_wgpu::scene::{Highlight, build_highlight};

    // 20 km survey lines in every direction, passing a pixel (0.1 µm) beside the view's centre at the deepest zoom,
    // highlighted in a deep view: its box the view and three times its size round it, as the drawing area's.
    let cam = camera(Vec2::new(E, N), MAX_SCALE);
    let origin = origins()[0];
    let f = frame(&cam, origin, 1.0);
    let view = cam.visible_bounds();
    let (w, h) = (view.max_x - view.min_x, view.max_y - view.min_y);
    let deep = Deep {
        bounds: Bounds {
            min_x: view.min_x - 3.0 * w,
            min_y: view.min_y - 3.0 * h,
            max_x: view.max_x + 3.0 * w,
            max_y: view.max_y + 3.0 * h,
        },
        tolerance: 0.25 / MAX_SCALE,
    };
    let style = Highlight {
        color: Rgba8::rgb(0x4c, 0x9b, 0xe8),
        fill: None,
        mark_size: 15.0,
        mark_shape: 0,
    };
    let dist = 1.0 / MAX_SCALE;
    let in_box = |p: Vec2| {
        p.x >= deep.bounds.min_x - 1e-12
            && p.x <= deep.bounds.max_x + 1e-12
            && p.y >= deep.bounds.min_y - 1e-12
            && p.y <= deep.bounds.max_y + 1e-12
    };
    let (mut worst_deep, mut worst_plain) = (0.0f64, 0.0f64);
    for k in 0..16 {
        let angle = 0.1 + f64::from(k) * std::f64::consts::PI / 16.0;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let near = Vec2::new(E - dir.y * dist, N + dir.x * dist);
        let (a, b) = (
            Vec2::new(near.x - dir.x * 10_000.0, near.y - dir.y * 10_000.0),
            Vec2::new(near.x + dir.x * 10_000.0, near.y + dir.y * 10_000.0),
        );
        let line: kentos_contracts::Entity = serde_json::from_str(&format!(
            r#"{{"kind":"line","layerId":"0","a":{{"x":{},"y":{}}},"b":{{"x":{},"y":{}}},"attrs":{{}},"id":1}}"#,
            a.x, a.y, b.x, b.y
        ))
        .expect("a line");
        let offset = |with: Option<&Deep>| {
            let part = build_highlight([&line], &style, origin, 0.05, 0, &deep.bounds, with);
            // The piece under the camera: the one in the box, or the 128 m piece through the view.
            let at = |hi: [f32; 2], lo: [f32; 2]| {
                let p = join(hi, lo);
                Vec2::new(origin.x + p[0], origin.y + p[1])
            };
            let piece = part
                .segments
                .iter()
                .find(|s| {
                    let (p, q) = (at(s.a_hi, s.a_lo), at(s.b_hi, s.b_lo));
                    in_box(Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0))
                        || ((p.x - E) * (q.x - E) <= 0.0 && (p.y - N) * (q.y - N) <= 0.0)
                })
                .expect("a piece through the view");
            centre_line_offset(line_corners(
                piece.a_hi, piece.a_lo, piece.b_hi, piece.b_lo, &f,
            ))
        };
        worst_deep = worst_deep.max((offset(Some(&deep)).abs() - 1.0).abs());
        worst_plain = worst_plain.max((offset(None).abs() - 1.0).abs());
    }
    assert!(
        worst_deep < 0.05,
        "split at the deep view's box: up to {worst_deep} px off"
    );
    // Cut at 128 m only, the pieces' ends 64 m away cost hundreds of pixels here: why the deep view splits them.
    assert!(
        worst_plain > 1.0,
        "128 m pieces: up to {worst_plain} px off"
    );
}
