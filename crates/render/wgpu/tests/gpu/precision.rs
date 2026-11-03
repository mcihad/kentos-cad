//! Sub-millimetre styled geometry, real GPU readback, including a camera pan and twist.
use super::Gpu;
use kentos_native_style::batches::{
    BatchKind, Cap, FillPaintBatch, MarkerLook, StyledBatch, StyledLayer, Unit,
};
use kentos_render_wgpu::{
    Camera, RenderSettings, Rgba8, Vec2,
    styled::{StyledLayerPart, StyledScene},
};
use std::sync::Arc;

fn scene(center: Vec2, kind: usize, id: u64) -> StyledScene {
    let origin = [
        (center.x / 65536.).round() * 65536.,
        (center.y / 65536.).round() * 65536.,
    ];
    let corners = [
        (-0.0003, -0.00025),
        (0.0003, -0.00025),
        (0.0003, 0.00025),
        (-0.0003, 0.00025),
    ];
    let p: Vec<_> = corners
        .iter()
        .map(|&(x, y)| [center.x + x - origin[0], center.y + y - origin[1]])
        .collect();
    let (batch_kind, values) = match kind {
        0 => (
            BatchKind::Fill {
                paint: FillPaintBatch::Solid { color: [1.; 4] },
            },
            [p[0], p[1], p[2], p[0], p[2], p[3]]
                .into_iter()
                .flatten()
                .collect::<Vec<f64>>(),
        ),
        1 => (
            BatchKind::Stroke {
                color: [1.; 4],
                width: 1.,
                unit: Unit::Px,
                dash: None,
                dash_offset: 0.,
                cap: Cap::Butt,
                blur: 0.,
            },
            (0..4)
                .flat_map(|i| {
                    [
                        p[i][0],
                        p[i][1],
                        p[(i + 1) % 4][0],
                        p[(i + 1) % 4][1],
                        0.,
                        0.,
                    ]
                })
                .collect(),
        ),
        _ => (
            BatchKind::Marker {
                unit: Unit::World,
                look: MarkerLook::Shape {
                    shape: "rectangle".into(),
                    fill: Some([1.; 4]),
                    stroke: None,
                    stroke_width: 0.,
                    params: [0.; 4],
                },
                offset: [0.; 2],
                anchor: [0.; 2],
                opacity: 1.,
                extent: [0.0006, 0.0005],
            },
            vec![
                center.x - origin[0],
                center.y - origin[1],
                0.,
                0.0006,
                0.0005,
            ],
        ),
    };
    let data: Vec<f32> = values.iter().map(|&x| x as f32).collect();
    let low = values
        .iter()
        .zip(&data)
        .map(|(&x, &hi)| (x - f64::from(hi)) as f32)
        .collect();
    let len = data.len();
    StyledScene {
        layers: vec![Arc::new(StyledLayerPart {
            id,
            layer: StyledLayer {
                data,
                low,
                batches: vec![StyledBatch {
                    range: 0..len,
                    kind: batch_kind,
                    level: 0.,
                    key: id,
                    bounds: [
                        center.x - 0.0003,
                        center.y - 0.00025,
                        center.x + 0.0003,
                        center.y + 0.00025,
                    ],
                    origin,
                    reach: 2.,
                    reach_unit: Unit::Px,
                    min_scale: None,
                    max_scale: None,
                    group: 0,
                    effect: None,
                    weight_cap: 0.,
                }],
                ..Default::default()
            },
        })],
        ..Default::default()
    }
}

pub(super) fn tiny_symbols_keep_their_height(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let keep = gpu.keep;
    gpu.keep = true;
    let settings = RenderSettings::new(Rgba8::rgb(0, 0, 0));
    let size = 64;
    for (i, center) in [
        Vec2::new(0., 0.),
        Vec2::new(4_000_000_000., 1_000_000.),
        Vec2::new(-4_000_000_000., -1_000_000.),
    ]
    .into_iter()
    .enumerate()
    {
        for kind in 0..3 {
            gpu.styled = Some(scene(center, kind, 97000 + (i * 3 + kind) as u64));
            for pan in [0., 0.000013] {
                for twist in [0., std::f64::consts::FRAC_PI_2] {
                    let camera = Camera {
                        width: size as f64,
                        height: size as f64,
                        center: Vec2::new(center.x + pan, center.y - pan),
                        scale: 10000.,
                        twist,
                        ..Default::default()
                    };
                    let rgba = gpu.draw_scaled(&[], &camera, &settings, size, size, 2.);
                    let mut bounds = [size, size, 0, 0];
                    for y in 0..size {
                        for x in 0..size {
                            if rgba[((y * size + x) * 4) as usize] > 128 {
                                bounds[0] = bounds[0].min(x);
                                bounds[1] = bounds[1].min(y);
                                bounds[2] = bounds[2].max(x);
                                bounds[3] = bounds[3].max(y);
                            }
                        }
                    }
                    assert!(
                        bounds[2] >= bounds[0] && bounds[3] >= bounds[1],
                        "blank kind={kind} at {center:?}"
                    );
                    let w = bounds[2] - bounds[0] + 1;
                    let h = bounds[3] - bounds[1] + 1;
                    let expected = if twist == 0. { [12, 10] } else { [10, 12] };
                    assert!(
                        w.abs_diff(expected[0]) <= 3 && h.abs_diff(expected[1]) <= 3,
                        "kind={kind} center={center:?} pan={pan} twist={twist}: {w}x{h}, expected {expected:?}"
                    );
                }
            }
        }
    }
    gpu.styled = previous;
    gpu.keep = keep;
}
