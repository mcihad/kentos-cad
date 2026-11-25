//! A pattern tile's SVG marks drawn from their triangles (plan/render.md RM-11, docs/adr/9239): past the scale where
//! its bitmap would be magnified the GPU draws what the program's reading on the CPU (`vector_tiles::covers`) says,
//! pixel for pixel; and just below that scale the bitmap draws the same marks. The tile is staggered, turned and
//! shifted, its mark an L (no turn or mirror of it looks the same), turned and moved in its cell.
use std::sync::Arc;

use kentos_native_style::batches::{
    AtlasImage, BatchKind, FillPaintBatch, MarkerLook, StyledBatch, StyledLayer, TileMark, Unit,
};
use kentos_native_style::svg_marks::VECTORS_FROM_PX;
use kentos_native_style::vector_tiles::{covers, program, switch};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8, Vec2};

use super::Gpu;

const ELL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><path d="M1 1 H9 V3 H3 V8 H1 Z" fill="#3366cc"/></svg>"##;
/// The tile: 10 m across, 7 m a row, staggered (two rows high), turned and shifted.
const SIZE: [f64; 2] = [10.0, 14.0];
const ANGLE: f64 = 0.3;
const OFFSET: [f64; 2] = [0.4, -0.2];

fn image() -> AtlasImage {
    AtlasImage::Tile {
        key: "ell".into(),
        aspect: 1.4,
        stagger: true,
        draw: vec![TileMark {
            look: MarkerLook::Image {
                image: AtlasImage::Svg {
                    key: "ell-svg".into(),
                    svg: ELL.into(),
                    width: 20.0,
                    height: 10.0,
                },
                fit_height: false,
            },
            w: 0.45,
            h: 0.45,
            offset: [0.06, -0.04],
            rotation: 0.35,
        }],
    }
}

fn scene(id: u64) -> StyledScene {
    let data = vec![
        -60., -60., 60., -60., 60., 60., -60., -60., 60., 60., -60., 60.,
    ];
    let batch = StyledBatch {
        range: 0..data.len(),
        kind: BatchKind::Fill {
            paint: FillPaintBatch::Tile {
                image: image(),
                size: SIZE,
                angle: ANGLE,
                offset: OFFSET,
                opacity: 1.0,
                unit: Unit::World,
            },
        },
        level: 0.,
        key: id,
        bounds: [-60., -60., 60., 60.],
        origin: [0., 0.],
        reach: 0.,
        reach_unit: Unit::World,
        min_scale: None,
        max_scale: None,
        group: 0,
        effect: None,
        weight_cap: 0.0,
        line_type_loss: None,
        lod: None,
        instances: None,
    };
    StyledScene {
        layers: vec![Arc::new(StyledLayerPart::new(
            id,
            StyledLayer {
                data,
                batches: vec![batch],
                ..Default::default()
            },
        ))],
        ..Default::default()
    }
}

/// Pixels clearly inside a mark or clearly outside (the program reads the same at the pixel's middle and its four
/// corners) that the picture shows the other way: inked (blue over the black ground) or not.
fn disagreements(pixels: &[u8], size: u32, centre: Vec2, scale: f64) -> (usize, usize) {
    let program = program(&image()).expect("a program");
    let (s, c) = ANGLE.sin_cos();
    let read = |x: f64, y: f64| {
        let w = [
            centre.x + (x - f64::from(size) / 2.0) / scale,
            centre.y - (y - f64::from(size) / 2.0) / scale,
        ];
        let t = [
            c * w[0] + s * w[1] - OFFSET[0],
            -s * w[0] + c * w[1] - OFFSET[1],
        ];
        covers(&program, SIZE, t).is_some()
    };
    let (mut wrong, mut inked) = (0, 0);
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (f64::from(x), f64::from(y));
            let middle = read(fx + 0.5, fy + 0.5);
            if [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
                .iter()
                .any(|(dx, dy)| read(fx + dx, fy + dy) != middle)
            {
                continue;
            }
            let i = ((y * size + x) * 4) as usize;
            let blue = pixels[i + 2] > 128 && pixels[i] < 128;
            inked += usize::from(middle);
            wrong += usize::from(blue != middle);
        }
    }
    (wrong, inked)
}

pub(super) fn a_magnified_tile_draws_its_marks_from_their_triangles(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let mut settings = RenderSettings::new(Rgba8::rgb(0, 0, 0));
    let px = 192;
    // The scale (px a metre) past which the triangles draw.
    let at = VECTORS_FROM_PX / switch(&image(), SIZE).expect("the tile has a program");
    // On a mark's edge: close up the view holds ink and ground, its edge across the middle.
    let program = program(&image()).expect("a program");
    let (s, c) = ANGLE.sin_cos();
    let inked = |x: f64, y: f64| {
        covers(
            &program,
            SIZE,
            [c * x + s * y - OFFSET[0], -s * x + c * y - OFFSET[1]],
        )
        .is_some()
    };
    let edge = (0..1000 * 40)
        .map(|n| {
            (
                -10.0 + f64::from(n % 1000) * 0.02,
                -10.0 + f64::from(n / 1000) * 0.5,
            )
        })
        .find(|&(x, y)| inked(x, y) && !inked(x + 0.02, y))
        .expect("a mark's edge near the origin");
    let centre = Vec2::new(edge.0, edge.1);
    for samples in [1, 4] {
        settings.samples = samples;
        for (case, scale) in [at * 0.95, at * 1.05, at * 6.0, at * 40.0]
            .into_iter()
            .enumerate()
        {
            let camera = Camera {
                center: centre,
                width: f64::from(px),
                height: f64::from(px),
                scale,
                ..Default::default()
            };
            gpu.styled = Some(scene(80_000 + case as u64 + u64::from(samples) * 10));
            let pixels = gpu.draw(&[], &camera, &settings, px, px);
            let (wrong, inked) = disagreements(&pixels, px, centre, scale);
            assert!(
                inked > 200,
                "case {case}: the marks are in view ({inked} px)"
            );
            assert!(
                wrong <= inked / 200,
                "case {case} at {scale:.1} px/m, {samples} samples: {wrong} of {inked} px drawn the other way"
            );
        }
    }
    gpu.styled = previous;
}
