//! Independent signed-distance samples, not an expected image from the renderer.
use super::Gpu;
use kentos_native_style::batches::{
    BatchKind, Cap, FillPaintBatch, StyledBatch, StyledLayer, Unit,
};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8};
use std::sync::Arc;

/// Expected interior ink and distance from the nearest boundary, in raw units.
fn expected(values: &[f64], at: f64) -> (bool, f64) {
    let period: f64 = values.iter().sum::<f64>() * if values.len() % 2 == 1 { 2. } else { 1. };
    let mut t = at.rem_euclid(period);
    for (i, length) in values.iter().cycle().take(values.len() * 2).enumerate() {
        if t < *length {
            return (i % 2 == 0, t.min(length - t));
        }
        t -= length;
    }
    panic!("point belongs to one interval");
}

pub(super) fn full_patterns_match_independent_samples(gpu: &mut Gpu) {
    let old = gpu.styled.take();
    let keep = gpu.keep;
    gpu.keep = false;
    let (w, h) = (512usize, 96usize);
    // Deliberately differ after the eighth slot; the odd case needs all 18.
    let recipes = [
        vec![2., 3., 4., 3., 5., 2., 6., 4., 17., 7., 19., 9.],
        vec![3., 2., 5., 2., 7., 3., 9., 4., 21.],
        vec![
            2., 3., 4., 3., 5., 2., 6., 4., 17., 7., 19., 9., 23., 11., 29., 13.,
        ],
    ];
    for (case, zoom) in [0.5, 1., 2., 8.].into_iter().enumerate() {
        for hatch in [false, true] {
            let mut layer = StyledLayer::default();
            for (i, values) in recipes.iter().enumerate() {
                let y = (i as f64 - 1.) * 24. / zoom;
                let phase = if i == 0 { 7.25 } else { -11.5 };
                let data: Vec<f32> = if hatch {
                    vec![
                        -512.,
                        (y - 2.) as f32,
                        512.,
                        (y - 2.) as f32,
                        512.,
                        (y + 2.) as f32,
                        -512.,
                        (y - 2.) as f32,
                        512.,
                        (y + 2.) as f32,
                        -512.,
                        (y + 2.) as f32,
                    ]
                } else {
                    vec![-512., y as f32, 512., y as f32, 0., 3.]
                };
                let from = layer.data.len();
                layer.data.extend(&data);
                let kind = if hatch {
                    BatchKind::Fill {
                        paint: FillPaintBatch::Hatch {
                            color: [1.; 4],
                            angle: 0.,
                            spacing: 1024.,
                            width: 3. / zoom,
                            offset: y,
                            dash: Some(values.clone()),
                            dash_offset: phase,
                            stagger: 0.,
                            unit: Unit::World,
                        },
                    }
                } else {
                    BatchKind::Stroke {
                        color: [1.; 4],
                        width: 3. / zoom,
                        unit: Unit::World,
                        dash: Some(values.clone()),
                        dash_offset: phase,
                        cap: Cap::Butt,
                        blur: 0.,
                    }
                };
                layer.batches.push(StyledBatch {
                    range: from..layer.data.len(),
                    kind,
                    level: 0.,
                    key: i as u64,
                    bounds: [-512., y - 2., 512., y + 2.],
                    origin: [0.; 2],
                    reach: 2.,
                    reach_unit: Unit::World,
                    min_scale: None,
                    max_scale: None,
                    group: 0,
                    effect: None,
                    weight_cap: 0.,
                    line_type_loss: None,
                });
            }
            let id = 1000 + case as u64 * 2 + u64::from(hatch);
            let scene = StyledScene {
                layers: vec![Arc::new(StyledLayerPart { id, layer })],
                ..Default::default()
            };
            let cpu = kentos_native_style::cpu::paint(
                w as u32,
                h as u32,
                &kentos_native_style::cpu::CpuView {
                    m: [zoom, 0., 0., -zoom],
                    t: [w as f64 / 2., h as f64 / 2.],
                    px_per_m: zoom,
                    dpr: 1.,
                    scale: 100.,
                },
                &[&scene.layers[0].layer],
                &kentos_native_style::picture::NoImages,
            )
            .unwrap();
            gpu.styled = Some(scene);
            let camera = Camera {
                width: w as f64,
                height: h as f64,
                scale: zoom,
                ..Default::default()
            };
            let settings = RenderSettings::new(Rgba8::rgb(0, 0, 0));
            let image = gpu.draw(&[], &camera, &settings, w as u32, h as u32);
            for (i, values) in recipes.iter().enumerate() {
                let y = (i as f64 - 1.) * 24. / zoom;
                let phase = if i == 0 { 7.25 } else { -11.5 };
                let row = (h as f64 / 2. - y * zoom).floor() as isize;
                if row < 0 || row >= h as isize {
                    continue;
                }
                let mut checked = 0;
                for x in 2..w - 2 {
                    let ground = (x as f64 + 0.5 - w as f64 / 2.) / zoom;
                    let along = if hatch { ground } else { ground + 512. };
                    let (ink, edge) = expected(values, along + phase);
                    if edge * zoom < 1.5 {
                        continue;
                    }
                    let p = (row as usize * w + x) * 4;
                    assert_eq!(
                        image[p] > 128,
                        ink,
                        "GPU case={case} hatch={hatch} recipe={i} x={x}"
                    );
                    assert_eq!(
                        cpu[p + 3] > 128,
                        ink,
                        "CPU case={case} hatch={hatch} recipe={i} x={x}"
                    );
                    checked += 1;
                }
                assert!(checked > 30, "enough interior samples {checked}");
            }
            // Same scene/camera: stable image and retained buffer accounting.
            let bytes = gpu.renderer.stats(1).unwrap().resident_bytes;
            let again = gpu.draw(&[], &camera, &settings, w as u32, h as u32);
            assert_eq!(image, again);
            assert_eq!(gpu.renderer.stats(1).unwrap().resident_bytes, bytes);
        }
    }
    gpu.styled = old;
    gpu.keep = keep;
}
