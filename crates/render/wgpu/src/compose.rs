//! Katman çizimi and draw effects on the GPU (docs/adr/0226 §2–§4), with
//! the shared WGSL (`shaders/wgsl/compose`, `compose.layout.json`) the web's
//! WebGPU backend builds too.
//!
//! A layer whose opacity is below 1, whose mode is not normal or which a
//! mask cuts into is drawn into a picture of its own (`layer`), then laid
//! over the drawing so far, or mixed with it: the drawing is copied
//! (`backdrop`) and replaced with the mix. An effect group's batches (one
//! symbol layer with effects, over every object) are drawn into a picture
//! of their own (`group`) and laid over their target with their effects in
//! one pass, in the fixed order: shadow, outer glow, the group (blurred, with
//! Bulanıklaştır), inner shadow, inner glow. Each effect reads its channel of
//! a fused map of the group's alpha: the effects whose blurs are made at the
//! same level (whole, a half or a quarter for a large blur) share one
//! four-channel map, every pass doing each channel's own work (spread or
//! shrunk by a square filter, blurred by a Gaussian; two passes each, x then
//! y). A mask is the union of its sources' boxes (placed labels, drawn
//! markers), grown with round corners, drawn into a one-channel picture.
//! The passes into the drawing's picture run on in one render pass, resolved
//! only where the picture is read (a mode's backdrop) and at the end.
//!
//! Everything a frame needs is made in [`Renderer::prepare_styled`]: the
//! plan, the pictures (kept between frames at one size and sample count),
//! the passes' numbers in one buffer at dynamic offsets and their bind
//! groups; [`Renderer::render`] only records the passes. A drawing without
//! a composed layer or an effect group draws as it always did.
//!
//! [`Renderer::prepare_styled`]: crate::renderer::Renderer::prepare_styled
//! [`Renderer::render`]: crate::renderer::Renderer::render

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use kentos_native_style::batches::{Effect, Unit};

/// The composition shader.
pub const COMPOSE_WGSL: &str = include_str!("../../../../shaders/wgsl/compose/compose.wgsl");
/// Its binding and pipeline contract.
pub const COMPOSE_LAYOUT_JSON: &str = include_str!("../../../../shaders/wgsl/compose.layout.json");

/// Bytes of a pass's numbers (`Params`: eight `vec4f`); each takes a slot of the device's alignment.
const PARAMS_BYTES: u64 = 128;
/// A pass's numbers.
type Numbers = [f32; 32];
/// The pictures a pass reads, srcA to srcF.
type Reads = [Tex; 6];
/// Bytes of a mask box (`compose.layout.json` → `boxInstance`).
const BOX_BYTES: u64 = 28;
/// The largest blur (σ) and spread, device pixels (docs/adr/0226 §2).
const MAX_SIGMA: f64 = 64.0;
const MAX_SPREAD: f64 = 64.0;

/// A placed label as a mask takes it: its middle from the layers' origin
/// (metres), its width and height (CSS px) and its turn (degrees,
/// counter-clockwise).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelBox {
    pub centre: [f64; 2],
    pub size: [f32; 2],
    pub turn: f32,
}

/// One source of a layer's mask (docs/adr/0226 §3): another layer's drawn
/// markers (its scene layers) and its placed labels, grown by `grow` CSS px.
#[derive(Clone, Debug, PartialEq)]
pub struct MaskSource {
    pub markers: Option<Range<u32>>,
    pub labels: Arc<Vec<LabelBox>>,
    pub grow: f32,
}

/// How a layer meets the drawing (docs/adr/0226 §3, §4): its opacity, its
/// blending mode (`BlendMode::index`) and the masks cutting into it.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerComposeDraw {
    pub opacity: f32,
    pub blend: u32,
    pub masks: Vec<MaskSource>,
}

impl LayerComposeDraw {
    /// Into a picture's key: the same composition, the same pixels.
    pub(crate) fn hash_into(&self, h: &mut impl Hasher) {
        self.opacity.to_bits().hash(h);
        self.blend.hash(h);
        for m in &self.masks {
            m.markers.hash(h);
            m.grow.to_bits().hash(h);
            for b in m.labels.iter() {
                b.centre.map(f64::to_bits).hash(h);
                b.size.map(f32::to_bits).hash(h);
                b.turn.to_bits().hash(h);
            }
        }
    }
}

/// The pictures a pass reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Tex {
    /// A composed layer's picture, resolved.
    Layer,
    /// An effect group's picture, resolved.
    Group,
    Backdrop,
    Mask,
    /// A row-packed map of the group's alpha, four pixels of a row a texel: (level 1, 2 or 4, which).
    Packed(u8, u8),
    /// A column-packed one, four pixels of a column a texel.
    Column(u8, u8),
    /// A blurred group's picture, a pixel a texel: (level, which).
    Wide(u8, u8),
    /// A 1 × 1 stand-in for an unused binding.
    None,
}

/// What a map pass runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapPipe {
    /// The group's alpha column-packed.
    Pack,
    /// A column-packed map row-packed, and back.
    Rows,
    Columns,
    /// A column-packed map's filter along x, a row-packed one's along y; a row-packed one's half.
    H,
    V,
    Down,
    /// A blurred picture's half and blur.
    DownWide,
    BlurWide,
}

/// Where styled batches or a layer go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Into {
    Picture,
    Layer,
}

/// One pass of a composed frame, in order.
#[derive(Clone, Debug)]
pub(crate) enum Pass {
    /// The styled batches of a stretch of the draw order into the picture or the layer.
    Batches { seq: Range<usize>, into: Into },
    /// The layer's picture cleared.
    LayerClear,
    /// An effect group's batches into its picture, cleared first.
    Group { seq: Range<usize> },
    /// A map made: the pipeline, the picture written, the pass's numbers and bindings.
    Map {
        pipe: MapPipe,
        target: Tex,
        bind: usize,
        offset: u32,
    },
    /// An effect group's effects and picture over its target.
    Effect {
        into: Into,
        bind: usize,
        offset: u32,
    },
    /// The mask cleared and its boxes drawn.
    Boxes {
        range: Range<u32>,
        bind: usize,
        offset: u32,
    },
    /// The drawing copied for a mode's mix.
    Backdrop,
    /// The layer laid over the drawing (`blend` false) or mixed with it.
    Layer {
        blend: bool,
        bind: usize,
        offset: u32,
    },
}

/// The pipelines of the picture's sample count.
struct PicturePipes {
    over: wgpu::RenderPipeline,
    blend: wgpu::RenderPipeline,
    effect: wgpu::RenderPipeline,
}

/// The composition pipelines and layout of one device.
pub struct Composer {
    format: wgpu::TextureFormat,
    module: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    picture: HashMap<u32, PicturePipes>,
    pack: wgpu::RenderPipeline,
    rows: wgpu::RenderPipeline,
    columns: wgpu::RenderPipeline,
    h: wgpu::RenderPipeline,
    v: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    down_wide: wgpu::RenderPipeline,
    blur_wide: wgpu::RenderPipeline,
    boxes: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    none: wgpu::TextureView,
    /// Slots of the numbers' buffer, the device's dynamic offset alignment.
    slot: u64,
}

const PREMULTIPLIED: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

const LARGEST: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Max,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Max,
    },
};

impl std::fmt::Debug for Composer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Composer")
            .field("format", &self.format)
            .field("samples", &self.picture.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl Composer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Composer {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.compose"),
            source: wgpu::ShaderSource::Wgsl(COMPOSE_WGSL.into()),
        });
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.compose"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(PARAMS_BYTES),
                    },
                    count: None,
                },
                texture(1),
                texture(2),
                texture(3),
                texture(4),
                texture(5),
                texture(6),
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.compose"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let map = |label: &'static str, fragment: &'static str| {
            pipeline(
                device,
                &module,
                &pipeline_layout,
                label,
                ("fullVs", fragment),
                wgpu::TextureFormat::Rgba8Unorm,
                None,
                1,
                &[],
            )
        };
        let box_attributes =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32];
        let boxes = pipeline(
            device,
            &module,
            &pipeline_layout,
            "kentos.compose.box",
            ("boxVs", "boxFs"),
            wgpu::TextureFormat::R8Unorm,
            Some(LARGEST),
            1,
            &[wgpu::VertexBufferLayout {
                array_stride: BOX_BYTES,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &box_attributes,
            }],
        );
        let none = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("kentos.compose.none"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let min_align = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        Composer {
            pack: map("kentos.compose.pack", "packFs"),
            rows: map("kentos.compose.rows", "rowsFs"),
            columns: map("kentos.compose.columns", "columnsFs"),
            h: map("kentos.compose.h", "hFs"),
            v: map("kentos.compose.v", "vFs"),
            down: map("kentos.compose.down", "downFs"),
            down_wide: map("kentos.compose.downWide", "downWideFs"),
            blur_wide: map("kentos.compose.blurWide", "blurWideFs"),
            boxes,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("kentos.compose"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..wgpu::SamplerDescriptor::default()
            }),
            none,
            slot: PARAMS_BYTES.next_multiple_of(min_align.max(PARAMS_BYTES)),
            format,
            module,
            layout,
            pipeline_layout,
            picture: HashMap::new(),
        }
    }

    /// The picture's pipelines for `samples` per pixel, made the first time that count composes.
    pub fn ensure(&mut self, device: &wgpu::Device, samples: u32) {
        if self.picture.contains_key(&samples) {
            return;
        }
        let make = |label, fragment, blend| {
            pipeline(
                device,
                &self.module,
                &self.pipeline_layout,
                label,
                ("fullVs", fragment),
                self.format,
                blend,
                samples,
                &[],
            )
        };
        let pipes = PicturePipes {
            over: make("kentos.compose.over", "overFs", Some(PREMULTIPLIED)),
            blend: make("kentos.compose.blend", "blendFs", None),
            effect: make("kentos.compose.effect", "effectFs", Some(PREMULTIPLIED)),
        };
        self.picture.insert(samples, pipes);
    }

    fn map_pipeline(&self, pipe: MapPipe) -> &wgpu::RenderPipeline {
        match pipe {
            MapPipe::Pack => &self.pack,
            MapPipe::Rows => &self.rows,
            MapPipe::Columns => &self.columns,
            MapPipe::H => &self.h,
            MapPipe::V => &self.v,
            MapPipe::Down => &self.down,
            MapPipe::DownWide => &self.down_wide,
            MapPipe::BlurWide => &self.blur_wide,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    label: &'static str,
    (vertex, fragment): (&'static str, &'static str),
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
    samples: u32,
    buffers: &[wgpu::VertexBufferLayout<'_>],
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some(vertex),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: samples,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fragment),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

/// A picture to draw into: multisampled resolving into one sample, or one sample only.
struct Drawn {
    msaa: Option<wgpu::TextureView>,
    resolve: wgpu::TextureView,
}

impl Drawn {
    fn new(
        device: &wgpu::Device,
        label: &'static str,
        format: wgpu::TextureFormat,
        size: [u32; 2],
        samples: u32,
    ) -> Drawn {
        let make = |samples: u32, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        Drawn {
            msaa: (samples > 1).then(|| make(samples, wgpu::TextureUsages::RENDER_ATTACHMENT)),
            resolve: make(
                1,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
        }
    }

    /// The attachment a pass draws through: cleared to transparent, or loaded.
    fn attachment(&self, clear: bool) -> wgpu::RenderPassColorAttachment<'_> {
        let load = if clear {
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
        } else {
            wgpu::LoadOp::Load
        };
        match &self.msaa {
            Some(msaa) => wgpu::RenderPassColorAttachment {
                view: msaa,
                depth_slice: None,
                resolve_target: Some(&self.resolve),
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            },
            None => wgpu::RenderPassColorAttachment {
                view: &self.resolve,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            },
        }
    }
}

/// A one-sample picture the maps and the mask are drawn into.
fn plain(
    device: &wgpu::Device,
    label: &'static str,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    usage: wgpu::TextureUsages,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0].max(1),
            height: size[1].max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: usage | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// A view's composing pictures for one size and sample count, made as a frame first needs each.
pub(crate) struct ComposeTargets {
    pub size: [u32; 2],
    pub samples: u32,
    format: wgpu::TextureFormat,
    layer: Option<Drawn>,
    group: Option<Drawn>,
    backdrop: Option<(wgpu::Texture, wgpu::TextureView)>,
    mask: Option<(wgpu::Texture, wgpu::TextureView)>,
    /// The maps by (form: 0 a pixel a texel, 1 row-packed, 2 column-packed; level; which).
    maps: HashMap<(u8, u8, u8), (wgpu::Texture, wgpu::TextureView)>,
    /// The passes' numbers and the masks' boxes, grown as needed.
    params: Option<(wgpu::Buffer, u64)>,
    boxes: Option<(wgpu::Buffer, u64)>,
    /// Bytes of the pictures, for the frame's statistics.
    pub bytes: u64,
}

impl ComposeTargets {
    pub fn new(format: wgpu::TextureFormat, size: [u32; 2], samples: u32) -> ComposeTargets {
        ComposeTargets {
            size,
            samples,
            format,
            layer: None,
            group: None,
            backdrop: None,
            mask: None,
            maps: HashMap::new(),
            params: None,
            boxes: None,
            bytes: 0,
        }
    }

    fn texel(&self) -> u64 {
        u64::from(self.format.block_copy_size(None).unwrap_or(4))
    }

    fn pixels(&self) -> u64 {
        u64::from(self.size[0]) * u64::from(self.size[1])
    }

    /// The size of a map of `level` in pixels: the picture's, halved or quartered, rounded up.
    pub fn level_size(&self, level: u8) -> [u32; 2] {
        let k = u32::from(level.max(1));
        [self.size[0].div_ceil(k), self.size[1].div_ceil(k)]
    }

    /// A row-packed map's size in texels: four pixels of a row a texel.
    pub fn packed_size(&self, level: u8) -> [u32; 2] {
        let [w, h] = self.level_size(level);
        [w.div_ceil(4), h]
    }

    /// A column-packed map's size in texels: four pixels of a column a texel.
    pub fn column_size(&self, level: u8) -> [u32; 2] {
        let [w, h] = self.level_size(level);
        [w, h.div_ceil(4)]
    }

    fn need(&mut self, device: &wgpu::Device, tex: Tex) {
        let (size, samples, format) = (self.size, self.samples, self.format);
        match tex {
            Tex::Layer if self.layer.is_none() => {
                self.layer = Some(Drawn::new(
                    device,
                    "kentos.compose.layer",
                    format,
                    size,
                    samples,
                ));
                self.bytes +=
                    self.pixels() * self.texel() * (u64::from(samples) + u64::from(samples > 1));
            }
            Tex::Group if self.group.is_none() => {
                self.group = Some(Drawn::new(
                    device,
                    "kentos.compose.group",
                    format,
                    size,
                    samples,
                ));
                self.bytes +=
                    self.pixels() * self.texel() * (u64::from(samples) + u64::from(samples > 1));
            }
            Tex::Backdrop if self.backdrop.is_none() => {
                self.backdrop = Some(plain(
                    device,
                    "kentos.compose.backdrop",
                    format,
                    size,
                    wgpu::TextureUsages::COPY_DST,
                ));
                self.bytes += self.pixels() * self.texel();
            }
            Tex::Mask if self.mask.is_none() => {
                self.mask = Some(plain(
                    device,
                    "kentos.compose.mask",
                    wgpu::TextureFormat::R8Unorm,
                    size,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                ));
                self.bytes += self.pixels();
            }
            Tex::Packed(level, which) | Tex::Column(level, which) | Tex::Wide(level, which) => {
                let form = match tex {
                    Tex::Packed(..) => 1u8,
                    Tex::Column(..) => 2,
                    _ => 0,
                };
                if self.maps.contains_key(&(form, level, which)) {
                    return;
                }
                let s = match form {
                    1 => self.packed_size(level),
                    2 => self.column_size(level),
                    _ => self.level_size(level),
                };
                let made = plain(
                    device,
                    "kentos.compose.map",
                    wgpu::TextureFormat::Rgba8Unorm,
                    s,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                );
                self.bytes += u64::from(s[0]) * u64::from(s[1]) * 4;
                self.maps.insert((form, level, which), made);
            }
            _ => {}
        }
    }

    /// A picture's view to read; the stand-in when it is not made.
    fn view<'a>(&'a self, tex: Tex, none: &'a wgpu::TextureView) -> &'a wgpu::TextureView {
        match tex {
            Tex::Layer => self.layer.as_ref().map_or(none, |d| &d.resolve),
            Tex::Group => self.group.as_ref().map_or(none, |d| &d.resolve),
            Tex::Backdrop => self.backdrop.as_ref().map_or(none, |(_, v)| v),
            Tex::Mask => self.mask.as_ref().map_or(none, |(_, v)| v),
            Tex::Wide(l, w) => self.maps.get(&(0, l, w)).map_or(none, |(_, v)| v),
            Tex::Packed(l, w) => self.maps.get(&(1, l, w)).map_or(none, |(_, v)| v),
            Tex::Column(l, w) => self.maps.get(&(2, l, w)).map_or(none, |(_, v)| v),
            Tex::None => none,
        }
    }
}

/// A frame's composing work, made in prepare and recorded in render.
pub(crate) struct ComposeFrame {
    pub passes: Vec<Pass>,
    binds: Vec<wgpu::BindGroup>,
}

/// The level a blur of σ device pixels is made at: whole up to 8, half up to 32, a quarter beyond.
pub fn level_of(sigma: f64) -> u8 {
    if sigma <= 8.0 {
        1
    } else if sigma <= 32.0 {
        2
    } else {
        4
    }
}

/// The Gaussian weights' sum for σ over ±⌈3σ⌉ (the blur passes divide by it).
pub fn gauss_sum(sigma: f64) -> f64 {
    if sigma <= 0.0 {
        return 1.0;
    }
    let r = (3.0 * sigma).ceil() as i64;
    (-r..=r)
        .map(|k| (-(k * k) as f64 / (2.0 * sigma * sigma)).exp())
        .sum()
}

/// One of an effect group's alpha maps (docs/adr/0226 §2): its kind (0 the
/// shadow, 1 the outer glow, 2 the inner shadow, 3 the inner glow), its square
/// filter's n (+ spread, − shrink; 0 none), its blur's σ and the level the
/// blur is made at, device pixels.
#[derive(Clone, Copy, Debug)]
struct Chain {
    kind: usize,
    n: f64,
    sigma: f64,
    level: u8,
}

impl Chain {
    fn new(kind: usize, spread: f64, sigma: f64) -> Chain {
        let n = spread.abs().min(MAX_SPREAD).round();
        let sigma = sigma.min(MAX_SIGMA);
        Chain {
            kind,
            n: if n >= 1.0 { n.copysign(spread) } else { 0.0 },
            sigma,
            level: level_of(sigma),
        }
    }
}

/// A map pass's numbers: its work (2 a blur, 3 the largest, 4 the smallest),
/// σ or n, a blur's weights' sum (for σ as the shader takes it, in single
/// precision); the map's size in pixels, its source's in texels.
fn work(op: f32, value: f64, size: [u32; 2], source: [u32; 2]) -> Numbers {
    let value = value as f32;
    let sum = if op == 2.0 {
        gauss_sum(f64::from(value)) as f32
    } else {
        1.0
    };
    let mut out = [0.0f32; 32];
    out[..4].copy_from_slice(&[op, value, sum, 0.0]);
    out[4..8].copy_from_slice(&[
        size[0] as f32,
        size[1] as f32,
        source[0] as f32,
        source[1] as f32,
    ]);
    out
}

/// Builds a frame's passes: the numbers, the boxes and the bind groups.
pub(crate) struct Planner<'a> {
    composer: &'a Composer,
    targets: &'a mut ComposeTargets,
    device: &'a wgpu::Device,
    params: Vec<Numbers>,
    boxes: Vec<[f32; 7]>,
    passes: Vec<Pass>,
    /// Each bind group's pictures; the groups are made at the end.
    reads: Vec<Reads>,
    /// Device pixels per metre and per CSS pixel.
    px_per_m: f64,
    dpr: f64,
    /// Whether this group's alpha is row-packed already.
    alpha_rows: bool,
}

impl<'a> Planner<'a> {
    pub fn new(
        composer: &'a Composer,
        targets: &'a mut ComposeTargets,
        device: &'a wgpu::Device,
        px_per_m: f64,
        dpr: f64,
    ) -> Planner<'a> {
        Planner {
            composer,
            targets,
            device,
            params: Vec::new(),
            boxes: Vec::new(),
            passes: Vec::new(),
            reads: Vec::new(),
            px_per_m,
            dpr,
            alpha_rows: false,
        }
    }

    /// A pass's numbers and pictures: its offset and bind group.
    fn slot(&mut self, numbers: Numbers, reads: Reads) -> (usize, u32) {
        let offset = (self.params.len() as u64 * self.composer.slot) as u32;
        self.params.push(numbers);
        for r in reads {
            self.targets.need(self.device, r);
        }
        self.reads.push(reads);
        (self.reads.len() - 1, offset)
    }

    pub fn batches(&mut self, seq: Range<usize>, into: Into) {
        if seq.is_empty() {
            return;
        }
        if into == Into::Layer {
            self.targets.need(self.device, Tex::Layer);
        }
        // Next to the last stretch into the same place: one stretch.
        if let Some(Pass::Batches {
            seq: last,
            into: was,
        }) = self.passes.last_mut()
            && *was == into
            && last.end == seq.start
        {
            last.end = seq.end;
            return;
        }
        self.passes.push(Pass::Batches { seq, into });
    }

    pub fn layer_clear(&mut self) {
        self.targets.need(self.device, Tex::Layer);
        self.passes.push(Pass::LayerClear);
    }

    /// Lengths in the effect's unit as device pixels.
    fn px(&self, unit: Unit, v: f64) -> f64 {
        let k = if unit == Unit::World {
            self.px_per_m
        } else {
            self.dpr
        };
        (v * k).max(0.0)
    }

    fn map(&mut self, pipe: MapPipe, from: Tex, to: Tex, numbers: Numbers) {
        self.targets.need(self.device, to);
        let (bind, offset) = self.slot(
            numbers,
            [from, Tex::None, Tex::None, Tex::None, Tex::None, Tex::None],
        );
        self.passes.push(Pass::Map {
            pipe,
            target: to,
            bind,
            offset,
        });
    }

    /// A column-packed map's filter along x, row-packed, along y into `to`, at
    /// `level`; between them the level's scratch maps `scratch` (column- and
    /// row-packed).
    fn filtered(&mut self, from: Tex, scratch: u8, to: Tex, (op, value): (f32, f64), level: u8) {
        let (between, rows) = (Tex::Column(level, scratch), Tex::Packed(level, scratch));
        let size = self.targets.level_size(level);
        let (columns, packed) = (
            self.targets.column_size(level),
            self.targets.packed_size(level),
        );
        self.map(MapPipe::H, from, between, work(op, value, size, columns));
        self.map(MapPipe::Rows, between, rows, work(0.0, 0.0, size, columns));
        self.map(MapPipe::V, rows, to, work(op, value, size, packed));
    }

    /// The group's alpha row-packed, made once a group from its column-packed form.
    fn alpha_rows(&mut self) -> Tex {
        let rows = Tex::Packed(1, 0);
        if !self.alpha_rows {
            let size = self.targets.level_size(1);
            let columns = self.targets.column_size(1);
            self.map(
                MapPipe::Rows,
                Tex::Column(1, 0),
                rows,
                work(0.0, 0.0, size, columns),
            );
            self.alpha_rows = true;
        }
        rows
    }

    /// An effect's row-packed map of the group's alpha (column-packed whole,
    /// `Column(1, 0)`): its square filter whole, the map halved to its level,
    /// its blur there. Returns the map: the effect's own (`kind`), or the alpha
    /// itself.
    fn chain(&mut self, c: &Chain) -> Tex {
        let own = |level: u8| Tex::Packed(level, 4 + c.kind as u8);
        let mut rows = None;
        if c.n != 0.0 {
            let op = if c.n > 0.0 { 3.0 } else { 4.0 };
            let to = if c.sigma > 0.0 {
                Tex::Packed(1, 2)
            } else {
                own(1)
            };
            self.filtered(Tex::Column(1, 0), 1, to, (op, c.n.abs()), 1);
            if c.sigma <= 0.0 {
                return to;
            }
            rows = Some(to);
        } else if c.sigma <= 0.0 {
            return self.alpha_rows();
        }
        let level = c.level;
        // The blur's source, column-packed at its level.
        let columns = if level > 1 {
            let mut from = match rows {
                Some(r) => r,
                None => self.alpha_rows(),
            };
            let mut at = 1u8;
            while at < level {
                let next = at * 2;
                let numbers = work(
                    0.0,
                    0.0,
                    self.targets.level_size(next),
                    self.targets.packed_size(at),
                );
                self.map(MapPipe::Down, from, Tex::Packed(next, 1), numbers);
                from = Tex::Packed(next, 1);
                at = next;
            }
            let numbers = work(
                0.0,
                0.0,
                self.targets.level_size(level),
                self.targets.packed_size(level),
            );
            self.map(MapPipe::Columns, from, Tex::Column(level, 1), numbers);
            Tex::Column(level, 1)
        } else if let Some(r) = rows {
            let numbers = work(
                0.0,
                0.0,
                self.targets.level_size(1),
                self.targets.packed_size(1),
            );
            self.map(MapPipe::Columns, r, Tex::Column(1, 2), numbers);
            Tex::Column(1, 2)
        } else {
            Tex::Column(1, 0)
        };
        let s = c.sigma / f64::from(level);
        self.filtered(columns, 3, own(level), (2.0, s), level);
        own(level)
    }

    /// The group's picture blurred by σ at its level (Bulanıklaştır), four channels a pixel. Returns the map and its level.
    fn blurred(&mut self, sigma: f64) -> (Tex, u8) {
        let sigma = sigma.min(MAX_SIGMA);
        let level = level_of(sigma);
        let mut from = Tex::Group;
        let mut at = 1u8;
        while at < level {
            let next = at * 2;
            let numbers = work(
                0.0,
                0.0,
                self.targets.level_size(next),
                self.targets.level_size(at),
            );
            self.map(MapPipe::DownWide, from, Tex::Wide(next, 0), numbers);
            from = Tex::Wide(next, 0);
            at = next;
        }
        let s = (sigma / f64::from(level)) as f32;
        let size = self.targets.level_size(level);
        let along = |dir: [f32; 2]| {
            let mut out = [0.0f32; 32];
            out[..4].copy_from_slice(&[0.0, s, gauss_sum(f64::from(s)) as f32, 0.0]);
            out[4..8].copy_from_slice(&[dir[0], dir[1], size[0] as f32, size[1] as f32]);
            out
        };
        self.map(
            MapPipe::BlurWide,
            from,
            Tex::Wide(level, 1),
            along([1.0, 0.0]),
        );
        self.map(
            MapPipe::BlurWide,
            Tex::Wide(level, 1),
            Tex::Wide(level, 2),
            along([0.0, 1.0]),
        );
        (Tex::Wide(level, 2), level)
    }

    /// The scale from the target's uv to a map's of `level`.
    fn map_scale(&self, level: u8) -> [f32; 2] {
        let s = self.targets.level_size(level);
        let k = f32::from(level);
        [
            self.targets.size[0] as f32 / (k * s[0] as f32),
            self.targets.size[1] as f32 / (k * s[1] as f32),
        ]
    }

    /// An effect group's batches and its effects over `into` (docs/adr/0226 §2).
    pub fn group(&mut self, seq: Range<usize>, effect: &Effect, into: Into) {
        self.targets.need(self.device, Tex::Group);
        if into == Into::Layer {
            self.targets.need(self.device, Tex::Layer);
        }
        self.passes.push(Pass::Group { seq });
        let unit = effect.unit;
        // A shadow's offset in device pixels: x right, y down (the effect's y is up).
        let shift = |p: &Self, dx: f64, dy: f64| {
            let k = if unit == Unit::World {
                p.px_per_m
            } else {
                p.dpr
            };
            [(dx * k) as f32, (-dy * k) as f32]
        };
        let mut numbers = [0.0f32; 32];
        let colour = |numbers: &mut Numbers, at: usize, c: [f64; 4]| {
            numbers[at..at + 4].copy_from_slice(&c.map(|v| v as f32));
        };
        let mut chains = Vec::new();
        if let Some(s) = &effect.shadow {
            chains.push(Chain::new(0, 0.0, self.px(unit, s.blur) / 3.0));
            numbers[..2].copy_from_slice(&shift(self, s.dx, s.dy));
            colour(&mut numbers, 8, s.color);
        }
        if let Some(g) = &effect.glow {
            chains.push(Chain::new(
                1,
                self.px(unit, g.spread),
                self.px(unit, g.blur) / 3.0,
            ));
            colour(&mut numbers, 12, g.color);
        }
        if let Some(s) = &effect.inner_shadow {
            chains.push(Chain::new(2, 0.0, self.px(unit, s.blur) / 3.0));
            numbers[2..4].copy_from_slice(&shift(self, s.dx, s.dy));
            colour(&mut numbers, 16, s.color);
        }
        if let Some(g) = &effect.inner_glow {
            chains.push(Chain::new(
                3,
                -self.px(unit, g.spread),
                self.px(unit, g.blur) / 3.0,
            ));
            colour(&mut numbers, 20, g.color);
        }
        // The group's alpha column-packed once; each effect's map from it, read from srcB … srcE.
        let mut reads = [
            Tex::Group,
            Tex::None,
            Tex::None,
            Tex::None,
            Tex::None,
            Tex::None,
        ];
        if !chains.is_empty() {
            let size = self.targets.level_size(1);
            self.map(
                MapPipe::Pack,
                Tex::Group,
                Tex::Column(1, 0),
                work(0.0, 0.0, size, size),
            );
            self.alpha_rows = false;
            for c in &chains {
                let map = self.chain(c);
                reads[1 + c.kind] = map;
                // A map without a blur is whole.
                numbers[4 + c.kind] = if c.sigma > 0.0 {
                    f32::from(c.level)
                } else {
                    1.0
                };
            }
        }
        if effect.blur > 0.0 && self.px(unit, effect.blur) >= 0.5 {
            let (map, level) = self.blurred(self.px(unit, effect.blur) / 3.0);
            reads[5] = map;
            let scale = self.map_scale(level);
            numbers[24..28].copy_from_slice(&[f32::from(level), 0.0, scale[0], scale[1]]);
        }
        numbers[28..30]
            .copy_from_slice(&[self.targets.size[0] as f32, self.targets.size[1] as f32]);
        let (bind, offset) = self.slot(numbers, reads);
        self.passes.push(Pass::Effect { into, bind, offset });
    }

    /// The mask's boxes, already in the target's pixels.
    pub fn boxes(&mut self, boxes: &[[f32; 7]]) -> Range<u32> {
        let start = self.boxes.len() as u32;
        self.boxes.extend_from_slice(boxes);
        start..self.boxes.len() as u32
    }

    /// The layer's picture over the drawing: faded, masked by `mask` (a box range), mixed by `blend`.
    pub fn layer(&mut self, opacity: f32, blend: u32, mask: Option<Range<u32>>) {
        let masked = mask.as_ref().is_some_and(|r| !r.is_empty());
        if let Some(range) = mask.filter(|r| !r.is_empty()) {
            let size = self.targets.size;
            // The boxes draw into the mask: it is their target, not a picture they read.
            self.targets.need(self.device, Tex::Mask);
            let (bind, offset) = self.slot(
                numbers([size[0] as f32, size[1] as f32, 0.0, 0.0]),
                [Tex::None; 6],
            );
            self.passes.push(Pass::Boxes {
                range,
                bind,
                offset,
            });
        }
        let mixed = blend != 0;
        if mixed {
            self.targets.need(self.device, Tex::Backdrop);
            self.passes.push(Pass::Backdrop);
        }
        let (bind, offset) = self.slot(
            numbers([opacity, if masked { 1.0 } else { 0.0 }, blend as f32, 0.0]),
            [
                Tex::Layer,
                if mixed { Tex::Backdrop } else { Tex::None },
                if masked { Tex::Mask } else { Tex::None },
                Tex::None,
                Tex::None,
                Tex::None,
            ],
        );
        self.passes.push(Pass::Layer {
            blend: mixed,
            bind,
            offset,
        });
    }

    /// The frame: the numbers and boxes written, the bind groups made.
    pub fn finish(self, queue: &wgpu::Queue) -> ComposeFrame {
        let Planner {
            composer,
            targets,
            device,
            params,
            boxes,
            passes,
            reads,
            ..
        } = self;
        let slot = composer.slot;
        let need = (params.len().max(1) as u64) * slot;
        if targets.params.as_ref().is_none_or(|(_, size)| *size < need) {
            let size = need.next_power_of_two();
            targets.params = Some((
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("kentos.compose.params"),
                    size,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                size,
            ));
        }
        let mut bytes = vec![0u8; need as usize];
        for (i, p) in params.iter().enumerate() {
            let at = i * slot as usize;
            bytes[at..at + PARAMS_BYTES as usize].copy_from_slice(bytemuck::cast_slice(p));
        }
        if let Some((buffer, _)) = &targets.params {
            queue.write_buffer(buffer, 0, &bytes);
        }
        if !boxes.is_empty() {
            let need = boxes.len() as u64 * BOX_BYTES;
            if targets.boxes.as_ref().is_none_or(|(_, size)| *size < need) {
                let size = need.next_power_of_two();
                targets.boxes = Some((
                    device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("kentos.compose.boxes"),
                        size,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    size,
                ));
            }
            if let Some((buffer, _)) = &targets.boxes {
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(&boxes));
            }
        }
        let binds = reads
            .iter()
            .filter_map(|r| {
                let (buffer, _) = targets.params.as_ref()?;
                let view = |t: Tex| targets.view(t, &composer.none);
                let texture = |binding: u32, t: Tex| wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::TextureView(view(t)),
                };
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kentos.compose"),
                    layout: &composer.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer,
                                offset: 0,
                                size: wgpu::BufferSize::new(PARAMS_BYTES),
                            }),
                        },
                        texture(1, r[0]),
                        texture(2, r[1]),
                        texture(3, r[2]),
                        texture(4, r[3]),
                        texture(5, r[4]),
                        texture(6, r[5]),
                        wgpu::BindGroupEntry {
                            binding: 7,
                            resource: wgpu::BindingResource::Sampler(&composer.sampler),
                        },
                    ],
                }))
            })
            .collect();
        ComposeFrame { passes, binds }
    }
}

/// A pass's numbers with only its first `vec4f`.
fn numbers(a: [f32; 4]) -> Numbers {
    let mut out = [0.0f32; 32];
    out[..4].copy_from_slice(&a);
    out
}

/// What recording a composed frame needs of the renderer: the plain styled
/// batches of a stretch drawn into a pass.
pub(crate) trait DrawBatches {
    fn draw_batches(&self, pass: &mut wgpu::RenderPass<'_>, seq: Range<usize>);
}

/// A pass over the drawing's picture, as the renderer opens it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PicturePass {
    /// The frame's first: cleared, the ground drawn first.
    pub first: bool,
    /// The frame's last: the highlights drawn last.
    pub last: bool,
    /// Resolved at its end (multisampled): the picture is read next, or it is the last.
    pub resolve: bool,
}

/// A one-sample picture's attachment, cleared to transparent (a map, the mask).
fn cleared(view: &wgpu::TextureView) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    }
}

/// How the renderer opens a pass over the drawing's picture and lets a run record into it.
pub(crate) type PictureFn<'a> =
    dyn Fn(&mut wgpu::CommandEncoder, PicturePass, &mut dyn FnMut(&mut wgpu::RenderPass<'_>)) + 'a;

/// Where a pass draws: the picture or the layer (whose run a clear starts), or a picture of its own.
fn drawn_into(p: &Pass) -> Option<Into> {
    match p {
        Pass::Batches { into, .. } | Pass::Effect { into, .. } => Some(*into),
        Pass::Layer { .. } => Some(Into::Picture),
        Pass::LayerClear => Some(Into::Layer),
        _ => None,
    }
}

impl Composer {
    /// Records a composed frame. The passes into the drawing's picture run on in
    /// one render pass the renderer opens (`picture`: cleared with the ground
    /// first, the highlights last, resolved where the picture is copied next and
    /// at the end); so do a layer's, from its clear. `copy_picture` copies the
    /// drawing into the backdrop.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &ComposeTargets,
        frame: &ComposeFrame,
        samples: u32,
        batches: &dyn DrawBatches,
        picture: &PictureFn<'_>,
        copy_picture: &dyn Fn(&mut wgpu::CommandEncoder, &wgpu::Texture),
    ) {
        let Some(pipes) = self.picture.get(&samples) else {
            return;
        };
        let full = |pass: &mut wgpu::RenderPass<'_>,
                    pipeline: &wgpu::RenderPipeline,
                    bind: usize,
                    offset: u32| {
            if let Some(b) = frame.binds.get(bind) {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, b, &[offset]);
                pass.draw(0..3, 0..1);
            }
        };
        // A pass of a run into the picture or the layer.
        let step = |pass: &mut wgpu::RenderPass<'_>, p: &Pass| match p {
            Pass::Batches { seq, .. } => batches.draw_batches(pass, seq.clone()),
            Pass::Effect { bind, offset, .. } => full(pass, &pipes.effect, *bind, *offset),
            Pass::Layer {
                blend,
                bind,
                offset,
            } => {
                full(
                    pass,
                    if *blend { &pipes.blend } else { &pipes.over },
                    *bind,
                    *offset,
                );
            }
            _ => {}
        };
        let passes = &frame.passes;
        let mut first = true;
        let mut i = 0;
        loop {
            let next = passes.get(i);
            match next.and_then(drawn_into) {
                Some(Into::Picture) => {
                    // The picture's run: on until a pass of another place.
                    let start = i;
                    while i < passes.len() && drawn_into(&passes[i]) == Some(Into::Picture) {
                        i += 1;
                    }
                    let last = passes[i..]
                        .iter()
                        .all(|p| drawn_into(p) != Some(Into::Picture));
                    // Resolved when the drawing is copied before the picture's next run.
                    let copied = passes[i..]
                        .iter()
                        .take_while(|p| drawn_into(p) != Some(Into::Picture))
                        .any(|p| matches!(p, Pass::Backdrop));
                    let resolve = last || copied;
                    let run = &passes[start..i];
                    picture(
                        encoder,
                        PicturePass {
                            first,
                            last,
                            resolve,
                        },
                        &mut |pass| {
                            for p in run {
                                step(pass, p);
                            }
                        },
                    );
                    first = false;
                }
                Some(Into::Layer) => {
                    // The layer's run: cleared when it starts with its clear, on while it draws into the layer.
                    let clear = matches!(passes[i], Pass::LayerClear);
                    let start = if clear { i + 1 } else { i };
                    i = start;
                    while i < passes.len()
                        && drawn_into(&passes[i]) == Some(Into::Layer)
                        && !matches!(passes[i], Pass::LayerClear)
                    {
                        i += 1;
                    }
                    if let Some(layer) = &targets.layer {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("kentos.compose.layer"),
                            color_attachments: &[Some(layer.attachment(clear))],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        for p in &passes[start..i] {
                            step(&mut pass, p);
                        }
                    }
                }
                None => match next {
                    None => {
                        // A frame that never drew into the picture still has its ground and highlights.
                        if first {
                            let all = PicturePass {
                                first: true,
                                last: true,
                                resolve: true,
                            };
                            picture(encoder, all, &mut |_| {});
                        }
                        break;
                    }
                    Some(Pass::Backdrop) if first => {
                        // The ground, before the drawing is first copied.
                        let ground = PicturePass {
                            first: true,
                            last: false,
                            resolve: true,
                        };
                        picture(encoder, ground, &mut |_| {});
                        first = false;
                    }
                    Some(p) => {
                        self.record_own(encoder, targets, frame, batches, copy_picture, p);
                        i += 1;
                    }
                },
            }
        }
    }

    /// A pass into a picture of its own: an effect group's, a map, the mask; or the backdrop's copy.
    fn record_own(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &ComposeTargets,
        frame: &ComposeFrame,
        batches: &dyn DrawBatches,
        copy_picture: &dyn Fn(&mut wgpu::CommandEncoder, &wgpu::Texture),
        p: &Pass,
    ) {
        match p {
            Pass::Group { seq } => {
                if let Some(group) = &targets.group {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("kentos.compose.group"),
                        color_attachments: &[Some(group.attachment(true))],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    batches.draw_batches(&mut pass, seq.clone());
                }
            }
            Pass::Map {
                pipe,
                target,
                bind,
                offset,
            } => {
                let view = match target {
                    Tex::Wide(l, w) => targets.maps.get(&(0, *l, *w)).map(|(_, v)| v),
                    Tex::Packed(l, w) => targets.maps.get(&(1, *l, *w)).map(|(_, v)| v),
                    Tex::Column(l, w) => targets.maps.get(&(2, *l, *w)).map(|(_, v)| v),
                    _ => None,
                };
                if let (Some(view), Some(b)) = (view, frame.binds.get(*bind)) {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("kentos.compose.map"),
                        color_attachments: &[Some(cleared(view))],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_pipeline(self.map_pipeline(*pipe));
                    pass.set_bind_group(0, b, &[*offset]);
                    pass.draw(0..3, 0..1);
                }
            }
            Pass::Boxes {
                range,
                bind,
                offset,
            } => {
                if let (Some((_, view)), Some((buffer, _)), Some(b)) =
                    (&targets.mask, &targets.boxes, frame.binds.get(*bind))
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("kentos.compose.mask"),
                        color_attachments: &[Some(cleared(view))],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_pipeline(&self.boxes);
                    pass.set_bind_group(0, b, &[*offset]);
                    pass.set_vertex_buffer(0, buffer.slice(..));
                    pass.draw(0..6, range.clone());
                }
            }
            Pass::Backdrop => {
                if let Some((texture, _)) = &targets.backdrop {
                    copy_picture(encoder, texture);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blurs_go_smaller_as_they_grow() {
        assert_eq!(level_of(0.0), 1);
        assert_eq!(level_of(8.0), 1);
        assert_eq!(level_of(8.5), 2);
        assert_eq!(level_of(32.0), 2);
        assert_eq!(level_of(40.0), 4);
        // σ = 1: e^0 + 2(e^-½ + e^-2 + e^-4½).
        let s = 1.0 + 2.0 * ((-0.5f64).exp() + (-2.0f64).exp() + (-4.5f64).exp());
        assert!((gauss_sum(1.0) - s).abs() < 1e-12);
    }

    #[test]
    fn the_shader_has_every_entry_point_its_contract_names() {
        let layout: serde_json::Value = serde_json::from_str(COMPOSE_LAYOUT_JSON).expect("JSON");
        for p in layout["pipelines"].as_array().expect("pipelines") {
            for key in ["vertex", "fragment"] {
                let name = p[key].as_str().expect("an entry point");
                assert!(
                    COMPOSE_WGSL.contains(&format!("fn {name}(")),
                    "{name} is not in compose.wgsl"
                );
            }
        }
        assert_eq!(layout["structs"]["Params"]["size"], PARAMS_BYTES);
        assert_eq!(layout["boxInstance"]["stride"], BOX_BYTES);
    }
}
