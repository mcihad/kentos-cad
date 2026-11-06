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

/// Coordinates are nearby, but a later segment of a long path has a remote
/// cumulative distance. Its low part must survive phase evaluation too.
pub(super) fn remote_cumulative_phase_matches_local_reference(gpu: &mut Gpu) {
    let (w, h) = (512usize, 64usize);
    let recipe = vec![0.125, 0.0625, 0.25, 0.0625, 0.1875, 0.0625]; // exact binary period .75
    let source = 1_000_000_000.125f64;
    let hi = source as f32;
    let low = (source - f64::from(hi)) as f32;
    for (case, offset) in [0., 1_000_000_000.25, -1_000_000_000.25]
        .into_iter()
        .enumerate()
    {
        let batch = StyledBatch {
            range: 0..6,
            kind: BatchKind::Stroke {
                color: [1.; 4],
                width: 0.03,
                unit: Unit::World,
                dash: Some(recipe.clone()),
                dash_offset: offset,
                cap: Cap::Butt,
                blur: 0.,
            },
            level: 0.,
            key: 0,
            bounds: [-3., -0.02, 3., 0.02],
            origin: [0.; 2],
            reach: 0.02,
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
        let layer = StyledLayer {
            data: vec![-3., 0., 3., 0., hi, 0.],
            low: vec![0., 0., 0., 0., low, 0.],
            batches: vec![batch],
            ..Default::default()
        };
        let zoom = 100.;
        let cpu = kentos_native_style::cpu::paint(
            w as u32,
            h as u32,
            &kentos_native_style::cpu::CpuView {
                m: [zoom, 0., 0., -zoom],
                t: [w as f64 / 2., h as f64 / 2.],
                px_per_m: zoom,
                dpr: 1.,
                scale: 50.,
            },
            &[&layer],
            &kentos_native_style::picture::NoImages,
        )
        .unwrap();
        gpu.styled = Some(StyledScene {
            layers: vec![Arc::new(StyledLayerPart {
                id: 9191 + case as u64,
                layer,
            })],
            ..Default::default()
        });
        let camera = Camera {
            width: w as f64,
            height: h as f64,
            scale: zoom,
            ..Default::default()
        };
        let image = gpu.draw(
            &[],
            &camera,
            &RenderSettings::new(Rgba8::rgb(0, 0, 0)),
            w as u32,
            h as u32,
        );
        let mut checked = 0;
        for x in 2..w - 2 {
            let along = (x as f64 + 0.5 - w as f64 / 2.) / zoom + 3.;
            // Independent bounded phase; no billion-metre term reaches f32.
            let (ink, edge) = expected(&recipe, 0.375 + offset.rem_euclid(0.75) + along);
            if edge * zoom < 1.5 {
                continue;
            }
            let p = ((h / 2) * w + x) * 4;
            assert_eq!(cpu[p + 3] > 128, ink, "CPU remote phase x={x}");
            assert_eq!(image[p] > 128, ink, "GPU remote phase x={x}");
            checked += 1;
        }
        assert!(checked > 100);
    }
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
                    lod: None,
                    instances: None,
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

/// Measures changed-camera frames over one immutable layer, rather than
/// including texture allocation or a readback in every timed frame. Test
/// builds retain CPU debug overhead; report the profile, never infer FPS.
pub(super) fn complex_linetype_timing(gpu: &mut Gpu) {
    use kentos_contracts::{Entity, LayerStyle, ProjectSettings, ProjectStyles};
    use kentos_geometry_core::{Vec2, entity::Shape, store::Store};
    use kentos_native_style::{
        StylePalette, View,
        batches::{DecodeOptions, decode},
        library::StyleLibrary,
        line_types::LineTypes,
        program::{BuildOptions, ReadingUnit, build_layer},
    };
    use serde_json::{Value, json};
    use std::time::Instant;
    let resource: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/shape-resources.json"
    ))
    .unwrap();
    let resources: ProjectStyles = serde_json::from_value(resource["styles"].clone()).unwrap();
    let mut entities = Vec::<Entity>::new();
    let mut store = Store::new();
    // KENTOS_LINETYPE_LINES measures another count (docs/adr/1062 §6): past the 2 000 000 contour points of one
    // build the pattern is refused, said as `lineTypeLoss`, and the case is reported, not drawn short.
    let lines: u32 = std::env::var("KENTOS_LINETYPE_LINES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(800);
    for i in 0..lines {
        let y = f64::from(i) * 0.01;
        entities.push(
            serde_json::from_value(json!({"kind":"line","id":i+1,"layerId":"a","attrs":{},
            "a":{"x":0.,"y":y},"b":{"x":4.,"y":y}}))
            .unwrap(),
        );
        store.put_many([(
            f64::from(i + 1),
            "a",
            false,
            Shape::Line {
                a: Vec2::new(0., y),
                b: Vec2::new(4., y),
            },
        )]);
    }
    let list: Vec<_> = entities.iter().collect();
    let library = StyleLibrary::default();
    let palette = StylePalette {
        fg: "#ffffff".into(),
        fg_dim: "#999999".into(),
        ink: "#000000".into(),
        paper: "#ffffff".into(),
    };
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("CAD-ADV-11 timing"),
        size: wgpu::Extent3d {
            width: 1440,
            height: 900,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: super::FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target = target.create_view(&Default::default());
    let mut camera = Camera {
        width: 1440.,
        height: 900.,
        center: Vec2::new(2., 5.),
        scale: 80.,
        ..Default::default()
    };
    let mut settings = RenderSettings::new(Rgba8::rgb(0, 0, 0));
    println!(
        "CAD-ADV-11: profile={}, {lines} lines × 4m, 1:50, 1440x900, 40 pan/zoom frames after 4 warmups; GPU completion included, no pixel readback",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    for (case, mode) in ["short", "long12", "text", "shape"].into_iter().enumerate() {
        let started = Instant::now();
        let elements = match mode {
            "short" => json!([{"length":5.},{"length":-3.}]),
            "long12" => json!(
                (0..12)
                    .map(|i| json!({"length":if i%2==0{1.}else{-1.}}))
                    .collect::<Vec<_>>()
            ),
            "text" => {
                json!([{"length":5.},{"length":-1.},{"length":0.,"text":{"text":"HW","style":"STANDARD","scale":0.7}},{"length":-2.}])
            }
            _ => {
                json!([{"length":5.},{"length":-1.},{"length":0.,"shape":{"name":"FENCE","file":"fence.shx","scale":0.7}},{"length":-2.}])
            }
        };
        let project:ProjectSettings=serde_json::from_value(json!({"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.,
            "lineTypes":[{"name":"P","unit":"paper","elements":elements}]})).unwrap();
        let types = LineTypes::with_resources(&project, &resources);
        let style: LayerStyle = serde_json::from_value(
            json!({"color":"#ffffff","lineWeight":0.25,"lineType":"continuous","linePattern":"P"}),
        )
        .unwrap();
        let opts = BuildOptions {
            origin: Vec2::new(0., 0.),
            plot_scale: 50.,
            screen: false,
            hairlines: false,
            clip: None,
            library: &library,
            layer_name: &|_| "A".into(),
            view: View::default(),
            frame: None,
            reading_unit: ReadingUnit::METRE,
            layer_styles: None,
            line_types: Some(&types),
        };
        let (_, batches) = build_layer(&store, &style, &list, &opts).unwrap();
        if batches.json.contains("lineTypeLoss") {
            assert_ne!(lines, 800, "{mode}: the shared 800-line case stays within the limit");
            println!(
                "{mode}: {lines} lines refused past the contour point limit ({:.1} ms, lineTypeLoss said)",
                started.elapsed().as_secs_f64() * 1000.
            );
            continue;
        }
        let layer = decode(
            batches,
            &DecodeOptions {
                palette: &palette,
                plot_scale: 50.,
                library: &library,
                view: Default::default(),
                hold_weights: false,
            },
        )
        .unwrap();
        let build_ms = started.elapsed().as_secs_f64() * 1000.;
        let n = layer.batches.len();
        let bytes = (layer.data.len() + layer.low.len()) * std::mem::size_of::<f32>();
        gpu.styled = Some(StyledScene {
            layers: vec![Arc::new(StyledLayerPart {
                id: 88_000 + case as u64,
                layer,
            })],
            ..Default::default()
        });
        for samples in [1, 4] {
            settings.samples = samples;
            let mut times = Vec::new();
            let mut cpus = Vec::new();
            let mut cold = 0.;
            for frame in 0..44 {
                camera.center.x = 2. + f64::from(frame % 2) / 80.;
                camera.scale = if frame % 4 < 2 { 80. } else { 81. };
                let (total, cpu) = super::compose_timing::frame(gpu, &target, &camera, &settings);
                if frame == 0 {
                    cold = total;
                }
                if frame >= 4 {
                    times.push(total);
                    cpus.push(cpu);
                }
            }
            times.sort_by(f64::total_cmp);
            cpus.sort_by(f64::total_cmp);
            println!(
                "{mode}: samples={samples}, source_build_ms={build_ms:.3}, batches={n}, geometry_bytes={bytes}, first_frame_ms={cold:.3}, median_frame_ms={:.3}, p95_frame_ms={:.3}, median_cpu_submit_ms={:.3}",
                times[20], times[37], cpus[20]
            );
        }
    }
    gpu.styled = None;
}

/// ADR 1124: a dot (a zero-length dash) is a disc of the line's weight on the GPU and its CPU twin, for a short
/// recipe (the uniform's eight) and a long one (the storage page), and the dot on a corner is drawn once. Expected
/// from the discs' own distances, not from a rendered image.
pub(super) fn round_dots_match_the_disc(gpu: &mut Gpu) {
    let old = gpu.styled.take();
    let keep = gpu.keep;
    gpu.keep = false;
    let (w, h) = (512usize, 128usize);
    let zoom = 100.;
    let (width, period) = (0.2, 0.6); // 20 px across, a dot every 60 px
    let short = vec![0., period];
    let long: Vec<f64> = (0..10).map(|i| if i % 2 == 0 { 0. } else { period }).collect();
    for (case, recipe) in [short, long].into_iter().enumerate() {
        // Along y = 0 from x = −2 to 1, then up to (1, 0.6): a dot on the corner and one on the end.
        let data = vec![-2., 0., 1., 0., 0., 1., 1., 0., 1., 0.6, 3., 2.];
        let batch = StyledBatch {
            range: 0..12,
            kind: BatchKind::Stroke {
                color: [1., 1., 1., 0.5],
                width,
                unit: Unit::World,
                dash: Some(recipe.clone()),
                dash_offset: 0.,
                cap: Cap::Butt,
                blur: 0.,
            },
            level: 0.,
            key: 0,
            bounds: [-2.2, -0.2, 1.2, 0.8],
            origin: [0.; 2],
            reach: 0.2,
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
        let layer = StyledLayer { data, batches: vec![batch], ..Default::default() };
        let view = kentos_native_style::cpu::CpuView {
            m: [zoom, 0., 0., -zoom],
            t: [w as f64 / 2., h as f64 / 2.],
            px_per_m: zoom,
            dpr: 1.,
            scale: 100.,
        };
        let cpu = kentos_native_style::cpu::paint(w as u32, h as u32, &view, &[&layer], &kentos_native_style::picture::NoImages)
            .unwrap();
        gpu.styled = Some(StyledScene {
            layers: vec![Arc::new(StyledLayerPart { id: 1124 + case as u64, layer })],
            ..Default::default()
        });
        let camera = Camera { width: w as f64, height: h as f64, scale: zoom, ..Default::default() };
        let image = gpu.draw(&[], &camera, &RenderSettings::new(Rgba8::rgb(0, 0, 0)), w as u32, h as u32);
        // The dots' centres every 0.6 m along the path: five on y = 0 from x = −2, the corner, the end.
        let centres: Vec<(f64, f64)> =
            (0..5).map(|k| (-2. + 0.6 * f64::from(k), 0.)).chain([(1., 0.), (1., 0.6)]).collect();
        let r = width / 2.;
        let (mut inside, mut outside) = (0, 0);
        for y in 0..h {
            for x in 0..w {
                let gx = (x as f64 + 0.5 - w as f64 / 2.) / zoom;
                let gy = (h as f64 / 2. - y as f64 - 0.5) / zoom;
                let d = centres.iter().map(|c| (gx - c.0).hypot(gy - c.1)).fold(f64::MAX, f64::min);
                let p = (y * w + x) * 4;
                // Half-transparent white over black: a dot drawn once is about 128, twice about 191.
                let (g, c) = (image[p], cpu[p + 3]);
                if (d - r) * zoom < -1.5 {
                    assert!((110..=145).contains(&g), "GPU case={case} inside a dot, once, at ({x},{y}): {g}");
                    assert!((110..=145).contains(&c), "CPU case={case} inside a dot, once, at ({x},{y}): {c}");
                    inside += 1;
                } else if (d - r) * zoom > 1.5 && (gy.abs() < r || (gx - 1.).abs() < r) {
                    assert!(g < 20, "GPU case={case} off the dots at ({x},{y}): {g}");
                    assert!(c < 20, "CPU case={case} off the dots at ({x},{y}): {c}");
                    outside += 1;
                }
            }
        }
        // Seven discs of 10 px radius (the top one partly above the picture).
        assert!(inside > 1200 && outside > 4000, "case={case}: {inside} inside, {outside} outside");
    }
    gpu.styled = old;
    gpu.keep = keep;
}
