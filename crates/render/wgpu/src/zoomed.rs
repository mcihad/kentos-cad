//! The kept picture scaled while the wheel zooms (docs/adr/1068). While the view moves, a frame whose camera only
//! changed its scale (and its place) shows the last sharp picture of the same drawing scaled about the new camera,
//! sampled once per frame with a linear filter: one full-picture triangle, clipped to the pixels the old picture
//! covers. The ring a zoom out uncovers is drawn as the whole picture would draw it there (the strips of docs/adr/1067
//! §2). Once the view settles, its picture is drawn whole again: the settled frame is today's.
//!
//! The source is the last sharp picture, never a scaled one: the blur of a gesture does not add up frame by frame.
//! It is used for scales from a quarter to four times its own (two octaves); past that the frame is drawn whole and is
//! the new source. The ring a zoom out uncovers is taken, when it can be, from a wider sharp picture kept beside it
//! (`wide`: drawn at a smaller scale, it covers the whole new picture), scaled up as the source is; only when none
//! covers it are its strips drawn.

use bytemuck::{Pod, Zeroable};

use crate::targets::Place;

/// The scale range, new picture pixels a source pixel, within which the source is reused (§ module docs).
pub const MIN_RATIO: f64 = 0.25;
pub const MAX_RATIO: f64 = 4.0;
/// How much farther than the picture the wide picture may have been drawn: one source's range more, so that the
/// source a zoom in leaves behind (drawn just past [`MAX_RATIO`] farther) still fills a zoom out's ring.
pub const WIDE_RATIO: f64 = MAX_RATIO * MAX_RATIO / 2.0;

/// Where the source is sampled for a picture pixel: `uv = (position * scale + offset) / size`, `position` the
/// pixel's centre in picture pixels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ZoomUniform {
    pub scale: [f32; 2],
    pub offset: [f32; 2],
    pub size: [f32; 2],
    pub _pad: [f32; 2],
}

/// The scaled source's shader: a full-picture triangle sampling the source where the new pixel's centre was.
const ZOOMED_WGSL: &str = r"
struct Zoom {
    scale: vec2f,
    offset: vec2f,
    size: vec2f,
    pad: vec2f,
};
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> zoom: Zoom;

@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment fn fs(@builtin(position) at: vec4f) -> @location(0) vec4f {
    let uv = (at.xy * zoom.scale + zoom.offset) / zoom.size;
    return textureSampleLevel(source, source_sampler, uv, 0.0);
}
";

/// The shader's source, for the WGSL checks.
pub fn zoomed_source() -> &'static str {
    ZOOMED_WGSL
}

/// The bind group layout: the source picture, a linear sampler, the uniform.
pub(crate) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("kentos.cad2d.picture.zoomed"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<ZoomUniform>() as u64),
                },
                count: None,
            },
        ],
    })
}

/// The pipeline for pictures of `format` with `samples` per pixel: the triangle is drawn into the picture's
/// multisampled samples like the scene (every sample of a pixel takes the pixel's colour, so the resolve gives it
/// back unchanged).
pub(crate) fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("kentos.cad2d.picture.zoomed"),
        source: wgpu::ShaderSource::Wgsl(ZOOMED_WGSL.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("kentos.cad2d.picture.zoomed"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("kentos.cad2d.picture.zoomed"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState { count: samples, ..wgpu::MultisampleState::default() },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview: None,
        cache: None,
    })
}

/// A zoom frame's plan: the source's sampling, the picture pixels it covers (x0, y0, x1, y1, y down) and the strips
/// round them drawn afresh, or the wide picture's sampling under it all instead of the strips.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub uniform: ZoomUniform,
    pub inner: [u32; 4],
    pub strips: Vec<[u32; 4]>,
    pub wide: Option<ZoomUniform>,
}

impl Plan {
    /// The ring taken from the wide picture drawn at `wide`, when it covers the whole picture at `now`.
    pub fn with_wide(mut self, wide: &Place, now: &Place) -> Self {
        if !self.strips.is_empty()
            && let Some(u) = cover(wide, now)
        {
            self.wide = Some(u);
            self.strips.clear();
        }
        self
    }
}

/// The sampling of a picture drawn at `wide` that shows the whole picture at `now`: the same frame, drawn at a scale
/// no larger than `now`'s and no more than [`WIDE_RATIO`] smaller, covering every pixel.
pub fn cover(wide: &Place, now: &Place) -> Option<ZoomUniform> {
    if !wide.same_frame(now) || !(wide.px_per_unit > 0.0 && wide.px_per_unit <= now.px_per_unit) {
        return None;
    }
    let (uniform, inner) = map(wide, now, WIDE_RATIO)?;
    (inner == [0, 0, now.size[0], now.size[1]]).then_some(uniform)
}

/// Whether the wide picture drawn at `wide` is to stay beside a sharp picture drawn at `now`: it covers it.
pub fn keeps_wide(wide: &Place, now: &Place) -> bool {
    cover(wide, now).is_some()
}

/// The plan for showing a picture drawn at `source` as the picture at `now`; none when the two are not the same
/// frame but for the camera's centre and scale (origin, size, device pixel ratio, no turn), the scale did not change,
/// the scale changed past [`MIN_RATIO`]..[`MAX_RATIO`], or the source covers no pixel.
pub fn plan(source: &Place, now: &Place) -> Option<Plan> {
    if !source.same_frame(now) || source.px_per_unit.to_bits() == now.px_per_unit.to_bits() {
        return None;
    }
    let (ks, kn) = (source.px_per_unit, now.px_per_unit);
    if !(ks > 0.0 && kn > 0.0) {
        return None;
    }
    let (uniform, inner) = map(source, now, MAX_RATIO)?;
    let [x0, y0, x1, y1] = inner;
    let [w, h] = now.size;
    let mut strips = Vec::with_capacity(4);
    if x0 > 0 {
        strips.push([0, 0, x0, h]);
    }
    if x1 < w {
        strips.push([x1, 0, w, h]);
    }
    if y0 > 0 {
        strips.push([x0, 0, x1, y0]);
    }
    if y1 < h {
        strips.push([x0, y1, x1, h]);
    }
    Some(Plan { uniform, inner, strips, wide: None })
}

/// Where a picture drawn at `source` is sampled for the picture at `now`, and the pixels its samples cover; none when
/// the scale changed more than `most` times either way or the source covers no pixel.
fn map(source: &Place, now: &Place, most: f64) -> Option<(ZoomUniform, [u32; 4])> {
    let (ks, kn) = (source.px_per_unit, now.px_per_unit);
    // Source pixels a new pixel: below one when zooming in.
    let a = ks / kn;
    if !(1.0 / most..=most).contains(&(1.0 / a)) {
        return None;
    }
    let (w, h) = (f64::from(now.size[0]), f64::from(now.size[1]));
    // A new pixel's centre f maps to the source's f * a + b (x right, y down).
    let bx = w / 2.0 * (1.0 - a) + (now.center[0] - source.center[0]) * ks;
    let by = h / 2.0 * (1.0 - a) + (source.center[1] - now.center[1]) * ks;
    // The pixels whose centres fall in the source: f in [-b / a, (size - b) / a].
    let span = |b: f64, size: f64| {
        let (f0, f1) = (-b / a, (size - b) / a);
        let lo = (f0 - 0.5).ceil().clamp(0.0, size);
        let hi = ((f1 - 0.5).floor() + 1.0).clamp(0.0, size);
        (lo as u32, hi as u32)
    };
    let (x0, x1) = span(bx, w);
    let (y0, y1) = span(by, h);
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    let uniform = ZoomUniform {
        scale: [a as f32, a as f32],
        offset: [bx as f32, by as f32],
        size: [w as f32, h as f32],
        _pad: [0.0; 2],
    };
    Some((uniform, [x0, y0, x1, y1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64, k: f64) -> Place {
        Place { center: [x, y], origin: [486_000.0, 4_420_000.0], px_per_unit: k, size: [100, 80], twist: 0.0, dpr: 2.0 }
    }

    /// A pixel's centre lands on the source where the same world point was drawn.
    fn lands(p: &Plan, fx: f64, fy: f64) -> (f64, f64) {
        let u = p.uniform;
        (fx * f64::from(u.scale[0]) + f64::from(u.offset[0]), fy * f64::from(u.scale[1]) + f64::from(u.offset[1]))
    }

    /// Twice in about the centre: the middle half of the source fills the picture, no strip.
    #[test]
    fn zooming_in_samples_the_middle_and_leaves_no_strip() {
        let p = plan(&at(10.0, 5.0, 2.0), &at(10.0, 5.0, 4.0)).unwrap();
        assert_eq!(p.inner, [0, 0, 100, 80]);
        assert!(p.strips.is_empty());
        // The picture's centre is the source's; its corner the source's quarter point.
        assert_eq!(lands(&p, 50.0, 40.0), (50.0, 40.0));
        assert_eq!(lands(&p, 0.0, 0.0), (25.0, 20.0));
    }

    /// Half out about a point off the centre: the source covers the middle, the ring round it is drawn afresh, and
    /// a world point keeps its place on the source.
    #[test]
    fn zooming_out_leaves_a_ring_of_strips() {
        let source = at(10.0, 5.0, 2.0);
        // The camera 3 m east and 1 m south of the source's: world (13, 4) is now the picture's centre.
        let p = plan(&source, &at(13.0, 4.0, 1.0)).unwrap();
        let (sx, sy) = lands(&p, 50.0, 40.0);
        // On the source, (13, 4) was 3 m * 2 px right and 1 m * 2 px down of its centre.
        assert!((sx - 56.0).abs() < 1e-9 && (sy - 42.0).abs() < 1e-9, "{sx} {sy}");
        let [x0, y0, x1, y1] = p.inner;
        assert!(x0 > 0 && y0 > 0 && x1 < 100 && y1 < 80, "{:?}", p.inner);
        // The strips and the inner rectangle tile the picture, once each.
        let mut area = u64::from(x1 - x0) * u64::from(y1 - y0);
        for [a, b, c, d] in &p.strips {
            area += u64::from(c - a) * u64::from(d - b);
        }
        assert_eq!(area, 100 * 80);
        assert_eq!(p.strips.len(), 4);
    }

    /// No zoom, a zoom past four times either way, another size, origin, device pixel ratio or a turn: drawn whole.
    #[test]
    fn only_a_zoom_within_two_octaves_of_the_same_frame_is_scaled() {
        let s = at(10.0, 5.0, 2.0);
        assert!(plan(&s, &at(12.0, 5.0, 2.0)).is_none(), "a pan is not a zoom");
        assert!(plan(&s, &at(10.0, 5.0, 8.0)).is_some());
        assert!(plan(&s, &at(10.0, 5.0, 8.01)).is_none());
        assert!(plan(&s, &at(10.0, 5.0, 0.5)).is_some());
        assert!(plan(&s, &at(10.0, 5.0, 0.499)).is_none());
        assert!(plan(&s, &Place { size: [101, 80], ..at(10.0, 5.0, 3.0) }).is_none());
        assert!(plan(&s, &Place { origin: [0.0, 0.0], ..at(10.0, 5.0, 3.0) }).is_none());
        assert!(plan(&s, &Place { dpr: 1.0, ..at(10.0, 5.0, 3.0) }).is_none());
        assert!(plan(&s, &Place { twist: 0.1, ..at(10.0, 5.0, 3.0) }).is_none());
    }

    /// Zooming out of a picture drawn nearer: the wide picture drawn farther covers the ring, and is used instead of
    /// the strips; one that does not cover it (too narrow, panned away, too far) leaves the strips.
    #[test]
    fn a_wide_picture_fills_the_ring_a_zoom_out_uncovers() {
        let near = at(10.0, 5.0, 4.0);
        let now = at(10.0, 5.0, 3.0);
        let p = plan(&near, &now).unwrap();
        assert!(!p.strips.is_empty());
        let wide = at(10.0, 5.0, 1.0);
        let filled = p.clone().with_wide(&wide, &now);
        assert!(filled.strips.is_empty() && filled.wide.is_some());
        assert_eq!(filled.inner, p.inner);
        // The wide picture's middle third shows the picture: its centre stays, its corner a third in.
        let u = filled.wide.unwrap();
        let land = |fx: f64| fx * f64::from(u.scale[0]) + f64::from(u.offset[0]);
        assert!((land(50.0) - 50.0).abs() < 1e-6 && (land(0.0) - 100.0 / 3.0).abs() < 1e-4);
        for other in [at(10.0, 5.0, 3.5), at(60.0, 5.0, 1.0), at(10.0, 5.0, 0.37)] {
            assert_eq!(p.clone().with_wide(&other, &now), p, "{other:?}");
        }
        assert!(keeps_wide(&wide, &near) && !keeps_wide(&near, &wide));
        // The source a zoom in left behind, drawn just past four times farther, still covers.
        assert!(keeps_wide(&at(10.0, 5.0, 0.95), &near) && !keeps_wide(&at(10.0, 5.0, 0.49), &near));
    }
}
