//! A styled part's numbers on the GPU only (plan/render.md RM-19, docs/adr/9235): a part made `on_gpu` lets its numbers
//! go once a view uploaded them; the view keeps its buffers while the app holds the part, drawn or not, and a renderer
//! without them says so instead of drawing what it no longer has. A part the CPU still reads keeps them.
use std::sync::Arc;

use kentos_native_style::batches::{BatchKind, Cap, MarkerLook, StyledBatch, StyledLayer, Unit};
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings, Renderer, Rgba8};

use super::{FORMAT, Gpu};

const SIZE: u32 = 64;

fn layer(kind: BatchKind, data: Vec<f32>) -> StyledLayer {
    let batch = StyledBatch {
        range: 0..data.len(),
        kind,
        level: 0.,
        key: 1,
        bounds: [-1000., -1000., 1000., 1000.],
        origin: [0., 0.],
        reach: 1.,
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
    StyledLayer {
        low: vec![0.; data.len()],
        data,
        batches: vec![batch],
        ..Default::default()
    }
}

/// A white line through the view's middle, 8 m wide.
fn line() -> StyledLayer {
    let kind = BatchKind::Stroke {
        color: [1.; 4],
        width: 8.,
        unit: Unit::World,
        dash: None,
        dash_offset: 0.,
        cap: Cap::Butt,
        blur: 0.,
    };
    layer(kind, vec![-1000., 0., 1000., 0., 0., 0.])
}

/// One circle marker in the view's middle.
fn mark() -> StyledLayer {
    let kind = BatchKind::Marker {
        unit: Unit::Px,
        look: MarkerLook::Shape {
            shape: "circle".into(),
            fill: Some([1.; 4]),
            stroke: None,
            stroke_width: 0.,
            params: [0.; 4],
        },
        offset: [0., 0.],
        anchor: [0.5, 0.5],
        opacity: 1.,
        extent: [8., 8.],
        vectors: false,
    };
    layer(kind, vec![0., 0., 0., 8., 8.])
}

fn scene(parts: &[&Arc<StyledLayerPart>]) -> StyledScene {
    StyledScene {
        layers: parts.iter().map(|p| Arc::clone(p)).collect(),
        ..Default::default()
    }
}

fn inked(rgba: &[u8]) -> usize {
    rgba.as_chunks::<4>().0.iter().filter(|p| p[0] > 128).count()
}

pub(super) fn styled_numbers_live_on_the_gpu(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let keep = gpu.keep;
    gpu.keep = false;
    let settings = RenderSettings::new(Rgba8::rgb(0, 0, 0));
    let camera = Camera {
        width: f64::from(SIZE),
        height: f64::from(SIZE),
        scale: 1.,
        ..Default::default()
    };
    let draw = |gpu: &mut Gpu, styled: StyledScene| {
        gpu.styled = Some(styled);
        gpu.draw(&[], &camera, &settings, SIZE, SIZE)
    };
    let held = |gpu: &Gpu| gpu.renderer.stats(1).expect("the view").resident_bytes;
    let _ = draw(gpu, StyledScene::default());
    let empty = held(gpu);

    // The app's part (its cache holds it) drawn: the CPU lets its numbers go.
    let part = Arc::new(StyledLayerPart::on_gpu(95_001, line()));
    let drawn = draw(gpu, scene(&[&part]));
    assert!(inked(&drawn) >= SIZE as usize * 8, "the line draws: {} px", inked(&drawn));
    assert!(part.numbers().is_none(), "the CPU let its numbers go");
    assert!(part.on_gpu_only());
    let resident = held(gpu);
    assert!(resident > empty, "its buffers are held");

    // Hidden for a while (its layer off, the drawing under the deep view): its buffers stay, it comes back as it was.
    for _ in 0..4 {
        let _ = draw(gpu, StyledScene::default());
        gpu.renderer.trim();
    }
    assert_eq!(held(gpu), resident, "kept while the app holds the part");
    assert_eq!(draw(gpu, scene(&[&part])), drawn, "the same line, from the kept buffers");
    assert!(!gpu.renderer.stats(1).expect("the view").parts_lost);

    // Undrawn (a sheet over the drawing area): the view holding the only copy is not released.
    for _ in 0..4 {
        gpu.renderer.trim();
    }
    assert!(gpu.renderer.stats(1).is_some(), "the view holding the only copy stays");

    // A renderer that never had its buffers (one made again): nothing drawn for it, and it says it lost the part.
    let had = std::mem::replace(
        &mut gpu.renderer,
        Renderer::new(&gpu.device, FORMAT).expect("the pipelines build"),
    );
    let lost = draw(gpu, scene(&[&part]));
    assert_eq!(inked(&lost), 0, "nothing drawn from numbers it does not have");
    assert!(gpu.renderer.stats(1).expect("the view").parts_lost);
    gpu.renderer = had;
    assert_eq!(draw(gpu, scene(&[&part])), drawn, "the renderer that has them draws it");

    // The app lets go: the buffers go with it, and the undrawn view is released.
    drop(part);
    let _ = draw(gpu, StyledScene::default());
    assert_eq!(held(gpu), empty, "the buffers went with the part");
    for _ in 0..4 {
        gpu.renderer.trim();
    }
    assert!(gpu.renderer.stats(1).is_none(), "nothing keeps the undrawn view");

    // A part the CPU reads while drawing (a marker's box for a mask) keeps its numbers, as does one made `new`.
    let marks = Arc::new(StyledLayerPart::on_gpu(95_002, mark()));
    let kept = Arc::new(StyledLayerPart::new(95_003, line()));
    let _ = draw(gpu, scene(&[&marks, &kept]));
    assert!(marks.numbers().is_some(), "a marker part keeps its numbers");
    assert!(kept.numbers().is_some(), "a part made new keeps its numbers");
    assert!(!marks.on_gpu_only() && !kept.on_gpu_only());

    let _ = draw(gpu, StyledScene::default());
    gpu.styled = previous;
    gpu.keep = keep;
}
