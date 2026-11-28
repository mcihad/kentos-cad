//! A long stretch of strokes and marks drawn one sample a pixel and laid over the multisampled picture (docs/adr/9246):
//! the picture the stretch gives on the picture's own samples. Their shaders smooth their edges and their quads reach a
//! pixel past them, so every sample of a pixel they draw gets the same colour, and laying their premultiplied picture
//! over each sample is the same blend. The stretch is gathered in half floats and rounded once, as it is laid over; the
//! samples' 8-bit picture rounds every blend, so where strokes overlap the two land a step apart.
use std::sync::Arc;

use kentos_native_style::batches::{
    BatchKind, Cap, FillPaintBatch, MarkerLook, StyledBatch, StyledLayer, Unit,
};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8, Vec2};

use super::Gpu;

const SIDE: f64 = 600.0;

fn scatter(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn batch(kind: BatchKind, range: std::ops::Range<usize>, key: u64, reach: f64) -> StyledBatch {
    StyledBatch {
        range,
        kind,
        level: 0.0,
        key,
        bounds: [-10.0, -10.0, SIDE + 10.0, SIDE + 10.0],
        origin: [0.0, 0.0],
        reach,
        reach_unit: Unit::World,
        min_scale: None,
        max_scale: None,
        group: 0,
        effect: None,
        weight_cap: 0.0,
        line_type_loss: None,
        lod: None,
        instances: None,
    }
}

/// Fills under, 150 000 half-transparent strokes (solid and dashed) and 20 000 outlined circles over them, then a fill
/// and a few strokes over those: the long stretch lies between fills.
fn layer() -> StyledLayer {
    let mut seed = 31u64;
    let mut data: Vec<f32> = Vec::new();
    let mut batches = Vec::new();
    let fill = |data: &mut Vec<f32>, seed: &mut u64, n: usize| {
        let start = data.len();
        for _ in 0..n {
            let (x, y) = (scatter(seed) * SIDE, scatter(seed) * SIDE);
            for _ in 0..3 {
                data.extend([
                    (x + scatter(seed) * 80.0) as f32,
                    (y + scatter(seed) * 80.0) as f32,
                ]);
            }
        }
        start..data.len()
    };
    let r = fill(&mut data, &mut seed, 300);
    batches.push(batch(
        BatchKind::Fill {
            paint: FillPaintBatch::Solid {
                color: [0.9, 0.8, 0.3, 0.5],
            },
        },
        r,
        1,
        0.0,
    ));
    let stroke = |dash: Option<Vec<f64>>, color: [f64; 4]| BatchKind::Stroke {
        color,
        width: 0.4,
        unit: Unit::World,
        dash,
        dash_offset: 0.0,
        cap: Cap::Butt,
        blur: 0.0,
    };
    let strokes = |data: &mut Vec<f32>, seed: &mut u64, n: usize| {
        let start = data.len();
        for i in 0..n {
            let (x, y) = (scatter(seed) * SIDE, scatter(seed) * SIDE);
            let (a, l) = (
                scatter(seed) * std::f64::consts::TAU,
                2.0 + scatter(seed) * 12.0,
            );
            data.extend([
                x as f32,
                y as f32,
                (x + a.cos() * l) as f32,
                (y + a.sin() * l) as f32,
                (i % 9) as f32,
                3.0,
            ]);
        }
        start..data.len()
    };
    let r = strokes(&mut data, &mut seed, 90_000);
    batches.push(batch(stroke(None, [0.1, 0.2, 0.6, 0.55]), r, 2, 1.4));
    let r = strokes(&mut data, &mut seed, 60_000);
    batches.push(batch(
        stroke(Some(vec![1.5, 0.8]), [0.6, 0.1, 0.1, 0.7]),
        r,
        3,
        1.4,
    ));
    let start = data.len();
    for _ in 0..20_000 {
        data.extend([
            (scatter(&mut seed) * SIDE) as f32,
            (scatter(&mut seed) * SIDE) as f32,
            0.0,
            3.0,
            3.0,
        ]);
    }
    let marker = BatchKind::Marker {
        unit: Unit::World,
        look: MarkerLook::Shape {
            shape: "circle".into(),
            fill: Some([0.2, 0.7, 0.3, 0.6]),
            stroke: Some([0.05, 0.05, 0.05, 0.9]),
            stroke_width: 0.4,
            params: [0.0; 4],
        },
        offset: [0.0, 0.0],
        anchor: [0.0, 0.0],
        opacity: 1.0,
        extent: [3.0, 3.0],
        vectors: false,
    };
    batches.push(batch(marker, start..data.len(), 4, 6.5));
    let r = fill(&mut data, &mut seed, 40);
    batches.push(batch(
        BatchKind::Fill {
            paint: FillPaintBatch::Solid {
                color: [0.3, 0.6, 0.9, 0.45],
            },
        },
        r,
        5,
        0.0,
    ));
    let r = strokes(&mut data, &mut seed, 3_000);
    batches.push(batch(stroke(None, [0.0, 0.0, 0.0, 0.8]), r, 6, 1.4));
    StyledLayer {
        data,
        batches,
        ..Default::default()
    }
}

pub(super) fn a_long_stretch_drawn_one_sample_a_pixel_is_the_same_picture(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let layer = layer();
    let px = 384u32;
    let mut settings = RenderSettings::new(Rgba8::rgb(255, 255, 255));
    settings.samples = 4;
    for (case, (centre, scale)) in [((300.0, 300.0), 0.62), ((180.0, 420.0), 2.5)]
        .into_iter()
        .enumerate()
    {
        let camera = Camera {
            center: Vec2::new(centre.0, centre.1),
            width: f64::from(px),
            height: f64::from(px),
            scale,
            ..Default::default()
        };
        let mut pictures = Vec::new();
        for (k, single) in [false, true].into_iter().enumerate() {
            settings.single_runs = single;
            let id = 97_000 + case as u64 * 10 + k as u64;
            gpu.styled = Some(StyledScene {
                layers: vec![Arc::new(StyledLayerPart::new(id, layer.clone()))],
                ..Default::default()
            });
            let pixels = gpu.draw(&[], &camera, &settings, px, px);
            let runs = gpu.renderer.stats(1).expect("the view").single_runs;
            assert_eq!(
                runs,
                u32::from(single),
                "case {case}: one-sample stretches drawn with the setting {single}"
            );
            pictures.push(pixels);
        }
        let (a, b) = (&pictures[0], &pictures[1]);
        let most = a
            .iter()
            .zip(b)
            .map(|(p, q)| p.abs_diff(*q))
            .max()
            .unwrap_or(0);
        let differ = a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count();
        let total = a.len() / 4;
        let mut by_step = [0usize; 6];
        for (p, q) in a.chunks(4).zip(b.chunks(4)) {
            let d = p
                .iter()
                .zip(q)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            by_step[usize::from(d.min(5))] += 1;
        }
        eprintln!(
            "single case {case}: {differ} of {total} px differ, most {most}; by step {by_step:?}"
        );
        // Where strokes overlap, the samples' 8-bit picture rounds each blend and the half-float one only once, as
        // it is laid over: mostly a step apart (the samples' own rounding), rarely three. A hard edge in a stroke's
        // cover lets a hair of float noise put a pixel on either side: the butt ends' half step, before docs/adr/9249,
        // set a pixel four and one five steps apart once the quads' corners moved (docs/adr/9247); a quad narrowed on
        // one sample a pixel only set 55 pixels five to 45 steps apart.
        assert!(
            most <= 3,
            "case {case}: {differ} px differ, most {most} steps"
        );
        assert!(
            by_step[2] * 10 < total && by_step[3] * 1000 < total,
            "case {case}: by step {by_step:?}"
        );
    }
    // A view with its own one-sample targets (HiDPI off on a 2× screen) has nothing to save: no stretch is laid over.
    settings.samples = 1;
    settings.hi_dpi = false;
    settings.single_runs = true;
    let camera = Camera {
        center: Vec2::new(300.0, 300.0),
        width: f64::from(px),
        height: f64::from(px),
        scale: 0.62,
        ..Default::default()
    };
    gpu.styled = Some(StyledScene {
        layers: vec![Arc::new(StyledLayerPart::new(97_100, layer.clone()))],
        ..Default::default()
    });
    let _ = gpu.draw_scaled(&[], &camera, &settings, px, px, 2.0);
    assert_eq!(
        gpu.renderer.stats(1).expect("the view").single_runs,
        0,
        "one sample a pixel: no stretch laid over"
    );
    gpu.styled = previous;
}
