//! A seen-through line's corners (docs/adr/9226): a polyline's segments meet at their joins once. Each segment draws
//! its own side of a join's bisector, so a line half seen through is as dark at its corners as along it, not twice as
//! dark where the two segments' round ends overlapped. Opaque, the same pixels are fully inked as before: no join
//! leaves a pixel out.
use std::sync::Arc;

use kentos_native_style::batches::{BatchKind, Cap, StyledBatch, StyledLayer, Unit};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings};

use super::Gpu;

/// A join's code as the style core writes it: 1024 steps a half turn, 1024 straight on.
fn turn_code(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let (d0, d1) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
    let turn = (d0[0] * d1[1] - d0[1] * d1[0]).atan2(d0[0] * d1[0] + d0[1] * d1[1]);
    (turn / std::f64::consts::PI * 1024.0).round() + 1024.0
}

/// A zigzag's segments, with their joins' codes or without (as a renderer before them wrote them).
fn zigzag(pts: &[[f64; 2]], joins: bool) -> Vec<f32> {
    let n = pts.len() - 1;
    let mut data = Vec::new();
    let mut along = 0.0;
    for i in 0..n {
        let (a, b) = (pts[i], pts[i + 1]);
        let ends = if i == 0 { 1.0 } else { 0.0 } + if i + 1 == n { 2.0 } else { 0.0 };
        let start = if joins && i > 0 {
            turn_code(pts[i - 1], a, b)
        } else {
            0.0
        };
        let end = if joins && i + 1 < n {
            turn_code(a, b, pts[i + 2])
        } else {
            0.0
        };
        data.extend(
            [
                a[0],
                a[1],
                b[0],
                b[1],
                along,
                ends + 4.0 * start + 8192.0 * end,
            ]
            .map(|v| v as f32),
        );
        along += (b[0] - a[0]).hypot(b[1] - a[1]);
    }
    data
}

fn scene(id: u64, data: Vec<f32>, alpha: f64) -> StyledScene {
    let batch = StyledBatch {
        range: 0..data.len(),
        kind: BatchKind::Stroke {
            color: [1., 0., 0., alpha],
            width: 12.,
            unit: Unit::World,
            dash: None,
            dash_offset: 0.,
            cap: Cap::Round,
            blur: 0.,
        },
        level: 0.,
        key: id,
        bounds: [-1000., -1000., 1000., 1000.],
        origin: [0., 0.],
        reach: 7.,
        reach_unit: Unit::World,
        min_scale: None,
        max_scale: None,
        group: 0,
        effect: None,
        weight_cap: 0.,
        line_type_loss: None,
        lod: None,
        instances: None,
    };
    StyledScene {
        layers: vec![Arc::new(StyledLayerPart::new(
            id,
            StyledLayer {
                low: Vec::new(),
                data,
                batches: vec![batch],
                ..Default::default()
            },
        ))],
        ..Default::default()
    }
}

/// The union of the zigzag's capsules at a pixel's middle, as the shader covers it: its half width less the distance to
/// the nearest segment, a pixel's ramp.
fn union_cover(pts: &[[f64; 2]], half: f64, p: [f64; 2]) -> f64 {
    let d = pts
        .windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let t =
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
        })
        .fold(f64::INFINITY, f64::min);
    (half - d + 0.5).clamp(0.0, 1.0)
}

pub(super) fn seen_through_corners_are_drawn_once(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let keep = gpu.keep;
    gpu.keep = true;
    let size = 160u32;
    let settings = RenderSettings::new(kentos_render_wgpu::Rgba8::rgb(0, 0, 0));
    // A pixel a world unit, the zigzag's corners turning 126.9° and back, a sharper one (138.5°) at the end; 12 wide.
    let camera = Camera {
        width: f64::from(size),
        height: f64::from(size),
        scale: 1.,
        ..Default::default()
    };
    let pts = [
        [-66., -30.],
        [-36., 30.],
        [-6., -30.],
        [24., 30.],
        [54., -30.],
        [70., 30.],
    ];
    let red = |pixels: &[u8]| -> Vec<u8> { pixels.chunks(4).map(|p| p[0]).collect() };
    // Each pixel's ink as the union of the segments' capsules gives it: the line drawn once everywhere.
    let half = size as f64 / 2.0;
    let want: Vec<f64> = (0..size * size)
        .map(|i| {
            let (x, y) = (
                f64::from(i % size) + 0.5 - half,
                half - (f64::from(i / size) + 0.5),
            );
            union_cover(&pts, 6.0, [x, y])
        })
        .collect();
    // The most a pixel is off the union, in levels of 255, for a red seen through by `alpha`.
    let mut off = |joins: bool, alpha: f64, n: u64| -> (f64, usize) {
        gpu.styled = Some(scene(n, zigzag(&pts, joins), alpha));
        let r = red(&gpu.draw(&[], &camera, &settings, size, size));
        let mut most = (0.0f64, 0usize);
        for (i, (&got, w)) in r.iter().zip(&want).enumerate() {
            let d = (f64::from(got) - 255.0 * alpha * w).abs();
            if d > most.0 {
                most = (d, i);
            }
        }
        most
    };
    for alpha in [0.5, 1.0] {
        let (most, at) = off(true, alpha, 91_000 + (alpha * 10.0) as u64);
        assert!(
            most <= 3.0,
            "seen through by {alpha}: a pixel {most} off the union at ({}, {})",
            at as u32 % size,
            at as u32 / size
        );
    }
    // Without the codes, as a renderer before them drew it: the round ends overlap at the corners, twice the ink.
    let (most, _) = off(false, 0.5, 91_020);
    assert!(most > 40.0, "the old way overlaps at the corners: {most}");

    gpu.styled = previous;
    gpu.keep = keep;
}
