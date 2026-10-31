//! A paper width on the model screen (docs/adr/0793 §1): a stroke's and a hatch's width, paper mm drawn at the drawing
//! scale, keep their paper width zoomed in past that scale instead of swelling with the world; zoomed out they thin
//! with the world, a device pixel at least; a batch that holds nothing (a sheet, a width on the ground) swells as
//! before.
use std::sync::Arc;

use kentos_native_style::batches::{BatchKind, Cap, FillPaintBatch, StyledBatch, StyledLayer, Unit};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings};

use super::Gpu;

/// Metres of paper per CSS pixel at 96 dpi (`kentos_native_style::program::METRES_PER_PX`).
const MPP: f64 = 0.00026458;

fn batch(id: u64, kind: BatchKind, data: &[f32], cap: f64) -> StyledScene {
    let batch = StyledBatch {
        range: 0..data.len(),
        kind,
        level: 0.,
        key: id,
        bounds: [-1000., -1000., 1000., 1000.],
        origin: [0., 0.],
        reach: 1.,
        reach_unit: Unit::World,
        min_scale: None,
        max_scale: None,
        group: 0,
        effect: None,
        weight_cap: cap,
    };
    StyledScene {
        layers: vec![Arc::new(StyledLayerPart {
            id,
            layer: StyledLayer {
                data: data.to_vec(),
                batches: vec![batch],
                ..Default::default()
            },
        })],
        ..Default::default()
    }
}

/// A horizontal line through the view's middle, `width` world units wide.
fn stroke(id: u64, width: f64, cap: f64) -> StyledScene {
    let kind = BatchKind::Stroke {
        color: [1.; 4],
        width,
        unit: Unit::World,
        dash: None,
        dash_offset: 0.,
        cap: Cap::Butt,
        blur: 0.,
    };
    batch(id, kind, &[-1000., 0., 1000., 0., 0., 0.], cap)
}

/// A hatch of horizontal lines a kilometre apart, one through the view's middle, `width` world units wide.
fn hatch(id: u64, width: f64, cap: f64) -> StyledScene {
    let kind = BatchKind::Fill {
        paint: FillPaintBatch::Hatch {
            color: [1.; 4],
            angle: 0.,
            spacing: 1000.,
            width,
            offset: 0.,
            dash: None,
            dash_offset: 0.,
            stagger: 0.,
            unit: Unit::World,
        },
    };
    let ring = [
        -1000., -1000., 1000., -1000., 1000., 1000., -1000., -1000., 1000., 1000., -1000., 1000.,
    ];
    batch(id, kind, &ring, cap)
}

/// The inked height of the view's middle column, in device pixels (coverage summed).
fn inked(rgba: &[u8], size: usize) -> f64 {
    let x = size / 2;
    (0..size).map(|y| f64::from(rgba[(y * size + x) * 4]) / 255.).sum()
}

pub(super) fn paper_widths_are_held(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let keep = gpu.keep;
    gpu.keep = true;
    let size = 128;
    let mut settings = RenderSettings::new(kentos_render_wgpu::Rgba8::rgb(0, 0, 0));
    // 1 mm at 1:100 is 0.1 m in the world; at the drawing scale 1/(MPP × 100) px a metre it is 3.78 px.
    let scale = 100.;
    let cap = 1. / (MPP * scale);
    let width = 0.001 * scale;
    let paper_px = width * cap;
    for samples in [1, 4] {
        settings.samples = samples;
        for (case, (px_per_m, held, want)) in [
            // At the drawing scale: the paper width.
            (cap, cap, paper_px),
            // Ten and two thousand six hundred times deeper (1:10, 0.1 mm a pixel): held at the paper width.
            (cap * 10., cap, paper_px),
            (10_000., cap, paper_px),
            // Zoomed out to 1:500: thinner with the world.
            (cap / 5., cap, paper_px / 5.),
            // A sheet's map, a width on the ground: swells with the world as before (the view is 128 px).
            (cap * 10., 0., paper_px * 10.),
        ]
        .into_iter()
        .enumerate()
        {
            let camera = Camera {
                width: size as f64,
                height: size as f64,
                scale: px_per_m,
                ..Default::default()
            };
            let n = 90_000 + case as u64 * 10 + u64::from(samples) * 100;
            for (what, scene) in [("stroke", stroke(n, width, held)), ("hatch", hatch(n + 5, width, held))] {
                gpu.styled = Some(scene);
                let pixels = gpu.draw(&[], &camera, &settings, size as u32, size as u32);
                let got = inked(&pixels, size);
                assert!(
                    (got - want.max(1.)).abs() < 0.6,
                    "{what}, {px_per_m} px/m, cap {held}, samples {samples}: inked {got} px, want {want}"
                );
            }
        }
    }
    gpu.styled = previous;
    gpu.keep = keep;
}
