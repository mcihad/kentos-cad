//! Dense zero-width hatches retain their screen hairline coverage through zoom.
//! Positive physical widths retain their original paper coverage.
use std::sync::Arc;

use kentos_native_style::batches::{BatchKind, FillPaintBatch, StyledBatch, StyledLayer, Unit};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings};

use super::Gpu;

fn hatch(id: u64, width: f64) -> StyledScene {
    let bounds = [-1000., -1000., 1000., 1000.];
    let data = vec![
        -1000., -1000., 1000., -1000., 1000., 1000., -1000., -1000., 1000., 1000., -1000., 1000.,
    ];
    let batch = StyledBatch {
        range: 0..data.len(),
        kind: BatchKind::Fill {
            paint: FillPaintBatch::Hatch {
                color: [1.; 4],
                angle: 0.,
                spacing: 1.,
                width,
                offset: 0.,
                dash: None,
                dash_offset: 0.,
                stagger: 0.,
                unit: Unit::World,
            },
        },
        level: 0.,
        key: id,
        bounds,
        origin: [0., 0.],
        reach: 0.,
        reach_unit: Unit::Px,
        min_scale: None,
        max_scale: None,
        group: 0,
        effect: None,
    };
    StyledScene {
        layers: vec![Arc::new(StyledLayerPart {
            id,
            layer: StyledLayer {
                data,
                batches: vec![batch],
                ..Default::default()
            },
        })],
        ..Default::default()
    }
}

fn centre_mean(rgba: &[u8]) -> f64 {
    let mut total = 0.;
    for y in 16..48 {
        for x in 16..48 {
            total += f64::from(rgba[(y * 64 + x) * 4]) / 255.;
        }
    }
    total / 1024.
}

pub(super) fn dense_hairlines_do_not_disappear(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let keep = gpu.keep;
    gpu.keep = true;
    let mut settings = RenderSettings::new(kentos_render_wgpu::Rgba8::rgb(0, 0, 0));
    for samples in [1, 4] {
        settings.samples = samples;
        for (case, gap) in [0.25_f64, 0.5, 0.9, 1., 1.5, 2., 2.5]
            .into_iter()
            .enumerate()
        {
            let camera = Camera {
                width: 64.,
                height: 64.,
                scale: gap,
                ..Default::default()
            };
            gpu.styled = Some(hatch(70_000 + case as u64 + u64::from(samples) * 100, 0.));
            let pixels = gpu.draw(&[], &camera, &settings, 64, 64);
            // A screen hairline occupies one device pixel per period; overlapping periods saturate.
            let expected = (1. / gap).min(1.);
            assert!(
                (centre_mean(&pixels) - expected).abs() < 0.015,
                "gap={gap}, samples={samples}: screen hairline disappeared"
            );
        }
        let camera = Camera {
            width: 64.,
            height: 64.,
            scale: 0.5,
            ..Default::default()
        };
        gpu.styled = Some(hatch(71_000 + u64::from(samples), 0.02));
        let pixels = gpu.draw(&[], &camera, &settings, 64, 64);
        assert!(
            (centre_mean(&pixels) - 0.02).abs() < 0.006,
            "a physical-width hatch retains its 2% paper coverage"
        );
    }
    gpu.styled = previous;
    gpu.keep = keep;
}
