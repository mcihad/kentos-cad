//! plan/render.md RM-08, docs/adr/9241: a large batch's items sorted by cell, a view drawing only the cells it meets,
//! draws what the batch drawn whole draws, with far fewer items near the ground. Over one another a batch's one colour
//! gives the same pixel in any order; the target's eight bits round each blend, so where many half-transparent items
//! of the batch overlap a pixel (a far view) it may land a step or two apart.
use std::sync::Arc;

use kentos_native_style::batches::{BatchKind, Cap, FillPaintBatch, MarkerLook, StyledBatch, StyledLayer, Unit};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8, Vec2};

use super::Gpu;

const SIDE: f64 = 4_000.0;

fn scatter(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn batch(kind: BatchKind, range: std::ops::Range<usize>, key: u64) -> StyledBatch {
    StyledBatch {
        range,
        kind,
        level: 0.0,
        key,
        bounds: [0.0, 0.0, SIDE, SIDE],
        origin: [0.0, 0.0],
        reach: 1.0,
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

/// A few segments far away (a batch no view here shows, before the large ones: each batch's cells are its own, shown
/// or not), then 30 000 dashed, half-transparent segments and 12 000 half-transparent triangles over 4 km.
fn layer() -> StyledLayer {
    let mut seed = 7u64;
    let mut data = Vec::new();
    for k in 0..10 {
        let x = 100_000.0 + f64::from(k);
        data.extend([x as f32, 100_000.0, (x + 0.5) as f32, 100_000.5, 0.0, 3.0]);
    }
    let away = 0..data.len();
    for i in 0..30_000 {
        let (x, y) = (scatter(&mut seed) * SIDE, scatter(&mut seed) * SIDE);
        let (dx, dy) = (scatter(&mut seed) * 16.0 - 8.0, scatter(&mut seed) * 16.0 - 8.0);
        data.extend([x as f32, y as f32, (x + dx) as f32, (y + dy) as f32, (i % 7) as f32, 3.0]);
    }
    let strokes = away.end..data.len();
    for _ in 0..12_000 {
        let (x, y) = (scatter(&mut seed) * SIDE, scatter(&mut seed) * SIDE);
        for _ in 0..3 {
            data.extend([(x + scatter(&mut seed) * 6.0) as f32, (y + scatter(&mut seed) * 6.0) as f32]);
        }
    }
    let fills = strokes.end..data.len();
    let stroke = BatchKind::Stroke {
        color: [0.1, 0.3, 0.8, 0.6],
        width: 0.8,
        unit: Unit::World,
        dash: Some(vec![2.0, 1.0]),
        dash_offset: 0.0,
        cap: Cap::Butt,
        blur: 0.0,
    };
    let fill = BatchKind::Fill { paint: FillPaintBatch::Solid { color: [0.8, 0.2, 0.1, 0.5] } };
    let mut far = batch(stroke.clone(), away, 3);
    far.bounds = [100_000.0, 100_000.0, 100_010.0, 100_001.0];
    StyledLayer {
        data,
        batches: vec![far, batch(fill, fills, 1), batch(stroke, strokes, 2)],
        ..Default::default()
    }
}

fn scene(id: u64, layer: &StyledLayer) -> StyledScene {
    StyledScene { layers: vec![Arc::new(StyledLayerPart::new(id, layer.clone()))], ..Default::default() }
}

pub(super) fn a_view_draws_its_cells_as_the_batch_drawn_whole(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let mut settings = RenderSettings::new(Rgba8::rgb(255, 255, 255));
    let whole = layer();
    let mut sorted = whole.clone();
    kentos_native_style::cells::add(&mut sorted);
    assert!(sorted.cells.len() == 3 && sorted.cells[0].is_none(), "the far batch is too small for cells");
    assert!(sorted.cells[1..].iter().all(Option::is_some), "both large batches have cells");
    let px = 256;
    for samples in [1, 4] {
        settings.samples = samples;
        // Two near views, the whole drawing, and 512 m of it: items a few pixels long, the cells at the view's edges cut.
        for (case, (centre, scale)) in [
            ((1_000.0, 2_500.0), 8.0),
            ((3_100.0, 700.0), 40.0),
            ((2_000.0, 2_000.0), 0.06),
            ((1_500.0, 1_500.0), 0.5),
        ]
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
            let id = 60_000 + case as u64 * 10 + u64::from(samples) * 100;
            gpu.styled = Some(scene(id, &whole));
            let a = gpu.draw(&[], &camera, &settings, px, px);
            let whole_items = gpu.renderer.stats(1).expect("the view").styled_items;
            gpu.styled = Some(scene(id + 1, &sorted));
            let b = gpu.draw(&[], &camera, &settings, px, px);
            let cell_items = gpu.renderer.stats(1).expect("the view").styled_items;
            let most = a.iter().zip(&b).map(|(p, q)| p.abs_diff(*q)).max().unwrap_or(0);
            let differ = a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count();
            eprintln!("cells case {case}, {samples} samples: items {whole_items} -> {cell_items}, {differ} px differ, most {most}");
            match case {
                // Near the ground: a few cells' items, the same picture.
                0 | 1 => {
                    assert!(cell_items * 4 < whole_items, "case {case}: {cell_items} of {whole_items} items drawn");
                    assert_eq!(differ, 0, "case {case}: {differ} pixels differ");
                }
                // Everything in view, in the cells' order: where many half-transparent items overlap a pixel the
                // eight-bit rounding of each blend may land a step or two apart.
                2 => {
                    assert_eq!(cell_items, whole_items, "a view over everything draws everything");
                    assert!(most <= 2 && differ * 100 < 3 * (a.len() / 4), "far: {differ} pixels differ, most {most} steps");
                }
                // Part of the drawing, items a few pixels long: an edge left out would differ by far more than a step.
                _ => {
                    assert!(cell_items * 4 < whole_items, "case {case}: {cell_items} of {whole_items} items drawn");
                    assert!(most <= 2, "case {case}: {differ} pixels differ, most {most} steps");
                }
            }
        }
        an_edge_cell_whose_lines_reach_the_view_is_drawn(gpu, &mut settings);
        markers_keep_their_order(gpu, &mut settings);
    }
    gpu.styled = previous;
}

/// 20 000 circles 6 m wide over 2 km, filled and outlined in two half-transparent colours, block by block of 250 m as a
/// drawing lays them: where two overlap, which is on top shows.
fn rings() -> StyledLayer {
    let mut seed = 11u64;
    let mut places: Vec<(f64, f64)> =
        (0..20_000).map(|_| (scatter(&mut seed) * 2_000.0, scatter(&mut seed) * 2_000.0)).collect();
    places.sort_by_key(|&(x, y)| ((y / 250.0) as u32, (x / 250.0) as u32));
    let data: Vec<f32> = places.iter().flat_map(|&(x, y)| [x as f32, y as f32, 0.0, 6.0, 6.0]).collect();
    let kind = BatchKind::Marker {
        unit: Unit::World,
        look: MarkerLook::Shape {
            shape: "circle".into(),
            fill: Some([0.9, 0.6, 0.1, 0.6]),
            stroke: Some([0.1, 0.2, 0.7, 0.8]),
            stroke_width: 0.8,
            params: [0.0; 4],
        },
        offset: [0.0, 0.0],
        anchor: [0.0, 0.0],
        opacity: 1.0,
        extent: [6.0, 6.0],
        vectors: false,
    };
    let mut b = batch(kind, 0..data.len(), 5);
    b.bounds = [0.0, 0.0, 2_000.0, 2_000.0];
    // As a marker's decoding gives it: its symbol's extent and a half, two units more.
    b.reach = 11.0;
    StyledLayer { data, batches: vec![b], ..Default::default() }
}

/// A marker batch keeps its order (cells.rs): a view draws the runs of the items its cells hold, so the picture is the
/// batch drawn whole near the ground and far out, where the overlapping circles' order shows.
fn markers_keep_their_order(gpu: &mut Gpu, settings: &mut RenderSettings) {
    let whole = rings();
    let mut kept = whole.clone();
    kentos_native_style::cells::add(&mut kept);
    assert_eq!(kept.data, whole.data, "the markers' numbers where they were");
    assert!(kept.cells[0].as_ref().is_some_and(|c| c.parts[0].order.is_some()), "cells in the markers' order");
    let px = 256u32;
    for (case, (centre, scale)) in [((700.0, 1_300.0), 4.0), ((1_650.0, 300.0), 12.0), ((1_000.0, 1_000.0), 0.12)].into_iter().enumerate() {
        let camera = Camera {
            center: Vec2::new(centre.0, centre.1),
            width: f64::from(px),
            height: f64::from(px),
            scale,
            ..Default::default()
        };
        let id = 62_000 + case as u64 * 10 + u64::from(settings.samples) * 100;
        gpu.styled = Some(scene(id, &whole));
        let a = gpu.draw(&[], &camera, settings, px, px);
        let whole_items = gpu.renderer.stats(1).expect("the view").styled_items;
        gpu.styled = Some(scene(id + 1, &kept));
        let b = gpu.draw(&[], &camera, settings, px, px);
        let kept_items = gpu.renderer.stats(1).expect("the view").styled_items;
        let differ = a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count();
        eprintln!("cells markers case {case}, {} samples: items {whole_items} -> {kept_items}, {differ} px differ", settings.samples);
        assert_eq!(differ, 0, "case {case}: {differ} pixels differ");
        if case < 2 {
            assert!(kept_items * 10 < whole_items, "case {case}: {kept_items} of {whole_items} markers drawn");
        }
    }
}

/// 32 × 32 upright lines 50 m apart, 20 m long, 0.8 m wide: a part of 4 × 4 cells, its first column of cells ending
/// at x = 350 m (columns 0 to 7), the next starting at 400 m.
fn lattice() -> StyledLayer {
    let mut data = Vec::new();
    for i in 0..32 {
        for j in 0..32 {
            let (x, y) = (f64::from(i) * 50.0, f64::from(j) * 50.0);
            data.extend([x as f32, y as f32, x as f32, (y + 20.0) as f32, 0.0, 3.0]);
        }
    }
    let stroke = BatchKind::Stroke {
        color: [0.1, 0.3, 0.8, 0.6],
        width: 0.8,
        unit: Unit::World,
        dash: None,
        dash_offset: 0.0,
        cap: Cap::Butt,
        blur: 0.0,
    };
    let mut b = batch(stroke, 0..data.len(), 4);
    b.bounds = [0.0, 0.0, 1_550.0, 1_570.0];
    // The batch's reach as a stroke's decoding gives it: its width and a unit.
    b.reach = 1.8;
    StyledLayer { data, batches: vec![b], ..Default::default() }
}

/// 10 m a pixel, the view's left edge 3 m (0.3 px) right of the first cells' last lines: past the batch's reach (1.8 m),
/// but a line's smoothing reaches a pixel and a half from it, into the view's first pixels. Those cells are drawn.
fn an_edge_cell_whose_lines_reach_the_view_is_drawn(gpu: &mut Gpu, settings: &mut RenderSettings) {
    let whole = lattice();
    let mut sorted = whole.clone();
    kentos_native_style::cells::add(&mut sorted);
    let part = &sorted.cells[0].as_ref().expect("the lattice has cells").parts[0];
    assert_eq!((part.nx, part.ny), (4, 4), "four by four cells");
    assert!((part.boxes[0][2] - 350.0).abs() < 0.01, "the first column of cells ends at 350 m");
    let px = 256u32;
    let scale = 0.1;
    let left = 353.0;
    let camera = Camera {
        center: Vec2::new(left + f64::from(px) / 2.0 / scale, 785.0),
        width: f64::from(px),
        height: f64::from(px),
        scale,
        ..Default::default()
    };
    let id = 61_000 + u64::from(settings.samples) * 100;
    gpu.styled = Some(scene(id, &whole));
    let a = gpu.draw(&[], &camera, settings, px, px);
    gpu.styled = Some(scene(id + 1, &sorted));
    let b = gpu.draw(&[], &camera, settings, px, px);
    let edge = (0..px as usize).any(|y| a[y * px as usize * 4..y * px as usize * 4 + 3].iter().any(|&v| v < 250));
    assert!(edge, "the first cells' lines tint the view's first column");
    let most = a.iter().zip(&b).map(|(p, q)| p.abs_diff(*q)).max().unwrap_or(0);
    let differ = a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count();
    eprintln!("cells edge, {} samples: {differ} px differ, most {most}", settings.samples);
    assert_eq!(differ, 0, "the view's edge: {differ} pixels differ, most {most} steps");
}
