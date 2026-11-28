//! A ground-sized text marker drawn from its glyphs' outlines past [`VECTORS_FROM_PX`] (plan/render.md RM-11,
//! docs/adr/9245), its bitmap below: on both sides of the switch and far past it, the ink is where the letter's outline
//! says, away from its edges.
use std::sync::Arc;

use kentos_geometry_core::Vec2 as GVec2;
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::text::Font;
use kentos_geometry_core::text::shaping::{OutlineOp, placed};
use kentos_native_style::batches::{
    AtlasImage, BatchKind, MarkerLook, StyledBatch, StyledLayer, Unit,
};
use kentos_native_style::picture::{Segment, TextOutline};
use kentos_native_style::svg_marks::VECTORS_FROM_PX;
use kentos_render_wgpu::styled::{StyledLayerPart, StyledScene};
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8, Vec2};

use super::{Gpu, RedTiles};

/// The letters' size the test's faces give, px.
const SIZE: f64 = 50.0;
/// The marker's height on the ground, metres.
const HEIGHT: f64 = 10.0;

/// The letters of `text` as a host's faces give them (`ImageSource::text`): the bundled face at [`SIZE`] px, x from
/// the pen's start, y down from the baseline.
pub(super) fn letters(text: &str) -> Option<Arc<TextOutline>> {
    let place = TextPlace {
        p: GVec2::new(0.0, 0.0),
        text,
        height: SIZE,
        rotation: 0.0,
        align: None,
        width_factor: None,
        box_width: None,
        line_spacing: None,
        runs: &[],
        flow: None,
        font: None,
        bold: false,
        italic: false,
        lean: 0.0,
        path: None,
    };
    let layout = placed::layout(&place, Font::DEFAULT, false).ok()?;
    let p = |q: [f64; 2]| [q[0] as f32, -q[1] as f32];
    let glyphs = layout
        .glyphs
        .iter()
        .filter_map(|g| g.outline().ok())
        .map(|ops| {
            ops.into_iter()
                .map(|op| match op {
                    OutlineOp::Move(a) => Segment::Move(p(a)),
                    OutlineOp::Line(a) => Segment::Line(p(a)),
                    OutlineOp::Quad(c, a) => Segment::Quad(p(c), p(a)),
                    OutlineOp::Cubic(c1, c2, a) => Segment::Cubic(p(c1), p(c2), p(a)),
                    OutlineOp::Close => Segment::Close,
                })
                .collect()
        })
        .collect();
    Some(Arc::new(TextOutline {
        size: SIZE as f32,
        advance: layout.width as f32,
        glyphs,
    }))
}

/// The outline's contours flattened (y down, the outline's px).
fn contours(outline: &TextOutline) -> Vec<Vec<[f64; 2]>> {
    let mut out: Vec<Vec<[f64; 2]>> = Vec::new();
    let f = |p: [f32; 2]| [f64::from(p[0]), f64::from(p[1])];
    for glyph in &outline.glyphs {
        let mut at = [0.0, 0.0];
        for s in glyph {
            match *s {
                Segment::Move(p) => {
                    out.push(vec![f(p)]);
                    at = f(p);
                }
                Segment::Line(p) => {
                    out.last_mut().expect("a contour").push(f(p));
                    at = f(p);
                }
                Segment::Quad(c, p) => {
                    let (c, p) = (f(c), f(p));
                    for k in 1..=32 {
                        let t = f64::from(k) / 32.0;
                        let u = 1.0 - t;
                        out.last_mut().expect("a contour").push([
                            u * u * at[0] + 2.0 * u * t * c[0] + t * t * p[0],
                            u * u * at[1] + 2.0 * u * t * c[1] + t * t * p[1],
                        ]);
                    }
                    at = p;
                }
                Segment::Cubic(c1, c2, p) => {
                    let (c1, c2, p) = (f(c1), f(c2), f(p));
                    for k in 1..=32 {
                        let t = f64::from(k) / 32.0;
                        let u = 1.0 - t;
                        let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                        out.last_mut().expect("a contour").push([
                            w[0] * at[0] + w[1] * c1[0] + w[2] * c2[0] + w[3] * p[0],
                            w[0] * at[1] + w[1] * c1[1] + w[2] * c2[1] + w[3] * p[1],
                        ]);
                    }
                    at = p;
                }
                Segment::Close => {}
            }
        }
    }
    out
}

/// Whether the outline's contours hold `q` by their winding (nonzero).
fn holds(contours: &[Vec<[f64; 2]>], q: [f64; 2]) -> bool {
    let mut winding = 0i32;
    for c in contours {
        for k in 0..c.len() {
            let (a, b) = (c[k], c[(k + 1) % c.len()]);
            let side = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]);
            if a[1] <= q[1] && b[1] > q[1] && side > 0.0 {
                winding += 1;
            } else if a[1] > q[1] && b[1] <= q[1] && side < 0.0 {
                winding -= 1;
            }
        }
    }
    winding != 0
}

fn layer(text: &str) -> StyledLayer {
    let image = AtlasImage::Text {
        key: format!("t|{text}"),
        text: text.into(),
        font: None,
        weight: 400.0,
        italic: false,
        color: "#1428a0".into(),
        halo: None,
    };
    let kind = BatchKind::Marker {
        unit: Unit::World,
        look: MarkerLook::Image {
            image,
            fit_height: true,
        },
        offset: [0.0, 0.0],
        anchor: [0.0, 0.0],
        opacity: 1.0,
        extent: [HEIGHT, HEIGHT],
        vectors: false,
    };
    let data = vec![0.0f32, 0.0, 0.0, HEIGHT as f32, HEIGHT as f32];
    let batch = StyledBatch {
        range: 0..data.len(),
        kind,
        level: 0.0,
        key: 9,
        bounds: [-HEIGHT, -HEIGHT, HEIGHT, HEIGHT],
        origin: [0.0, 0.0],
        reach: HEIGHT * 3.0,
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
    let mut layer = StyledLayer {
        data,
        batches: vec![batch],
        ..Default::default()
    };
    kentos_native_style::svg_marks::add(&mut layer, Some(&RedTiles));
    layer
}

pub(super) fn a_magnified_text_marker_draws_its_outline(gpu: &mut Gpu) {
    let previous = gpu.styled.take();
    let text = "E";
    let outline = letters(text).expect("the bundled face's letters");
    let contours = contours(&outline);
    let layer = layer(text);
    assert_eq!(
        layer.batches.len(),
        2,
        "the bitmap's batch and the letters' vectors"
    );
    // The box (`svg_marks::text_svg`): 100 units high, the letters 80, the baseline at 78; as wide as the advance.
    let s = 80.0 / SIZE;
    let width = f64::from(outline.advance) * s;
    let ground = |x: f64, y: f64| -> bool {
        // From the marker's place (the box's middle, y up) to the box's units (from its top left, y down), then the
        // outline's px.
        let u = (x + HEIGHT * width / 100.0 / 2.0) * 100.0 / HEIGHT;
        let v = (HEIGHT / 2.0 - y) * 100.0 / HEIGHT;
        holds(&contours, [u / s, (v - 78.0) / s])
    };
    let at = VECTORS_FROM_PX / HEIGHT;
    // Close up the view looks at an edge of the letter: the first ink-to-ground step along the box's middle row.
    let half_width = HEIGHT * width / 100.0 / 2.0;
    let edge = (0..4000)
        .map(|n| -half_width + f64::from(n) * half_width / 2000.0)
        .find(|&x| ground(x, 0.0) && !ground(x + half_width / 2000.0, 0.0))
        .expect("an edge of the letter on the middle row");
    let px = 192u32;
    let mut settings = RenderSettings::new(Rgba8::rgb(255, 255, 255));
    for samples in [1, 4] {
        settings.samples = samples;
        for (case, scale) in [at * 0.95, at * 1.05, at * 4.0, at * 20.0]
            .into_iter()
            .enumerate()
        {
            let centre = if case < 2 {
                Vec2::new(0.0, 0.0)
            } else {
                Vec2::new(edge, 0.0)
            };
            let camera = Camera {
                center: centre,
                width: f64::from(px),
                height: f64::from(px),
                scale,
                ..Default::default()
            };
            let id = 95_000 + case as u64 + u64::from(samples) * 10;
            gpu.styled = Some(StyledScene {
                layers: vec![Arc::new(StyledLayerPart::new(id, layer.clone()))],
                ..Default::default()
            });
            let pixels = gpu.draw(&[], &camera, &settings, px, px);
            let (mut wrong, mut inked) = (0, 0);
            let half = f64::from(px) / 2.0;
            let read = |fx: f64, fy: f64| {
                ground(
                    centre.x + (fx - half) / scale,
                    centre.y + (half - fy) / scale,
                )
            };
            for y in 0..px {
                for x in 0..px {
                    let (fx, fy) = (f64::from(x), f64::from(y));
                    let middle = read(fx + 0.5, fy + 0.5);
                    // Away from the outline's edge only: a pixel the edge crosses is smoothed.
                    if [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
                        .iter()
                        .any(|(dx, dy)| read(fx + dx, fy + dy) != middle)
                    {
                        continue;
                    }
                    let i = ((y * px + x) * 4) as usize;
                    let ink = pixels[i + 2] > 128 && pixels[i] < 128;
                    inked += usize::from(middle);
                    wrong += usize::from(ink != middle);
                }
            }
            eprintln!(
                "text marker case {case} ({scale:.1} px/m), {samples} samples: {wrong} of {inked} px the other way"
            );
            assert!(
                inked > 200,
                "case {case}: the letter is in view ({inked} px)"
            );
            // A bitmap magnified 5 times smooths its edges past the pixel next to them: 110 of 11 940 px the other way.
            assert!(
                wrong <= inked / 1000,
                "case {case} at {scale:.1} px/m, {samples} samples: {wrong} of {inked} px the other way"
            );
        }
    }
    // `KENTOS_TEXT_MARKS_OUT=<folder>`: the close view drawn both ways, for the eye (PPM files).
    if let Some(dir) = std::env::var_os("KENTOS_TEXT_MARKS_OUT") {
        let dir = std::path::PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        let mut bitmap_only = layer.clone();
        bitmap_only.batches.truncate(1);
        if let BatchKind::Marker { vectors, .. } = &mut bitmap_only.batches[0].kind {
            *vectors = false;
        }
        let camera = Camera {
            center: Vec2::new(edge, 0.0),
            width: f64::from(px),
            height: f64::from(px),
            scale: at * 20.0,
            ..Default::default()
        };
        for (name, l, id) in [
            ("once-bitmap", bitmap_only, 96_001u64),
            ("sonra-vektor", layer.clone(), 96_002),
        ] {
            gpu.styled = Some(StyledScene {
                layers: vec![Arc::new(StyledLayerPart::new(id, l))],
                ..Default::default()
            });
            let pixels = gpu.draw(&[], &camera, &settings, px, px);
            let mut ppm = format!("P6 {px} {px} 255\n").into_bytes();
            ppm.extend(pixels.chunks(4).flat_map(|p| [p[0], p[1], p[2]]));
            let _ = std::fs::write(dir.join(format!("{name}.ppm")), ppm);
        }
    }
    gpu.styled = previous;
}
