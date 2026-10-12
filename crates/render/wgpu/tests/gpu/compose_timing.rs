//! Katman çizimi's cost in a frame on a real GPU (docs/adr/0226 §7): a
//! 1440 × 900 drawing area with land use, a relief drawn with Çoğalt at
//! 70 %, roads masked by 300 labels and 2 000 buildings with a shadow, an
//! outer glow and an inner glow, against the same layers drawn straight into
//! the picture; and each composed part alone, for where the time goes. Every
//! frame pans a pixel, so the kept picture is drawn again (a pan), and is
//! waited for: the time is the CPU's and the GPU's. The values are the scene's
//! at 1:1000 (a millimetre 3.78 px). Prints the frames' medians, the scenes
//! taken in turn, single-sampled and with 4× multisampling, against the budget
//! (8 ms added); release, by hand:
//!
//! ```text
//! cargo test --release -p kentos-render-wgpu --test gpu composition_timing -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::time::Instant;

use kentos_native_style::batches::{
    BatchKind, Effect, FillPaintBatch, GlowFx, ShadowFx, StyledBatch, StyledLayer, Unit,
};
use kentos_render_wgpu::compose::{LabelBox, LayerComposeDraw, MaskSource};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Bounds, Camera, FrameInput, RenderSettings, Rgba8, Vec2};

use super::Gpu;

const W: u32 = 1440;
const H: u32 = 900;
/// A millimetre on the screen at 96 dpi, px.
const MM: f64 = 96.0 / 25.4;

/// A box (x0, y0, x1, y1) in the world: one metre a pixel, the origin in the middle, y up.
type Rect = [f64; 4];

/// One batch of solid boxes in a colour; an effect group when `effect` is given.
fn batch(
    data: &mut Vec<f32>,
    rects: &[Rect],
    color: [f64; 4],
    key: u64,
    effect: Option<&Arc<Effect>>,
) -> StyledBatch {
    let start = data.len();
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for r in rects {
        let [x0, y0, x1, y1] = r.map(|v| v as f32);
        for [x, y] in [[x0, y0], [x1, y0], [x1, y1], [x0, y0], [x1, y1], [x0, y1]] {
            data.extend_from_slice(&[x, y]);
        }
        bounds = [
            bounds[0].min(r[0]),
            bounds[1].min(r[1]),
            bounds[2].max(r[2]),
            bounds[3].max(r[3]),
        ];
    }
    StyledBatch {
        range: start..data.len(),
        kind: BatchKind::Fill {
            paint: FillPaintBatch::Solid { color },
        },
        level: 0.0,
        key,
        bounds,
        origin: [0.0, 0.0],
        reach: 0.0,
        reach_unit: Unit::Px,
        min_scale: None,
        max_scale: None,
        group: u32::from(effect.is_some()),
        effect: effect.cloned(),
    }
}

/// A layer of batches, each a colour and its boxes.
fn layer(batches: &[(&[Rect], [f64; 4])], effect: Option<&Arc<Effect>>) -> StyledLayer {
    let mut data = Vec::new();
    let made = batches
        .iter()
        .enumerate()
        .map(|(i, (rects, color))| batch(&mut data, rects, *color, i as u64 + 1, effect))
        .collect();
    StyledLayer {
        data,
        batches: made,
        ..StyledLayer::default()
    }
}

fn grey(v: f64) -> [f64; 4] {
    [v, v, v, 1.0]
}

/// What a measured scene composes: the relief's Çoğalt, the roads' mask, the buildings' effects.
#[derive(Clone, Copy)]
struct Parts {
    relief: bool,
    roads: bool,
    shadow: bool,
    glow: bool,
    inner_glow: bool,
    /// The shadow without its blur: the group, its alpha packed and the effect pass alone.
    sharp: bool,
}

const NONE: Parts = Parts {
    relief: false,
    roads: false,
    shadow: false,
    glow: false,
    inner_glow: false,
    sharp: false,
};

/// The scene's four layers, the buildings with the effects asked for.
fn layers(parts: Parts) -> [StyledLayer; 4] {
    let (hw, hh) = (f64::from(W) / 2.0, f64::from(H) / 2.0);
    // Land use: twelve blocks in four colours.
    let colors = [
        [0.63, 0.85, 0.61, 1.0],
        [1.0, 0.89, 0.57, 1.0],
        [0.99, 0.73, 0.63, 1.0],
        [0.8, 0.85, 0.9, 1.0],
    ];
    let blocks: Vec<Vec<Rect>> = (0..4)
        .map(|c| {
            (0..12)
                .filter(|k| k % 4 == c)
                .map(|k| {
                    let (col, row) = (f64::from(k % 4), f64::from(k / 4));
                    [
                        -hw + col * 360.0,
                        -hh + row * 300.0,
                        -hw + (col + 1.0) * 360.0,
                        -hh + (row + 1.0) * 300.0,
                    ]
                })
                .collect()
        })
        .collect();
    let land = layer(
        &blocks
            .iter()
            .zip(colors)
            .map(|(b, c)| (b.as_slice(), c))
            .collect::<Vec<_>>(),
        None,
    );
    // The relief: sixteen bands from light to dark grey.
    let bands: Vec<(Vec<Rect>, [f64; 4])> = (0..16)
        .map(|k| {
            let x = -hw + f64::from(k) * 90.0;
            (
                vec![[x, -hh, x + 90.0, hh]],
                grey(0.94 - f64::from(k) * 0.025),
            )
        })
        .collect();
    let relief = layer(
        &bands
            .iter()
            .map(|(r, c)| (r.as_slice(), *c))
            .collect::<Vec<_>>(),
        None,
    );
    // Roads: twelve east–west and eighteen north–south, 2.4 mm wide.
    let half = 1.2 * MM;
    let mut roads: Vec<Rect> = (0..12)
        .map(|k| {
            let y = -hh + 37.5 + f64::from(k) * 75.0;
            [-hw, y - half, hw, y + half]
        })
        .collect();
    roads.extend((0..18).map(|k| {
        let x = -hw + 40.0 + f64::from(k) * 80.0;
        [x - half, -hh, x + half, hh]
    }));
    let roads = layer(&[(&roads, [1.0, 1.0, 1.0, 1.0])], None);
    // Buildings: 2 000, 50 by 40.
    let buildings: Vec<Rect> = (0..2000)
        .map(|k| {
            let (col, row) = (f64::from(k % 50), f64::from(k / 50));
            let (x, y) = (-hw + 6.0 + col * 28.8, -hh + 4.0 + row * 22.5);
            [x, y, x + 18.0, y + 14.0]
        })
        .collect();
    let effects = parts.shadow || parts.glow || parts.inner_glow;
    let effect = effects.then(|| {
        Arc::new(Effect {
            unit: Unit::Px,
            shadow: parts.shadow.then(|| ShadowFx {
                dx: 1.2 * MM * std::f64::consts::FRAC_1_SQRT_2,
                dy: -1.2 * MM * std::f64::consts::FRAC_1_SQRT_2,
                blur: if parts.sharp { 0.0 } else { 1.5 * MM },
                color: [0.0, 0.0, 0.0, 0.45],
            }),
            glow: parts.glow.then_some(GlowFx {
                spread: 0.5 * MM,
                blur: 1.5 * MM,
                color: [1.0, 0.82, 0.0, 0.6],
            }),
            inner_shadow: None,
            inner_glow: parts.inner_glow.then_some(GlowFx {
                spread: 1.0 * MM,
                blur: 2.5 * MM,
                color: [1.0, 1.0, 1.0, 0.8],
            }),
            blur: 0.0,
        })
    });
    let buildings = layer(&[(&buildings, [0.85, 0.8, 0.75, 1.0])], effect.as_ref());
    [land, relief, roads, buildings]
}

/// The 300 labels cutting into the roads, turned every way, grown by 1 mm.
fn labels() -> MaskSource {
    let boxes = (0..300u32)
        .map(|k| LabelBox {
            centre: [
                f64::from((k * 97) % 1400) - 700.0,
                f64::from((k * 61) % 860) - 430.0,
            ],
            size: [64.0, 14.0],
            turn: ((k * 37) % 180) as f32 - 90.0,
        })
        .collect();
    MaskSource {
        markers: None,
        labels: Arc::new(boxes),
        grow: MM as f32,
    }
}

/// One frame into `view` as the desktop draws its area (the kept picture's own targets, composed
/// into the frame), waited for: milliseconds from preparing to the GPU's end, and to the submission.
fn frame(
    gpu: &mut Gpu,
    view: &wgpu::TextureView,
    camera: &Camera,
    settings: &RenderSettings,
) -> (f64, f64) {
    let t = Instant::now();
    gpu.renderer
        .prepare(
            &gpu.device,
            &gpu.queue,
            1,
            &[],
            &FrameInput {
                camera,
                origin: Vec2::new(0.0, 0.0),
                size_px: [W as f32, H as f32],
                origin_px: [0.0, 0.0],
                scale_factor: 1.0,
                settings,
                keep_picture: true,
                overlays: 0,
            },
        )
        .expect("the frame prepares");
    if let Some(scene) = &gpu.styled {
        let _ = gpu
            .renderer
            .prepare_styled(&gpu.device, &gpu.queue, 1, scene, &super::RedTiles);
    }
    assert!(
        gpu.renderer.owns_targets(1),
        "a kept picture draws into its own targets"
    );
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    gpu.renderer.render(&mut encoder, view, [0, 0, W, H], 1);
    let index = gpu.queue.submit([encoder.finish()]);
    let cpu = t.elapsed().as_secs_f64() * 1000.0;
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: None,
        })
        .expect("the frame finishes");
    (t.elapsed().as_secs_f64() * 1000.0, cpu)
}

/// The frames' times (ms) and the median.
fn median(mut ms: Vec<f64>) -> f64 {
    ms.sort_by(f64::total_cmp);
    ms[ms.len() / 2]
}

pub(super) fn composition_timing(gpu: &mut Gpu) {
    let mut settings = RenderSettings::new(Rgba8::rgb(223, 223, 223));
    let mut camera = Camera {
        width: f64::from(W),
        height: f64::from(H),
        ..Camera::default()
    };
    let (hw, hh) = (f64::from(W) / 2.0, f64::from(H) / 2.0);
    camera.fit(
        &Bounds {
            min_x: -hw,
            min_y: -hh,
            max_x: hw,
            max_y: hh,
        },
        0.0,
    );
    let mut id = 5000u64;
    let mut scene = |parts: Parts| {
        let made = layers(parts).map(|layer| {
            id += 1;
            Arc::new(StyledLayerPart { id, layer })
        });
        let mut compose = Vec::new();
        if parts.relief {
            compose.push((
                1..2,
                Arc::new(LayerComposeDraw {
                    opacity: 0.7,
                    blend: kentos_contracts::BlendMode::Multiply.index(),
                    masks: Vec::new(),
                }),
            ));
        }
        if parts.roads {
            compose.push((
                2..3,
                Arc::new(LayerComposeDraw {
                    opacity: 1.0,
                    blend: kentos_contracts::BlendMode::Normal.index(),
                    masks: vec![labels()],
                }),
            ));
        }
        StyledScene {
            layers: made.to_vec(),
            compose,
            ..StyledScene::default()
        }
    };
    let all = Parts {
        relief: true,
        roads: true,
        shadow: true,
        glow: true,
        inner_glow: true,
        sharp: false,
    };
    let configs = [
        ("yalın", scene(NONE)),
        ("hepsi", scene(all)),
        (
            "yalnız çoğaltan kabartma",
            scene(Parts {
                relief: true,
                ..NONE
            }),
        ),
        (
            "yalnız maskelenen yollar",
            scene(Parts {
                roads: true,
                ..NONE
            }),
        ),
        (
            "yalnız gölge",
            scene(Parts {
                shadow: true,
                ..NONE
            }),
        ),
        (
            "yalnız bulanıksız gölge",
            scene(Parts {
                shadow: true,
                sharp: true,
                ..NONE
            }),
        ),
        ("yalnız dış parıltı", scene(Parts { glow: true, ..NONE })),
        (
            "yalnız iç parıltı",
            scene(Parts {
                inner_glow: true,
                ..NONE
            }),
        ),
    ];
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: super::FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    println!(
        "ADR 0226 §7: release, {W} × {H}, a pixel's pan a frame; medians of 80 frames, the scenes in turn"
    );
    for samples in [1, 4] {
        settings.samples = samples;
        let mut times = vec![Vec::new(); configs.len()];
        let mut cpus = vec![Vec::new(); configs.len()];
        for round in 0..9 {
            for (k, (_, scene)) in configs.iter().enumerate() {
                gpu.styled = Some(scene.clone());
                for i in 0..12 {
                    camera.pan_by(if i % 2 == 0 { 1.0 } else { -1.0 }, 0.0);
                    let (ms, cpu) = frame(gpu, &view, &camera, &settings);
                    // The first round warms the GPU's clock; the first frames after a change upload.
                    if round > 0 && i >= 2 {
                        times[k].push(ms);
                        cpus[k].push(cpu);
                    }
                }
            }
        }
        let medians: Vec<f64> = times.into_iter().map(median).collect();
        let cpu: Vec<f64> = cpus.into_iter().map(median).collect();
        let (base, with) = (medians[0], medians[1]);
        let added = with - base;
        println!(
            "{:<70} {added:>7.2} ms   bütçe 8 ms  {}   (yalın {base:.2} ms, birleştirmeyle {with:.2} ms; CPU {:.2} → {:.2} ms)",
            format!(
                "Çoğaltan, maskelenen (300 etiket) ve üç efektli (2 000 bina) katman, {samples}×"
            ),
            if added <= 8.0 { "✓" } else { "✗" },
            cpu[0],
            cpu[1]
        );
        for (((name, _), t), c) in configs.iter().zip(&medians).zip(&cpu).skip(2) {
            println!(
                "    {:<66} {:>7.2} ms   (CPU +{:.2} ms)",
                format!("{name}, {samples}×"),
                t - base,
                c - cpu[0]
            );
        }
    }
    gpu.styled = None;
}
