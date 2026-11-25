//! A stroke's butt ends and its dashes' edges are smoothed across them as its sides are (docs/adr/9249): a pixel's ink
//! is the share of the pixel's span the line, or the dash, covers, half the ramp outside the edge and half inside. On a
//! real GPU and on the CPU twin, against the share itself. Before, a butt end's ramp lay wholly outside the end (the
//! line an eighth of a pixel long, a half step at the end) and a dash's wholly inside (each dash an eighth short at
//! either end, a half step at each edge).
use std::ops::Range;
use std::sync::Arc;

use kentos_native_style::batches::{BatchKind, Cap, StyledBatch, StyledLayer, Unit};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8};

use super::Gpu;

const W: usize = 256;
const H: usize = 32;

/// The share of the pixel span [c - 1/2, c + 1/2] inside [a, b].
fn share(c: f64, a: f64, b: f64) -> f64 {
    ((c + 0.5).min(b) - (c - 0.5).max(a)).clamp(0.0, 1.0)
}

fn batch(range: Range<usize>, dash: Option<Vec<f64>>, offset: f64) -> StyledBatch {
    StyledBatch {
        range,
        kind: BatchKind::Stroke {
            color: [1.0; 4],
            width: 5.0,
            unit: Unit::World,
            dash,
            dash_offset: offset,
            cap: Cap::Butt,
            blur: 0.0,
        },
        level: 0.0,
        key: 0,
        bounds: [-128.0, -4.0, 128.0, 4.0],
        origin: [0.0; 2],
        reach: 3.0,
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

pub(super) fn butt_ends_and_dash_edges_smooth_as_the_sides(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    // A world unit a pixel; the ends and the dashes' edges off the pixels' middles.
    let (x0, x1) = (-100.3, 97.7);
    let pattern = vec![7.3, 4.6];
    let offset = 0.2;
    for (case, dash) in [None, Some(pattern)].into_iter().enumerate() {
        let layer = StyledLayer {
            data: vec![x0 as f32, 0.0, x1 as f32, 0.0, 0.0, 3.0],
            batches: vec![batch(0..6, dash.clone(), offset)],
            ..Default::default()
        };
        let expected = |c: f64| match &dash {
            None => share(c, x0, x1),
            Some(p) => {
                let total: f64 = p.iter().sum();
                let t = (c - x0 + offset).rem_euclid(total);
                let mut acc = 0.0;
                let mut ink = 0.0;
                for (i, l) in p.iter().enumerate() {
                    if i % 2 == 0 {
                        for k in [-1.0, 0.0, 1.0] {
                            ink += share(t, acc + k * total, acc + l + k * total);
                        }
                    }
                    acc += l;
                }
                f64::min(ink, 1.0)
            }
        };
        gpu.styled = Some(StyledScene {
            layers: vec![Arc::new(StyledLayerPart::new(98_000 + case as u64, layer.clone()))],
            ..Default::default()
        });
        let camera = Camera { width: W as f64, height: H as f64, scale: 1.0, ..Default::default() };
        let image = gpu.draw(&[], &camera, &RenderSettings::new(Rgba8::rgb(0, 0, 0)), W as u32, H as u32);
        let cpu = kentos_native_style::cpu::paint(
            W as u32,
            H as u32,
            &kentos_native_style::cpu::CpuView {
                m: [1.0, 0.0, 0.0, -1.0],
                t: [W as f64 / 2.0, H as f64 / 2.0],
                px_per_m: 1.0,
                dpr: 1.0,
                scale: 50.0,
            },
            &[&layer],
            &kentos_native_style::picture::NoImages,
        )
        .expect("the CPU twin paints");
        let (mut checked, mut on_edges) = (0, 0);
        for x in 0..W {
            let c = x as f64 + 0.5 - W as f64 / 2.0;
            // The dashed case's own edges: away from the line's ends, where both would ramp.
            if dash.is_some() && (c < x0 + 3.0 || c > x1 - 3.0) {
                continue;
            }
            let want = 255.0 * expected(c);
            let p = ((H / 2) * W + x) * 4;
            let (on_gpu, on_cpu) = (f64::from(image[p]), f64::from(cpu[p + 3]));
            assert!(
                (on_gpu - want).abs() <= 2.0,
                "case {case}: GPU pixel {x} (at {c}) is {on_gpu}, the share {want:.1}"
            );
            assert!(
                (on_cpu - want).abs() <= 2.0,
                "case {case}: CPU pixel {x} (at {c}) is {on_cpu}, the share {want:.1}"
            );
            checked += 1;
            if want > 1.0 && want < 254.0 {
                on_edges += 1;
            }
        }
        // The solid line's two ends, each a pixel inside and one outside; a dash's two edges each period.
        assert!(checked > 150, "case {case}: {checked} pixels checked");
        assert!(on_edges >= if dash.is_some() { 24 } else { 2 }, "case {case}: {on_edges} pixels on an edge");
    }
    gpu.styled = previous;
}
