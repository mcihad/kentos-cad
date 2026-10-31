//! Katman çizimi and draw effects on a real GPU (docs/adr/0226 §2–§4):
//! small scenes drawn through the compositor and read back, compared with
//! the pictures `scripts/fixtures/effects_cases.py` computes from the ADR's
//! definitions with numpy, without KentOS code
//! (`fixtures/compose/v1/cases.json`): an effect group's five effects, the
//! thirteen blending modes with opacity, and masks of turned label boxes;
//! single-sampled and with 4× multisampling.

use std::sync::Arc;

use kentos_native_style::batches::{
    BatchKind, Effect, FillPaintBatch, GlowFx, ShadowFx, StyledBatch, StyledLayer, Unit,
};
use kentos_render_wgpu::compose::{LabelBox, LayerComposeDraw, MaskSource};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Bounds, Camera, RenderSettings, Rgba8};
use serde_json::Value;

use super::Gpu;

const CASES: &str = include_str!("../../../../../fixtures/compose/v1/cases.json");

/// The scenes' size: one metre a pixel, the world's origin in the middle.
struct Frame {
    w: u32,
    h: u32,
}

impl Frame {
    /// A box of the cases' pixels (x right, y down from the top left) in the world.
    fn world(&self, [x0, y0, x1, y1]: [f64; 4]) -> [f64; 4] {
        let (hw, hh) = (f64::from(self.w) / 2.0, f64::from(self.h) / 2.0);
        [x0 - hw, hh - y1, x1 - hw, hh - y0]
    }
}

fn numbers(v: &Value) -> Vec<f64> {
    v.as_array()
        .expect("an array")
        .iter()
        .map(|x| x.as_f64().expect("a number"))
        .collect()
}

fn four(v: &Value) -> [f64; 4] {
    let n = numbers(v);
    [n[0], n[1], n[2], n[3]]
}

/// A layer of solid boxes (world), each its own batch; an effect group when `effect` is given.
fn boxes(rects: &[([f64; 4], [f64; 4])], effect: Option<Arc<Effect>>) -> StyledLayer {
    let mut data = Vec::new();
    let mut batches = Vec::new();
    for (i, (r, color)) in rects.iter().enumerate() {
        let [x0, y0, x1, y1] = r.map(|v| v as f32);
        let start = data.len();
        for [x, y] in [[x0, y0], [x1, y0], [x1, y1], [x0, y0], [x1, y1], [x0, y1]] {
            data.extend_from_slice(&[x, y]);
        }
        batches.push(StyledBatch {
            range: start..data.len(),
            kind: BatchKind::Fill {
                paint: FillPaintBatch::Solid { color: *color },
            },
            level: 0.0,
            key: i as u64 + 1,
            bounds: *r,
            origin: [0.0, 0.0],
            reach: 0.0,
            reach_unit: Unit::Px,
            min_scale: None,
            max_scale: None,
            group: u32::from(effect.is_some()),
            weight_cap: 0.0,
            effect: effect.clone(),
        });
    }
    StyledLayer {
        data,
        batches,
        ..StyledLayer::default()
    }
}

/// A shadow of the cases (device px, y down) as the batch carries it (y up).
fn shadow(v: &Value) -> ShadowFx {
    ShadowFx {
        dx: v["dx"].as_f64().expect("dx"),
        dy: -v["dy"].as_f64().expect("dy"),
        blur: v["blur"].as_f64().expect("blur"),
        color: four(&v["color"]),
    }
}

fn glow(v: &Value) -> GlowFx {
    GlowFx {
        spread: v["spread"].as_f64().expect("spread"),
        blur: v["blur"].as_f64().expect("blur"),
        color: four(&v["color"]),
    }
}

fn effect(v: &Value) -> Effect {
    Effect {
        unit: Unit::Px,
        shadow: v.get("shadow").map(shadow),
        glow: v.get("glow").map(glow),
        inner_shadow: v.get("innerShadow").map(shadow),
        inner_glow: v.get("innerGlow").map(glow),
        blur: v.get("blur").and_then(Value::as_f64).unwrap_or(0.0),
    }
}

/// The blending mode's number (`BlendMode::index`).
fn mode_index(name: &str) -> u32 {
    kentos_contracts::BlendMode::from_key(name)
        .unwrap_or_else(|| panic!("a mode: {name}"))
        .index()
}

/// The frame read back against the case's picture: no channel off by more than the tolerance.
fn compare(name: &str, samples: u32, f: &Frame, got: &[u8], want: &[u8], tolerance: u8) {
    assert_eq!(got.len(), want.len(), "{name}: the picture's size");
    let mut worst = (0u8, 0usize);
    let mut off = 0usize;
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        let d = g.abs_diff(*w);
        if d > worst.0 {
            worst = (d, i);
        }
        if d > tolerance {
            off += 1;
        }
    }
    let (d, i) = worst;
    let (x, y) = ((i / 4) as u32 % f.w, (i / 4) as u32 / f.w);
    eprintln!("compose {name} ({samples}×): largest difference {d}/255 at ({x}, {y})");
    if off > 0 {
        let dir = std::env::temp_dir().join("kentos-compose");
        let _ = std::fs::create_dir_all(&dir);
        let slug: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        for (which, rgba) in [("got", got), ("want", want)] {
            let mut ppm = format!("P6\n{} {}\n255\n", f.w, f.h).into_bytes();
            for p in rgba.chunks(4) {
                ppm.extend_from_slice(&p[..3]);
            }
            let _ = std::fs::write(dir.join(format!("{slug}-{samples}x-{which}.ppm")), ppm);
        }
        let c = i - i % 4;
        panic!(
            "{name} ({samples}×): {off} channels off by more than {tolerance}/255; at ({x}, {y}) {:?}, expected {:?} (pictures in {})",
            &got[c..c + 4],
            &want[c..c + 4],
            dir.display()
        );
    }
}

pub(super) fn layer_composition_and_effects_match_the_reference(gpu: &mut Gpu) {
    let cases: Value = serde_json::from_str(CASES).expect("cases.json reads");
    let size = numbers(&cases["size"]);
    let f = Frame {
        w: size[0] as u32,
        h: size[1] as u32,
    };
    let tolerance = cases["tolerance"].as_u64().expect("tolerance") as u8;
    let ground = numbers(&cases["ground"]);
    let mut settings = RenderSettings::new(Rgba8::rgb(
        ground[0] as u8,
        ground[1] as u8,
        ground[2] as u8,
    ));
    let mut camera = Camera {
        width: f64::from(f.w),
        height: f64::from(f.h),
        ..Camera::default()
    };
    let [x0, y0, x1, y1] = f.world([0.0, 0.0, f64::from(f.w), f64::from(f.h)]);
    camera.fit(
        &Bounds {
            min_x: x0,
            min_y: y0,
            max_x: x1,
            max_y: y1,
        },
        0.0,
    );
    assert!((camera.scale - 1.0).abs() < 1e-12, "one metre a pixel");
    let picture = |c: &Value| -> Vec<u8> {
        c["picture"]
            .as_array()
            .expect("picture")
            .iter()
            .map(|v| v.as_u64().expect("a byte") as u8)
            .collect()
    };
    // Each scene's layers by new ids: the renderer keeps what it uploaded by id.
    let mut id = 1000u64;
    let mut part = |layer: StyledLayer| {
        id += 1;
        Arc::new(StyledLayerPart { id, layer })
    };
    // The view draws into its own targets (a kept picture), so the frame composes.
    let keep = gpu.keep;
    gpu.keep = true;
    let mut draw = |gpu: &mut Gpu, name: &str, scene: StyledScene, want: &[u8]| {
        gpu.styled = Some(scene);
        for samples in [1, 4] {
            settings.samples = samples;
            let got = gpu.draw(&[], &camera, &settings, f.w, f.h);
            compare(name, samples, &f, &got, want, tolerance);
        }
    };

    for c in cases["effects"].as_array().expect("effects") {
        let name = c["name"].as_str().expect("name");
        let fx = Arc::new(effect(&c["effects"]));
        let layer = boxes(
            &[(f.world(four(&c["square"])), four(&c["color"]))],
            Some(fx),
        );
        let scene = StyledScene {
            layers: vec![part(layer)],
            ..StyledScene::default()
        };
        draw(gpu, name, scene, &picture(c));
    }

    for c in cases["blends"].as_array().expect("blends") {
        let name = c["name"].as_str().expect("name");
        let under = c["under"].as_array().expect("under");
        let (w, h) = (f64::from(f.w), f64::from(f.h));
        let half = (w / 2.0).floor();
        let below = boxes(
            &[
                (f.world([0.0, 0.0, half, h]), four(&under[0])),
                (f.world([half, 0.0, w, h]), four(&under[1])),
            ],
            None,
        );
        let top = boxes(&[(f.world(four(&c["square"])), four(&c["color"]))], None);
        let scene = StyledScene {
            layers: vec![part(below), part(top)],
            compose: vec![(
                1..2,
                Arc::new(LayerComposeDraw {
                    opacity: c["opacity"].as_f64().expect("opacity") as f32,
                    blend: mode_index(c["mode"].as_str().expect("mode")),
                    masks: Vec::new(),
                }),
            )],
            ..StyledScene::default()
        };
        draw(gpu, name, scene, &picture(c));
    }

    for c in cases["masks"].as_array().expect("masks") {
        let name = c["name"].as_str().expect("name");
        let band = boxes(&[(f.world(four(&c["band"])), four(&c["color"]))], None);
        // One source a box, each with its own growth: its middle in the world, its turn
        // counter-clockwise (the screen's turns the other way, y being down).
        let masks = c["boxes"]
            .as_array()
            .expect("boxes")
            .iter()
            .map(|b| {
                let n = numbers(b);
                let [cx, cy, hx, hy, turn, grow] = n[..] else {
                    panic!("a box of six numbers");
                };
                MaskSource {
                    markers: None,
                    labels: Arc::new(vec![LabelBox {
                        centre: [cx - f64::from(f.w) / 2.0, f64::from(f.h) / 2.0 - cy],
                        size: [(2.0 * hx) as f32, (2.0 * hy) as f32],
                        turn: (-turn).to_degrees() as f32,
                    }]),
                    grow: grow as f32,
                }
            })
            .collect();
        let scene = StyledScene {
            layers: vec![part(band)],
            compose: vec![(
                0..1,
                Arc::new(LayerComposeDraw {
                    opacity: c["opacity"].as_f64().expect("opacity") as f32,
                    blend: mode_index(c["mode"].as_str().expect("mode")),
                    masks,
                }),
            )],
            ..StyledScene::default()
        };
        draw(gpu, name, scene, &picture(c));
    }

    // The tests after these draw no styled layers, as before.
    settings.samples = 1;
    gpu.styled = Some(StyledScene::default());
    let _ = gpu.draw(&[], &camera, &settings, f.w, f.h);
    gpu.styled = None;
    gpu.keep = keep;
}
