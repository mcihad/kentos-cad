//! A measurement, not a test (run by hand, `--ignored`): what a far view of a city's lines costs the GPU. A million
//! short segments over 20 km, each under a pixel or two at the far scale, drawn again and again without reading the
//! picture back; the frames' wall time while the GPU is the bottleneck, by variant.
use std::sync::Arc;
use std::time::Instant;

use kentos_native_style::batches::{BatchKind, Cap, StyledBatch, StyledLayer, Unit};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, FrameInput, RenderSettings, Rgba8, Vec2};

use super::{FORMAT, Gpu, RedTiles};

const SIDE: f64 = 20_000.0;

fn scatter(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

/// `n` segments 5–30 m long, in runs of a few along a path (a parcel's sides), batches of `per` segments.
fn city(n: usize, per: usize, dash: bool, alpha: f64) -> StyledLayer {
    let mut seed = 3u64;
    let mut data = Vec::with_capacity(n * 6);
    let mut batches = Vec::new();
    let stroke = BatchKind::Stroke {
        color: [0.1, 0.1, 0.1, alpha],
        width: 0.3,
        unit: Unit::World,
        dash: dash.then(|| vec![2.0, 1.0]),
        dash_offset: 0.0,
        cap: Cap::Butt,
        blur: 0.0,
    };
    while data.len() < n * 6 {
        let start = data.len();
        while data.len() < start + per * 6 && data.len() < n * 6 {
            let (mut x, mut y) = (scatter(&mut seed) * SIDE, scatter(&mut seed) * SIDE);
            for _ in 0..4 {
                let (a, l) = (scatter(&mut seed) * std::f64::consts::TAU, 5.0 + scatter(&mut seed) * 25.0);
                let (nx, ny) = (x + a.cos() * l, y + a.sin() * l);
                data.extend([x as f32, y as f32, nx as f32, ny as f32, 0.0, 3.0]);
                (x, y) = (nx, ny);
            }
        }
        batches.push(StyledBatch {
            range: start..data.len(),
            kind: stroke.clone(),
            level: 0.0,
            key: batches.len() as u64,
            bounds: [-50.0, -50.0, SIDE + 50.0, SIDE + 50.0],
            origin: [0.0, 0.0],
            reach: 1.3,
            reach_unit: Unit::World,
            min_scale: None,
            max_scale: None,
            group: 0,
            effect: None,
            weight_cap: 0.0,
            line_type_loss: None,
            lod: None,
            instances: None,
        });
    }
    StyledLayer { data, batches, ..Default::default() }
}

/// The milliseconds of `frames` frames of `scene` through `camera`, each submitted and waited for, no read back.
fn frames_ms(gpu: &mut Gpu, scene: StyledScene, camera: &Camera, settings: &RenderSettings, frames: usize) -> Vec<f64> {
    let (width, height) = (camera.width as u32, camera.height as u32);
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("far"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    gpu.styled = Some(scene);
    let mut out = Vec::new();
    for k in 0..frames + 2 {
        gpu.renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                1,
                &[],
                &FrameInput {
                    camera,
                    origin: Vec2::new(0.0, 0.0),
                    size_px: [width as f32, height as f32],
                    origin_px: [0.0, 0.0],
                    scale_factor: 1.0,
                    settings,
                    keep_picture: false,
                    moving: true,
                    drawing: None,
                    overlays: 0,
                },
            )
            .expect("the frame prepares");
        if let Some(scene) = &gpu.styled {
            let _ = gpu.renderer.prepare_styled(&gpu.device, &gpu.queue, 1, scene, &RedTiles);
        }
        let t = Instant::now();
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if !gpu.renderer.owns_targets(1) {
                gpu.renderer.draw(&mut pass, 1);
            }
        }
        if gpu.renderer.owns_targets(1) {
            gpu.renderer.render(&mut encoder, &view, [0, 0, width, height], 1);
        }
        let index = gpu.queue.submit([encoder.finish()]);
        gpu.device
            .poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: None })
            .expect("the frame finishes");
        // The first two warm the caches up.
        if k >= 2 {
            out.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    gpu.styled = None;
    out
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

pub(super) fn a_far_city_by_variant(gpu: &mut Gpu) {
    let camera = Camera {
        center: Vec2::new(SIDE / 2.0, SIDE / 2.0),
        width: 1440.0,
        height: 900.0,
        scale: 1440.0 / SIDE / 1.05,
        ..Default::default()
    };
    let n: usize = std::env::var("KENTOS_FAR_ITEMS").ok().and_then(|v| v.parse().ok()).unwrap_or(1_000_000);
    let mut id = 70_000u64;
    for (name, per, dash, alpha, samples) in [
        ("düz, saydam, 4×", 8_000, false, 0.6, 4),
        ("düz, saydam, 1×", 8_000, false, 0.6, 1),
        ("düz, opak, 4×", 8_000, false, 1.0, 4),
        ("kesikli, saydam, 4×", 8_000, true, 0.6, 4),
        ("düz, saydam, 4×, toplu 100 000", 100_000, false, 0.6, 4),
    ] {
        let mut settings = RenderSettings::new(Rgba8::rgb(255, 255, 255));
        settings.samples = samples;
        id += 1;
        let layer = city(n, per, dash, alpha);
        let scene = StyledScene { layers: vec![Arc::new(StyledLayerPart::new(id, layer))], ..Default::default() };
        let ms = frames_ms(gpu, scene, &camera, &settings, 30);
        let m = median(ms.clone());
        eprintln!(
            "UZAK {name}: {n} parça, kare medyan {m:.1} ms (en az {:.1}, en çok {:.1}); parça başına {:.0} ns",
            ms.iter().copied().fold(f64::MAX, f64::min),
            ms.iter().copied().fold(f64::MIN, f64::max),
            m * 1e6 / n as f64
        );
    }
}
