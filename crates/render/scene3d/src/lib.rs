//! KentOS's 3D scene on wgpu (docs/adr/0910 §5, plan/3d.md D1, SCENE-RENDER-01, 02): the tiles `kentos-scene3d`
//! selects drawn with a depth buffer. It knows no Iced and no window: a host hands it the device, the queue and a
//! render pass whose colour target it owns and whose depth target is a [`DepthTarget`].
//!
//! Its own package (docs/adr/8073 §2): the 3D views (the building's, the road's, the IFC viewer's) use it without the
//! 2D drawing pipeline of `kentos-render-wgpu`, whose style engine would bring the drawing document along; that
//! package re-exports it as `kentos_render_wgpu::scene3d`.
//!
//! - Each content (a tile's mesh) is uploaded once, as a vertex buffer (position float32 × 3, part u32, normal snorm8 × 4: 20 bytes)
//!   and an index buffer, and kept by [`Residency`] within two budgets (bytes resident, bytes uploaded a frame); the
//!   longest unused go to make room. A frame draws only what is resident: the host's selection keeps the parent in
//!   place until its children are (SCENE-STREAM-05).
//! - Per tile, a 16-byte block at a dynamic offset holds the tile's origin as shown minus the eye, computed in f64 by
//!   the camera and only then made float32 (SCENE-PREC-05, 08). The view block holds the projection times the
//!   rotation (no translation), the light, the exaggeration and the section's planes (docs/adr/8073 §3): each taken
//!   relative to the eye in f64 (its constant the eye's distance from it), so a fragment on the side a plane cuts away
//!   is left out with small numbers only, sample by sample (the quality's samples a pixel reach the shader with the
//!   view, so a cut face ends where its cap begins). A part whose style is not cut (a section's caps, docs/adr/8131)
//!   is drawn whole: its style reaches the shader with its alpha raised by 2.
//! - Opaque ranges first, nearest tile first (early depth rejection), writing depth; the see-through ranges (a part
//!   whose style has alpha below 1) after, farthest first, testing depth without writing it, premultiplied, biased a
//!   hair nearer so that one lying on an opaque face covers it evenly.
//! - Depth is reversed, `Depth32Float`, cleared to [`DEPTH_CLEAR`] (0), compared `Greater` (plan/3d.md SCENE-CAM-05, 06).
//! - A [`Quality`] (docs/adr/8120; Uygulama ayarları › 3B görünüm kalitesi): its samples a pixel, whether the bodies'
//!   edges are drawn (found once a content is uploaded, [`edges`]; lines over the opaque faces, before the see-through
//!   ones) and whether their corners are darkened by the ambient light's occlusion. A host drawing at a quality above
//!   the fastest draws into the view's own targets and lays them over its window ([`Scene3dRenderer::render_offscreen`],
//!   [`offscreen`]); the fastest may also draw into the host's pass ([`Scene3dRenderer::render`]).

pub mod edges;
pub mod offscreen;
pub mod residency;
pub mod shadow;

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use kentos_scene3d::{Camera, Content, Exaggeration, Viewport, math};
use wgpu::util::DeviceExt;

pub use offscreen::{COMPOSITE_WGSL, OCCLUSION_WGSL};
pub use residency::{Budgets, Plan, Residency};

/// How well a view is drawn (docs/adr/8120): Hızlı, one sample a pixel and the faces alone; Dengeli, four samples and
/// the bodies' edges; Kaliteli, besides, the ambient light's occlusion in their corners.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quality {
    Fast,
    Balanced,
    High,
}

impl Quality {
    /// The samples a pixel takes: four (every WebGPU device has them for the colour and the depth formats) or one.
    pub fn samples(self) -> u32 {
        if self == Quality::Fast { 1 } else { 4 }
    }

    /// Whether the bodies' edges are drawn.
    pub fn edges(self) -> bool {
        self != Quality::Fast
    }

    /// Whether the ambient light's occlusion darkens the corners.
    pub fn occlusion(self) -> bool {
        self == Quality::High
    }
}

/// How dark an edge is against its part's colour, and how much nearer than its faces it is drawn (a fraction of the
/// distance: a centimetre at 30 m).
const EDGE_DARK: f32 = 0.62;
const EDGE_NEAR: f32 = 4e-4;

/// The scene's WGSL (`shaders/wgsl/scene3d/mesh.wgsl`) and its layout contract.
pub const SCENE3D_WGSL: &str = include_str!("../../../../shaders/wgsl/scene3d/mesh.wgsl");
pub const SCENE3D_LAYOUT_JSON: &str = include_str!("../../../../shaders/wgsl/scene3d.layout.json");
/// The depth buffer's format and the value it is cleared to (reversed: far is 0).
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const DEPTH_CLEAR: f32 = 0.0;
/// Dynamic offsets are 256 bytes apart (wgpu's default alignment).
const STRIDE: u64 = 256;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewBlock {
    clip: [f32; 16],
    light: [f32; 4],
    look: [f32; 4],
    edge: [f32; 4],
    planes: [[f32; 4]; MAX_PLANES],
    /// The sun's projection (docs/adr/8177 §4, `shadow::projection`) and `sun`: x 1 while it casts shadows, y a texel
    /// in the map's coordinates, z in metres, w the depth bias.
    shadow: [f32; 16],
    sun: [f32; 4],
}

/// The most planes a section clips with: a box's six.
pub const MAX_PLANES: usize = 6;

/// A content's vertex as the faces', the edges' and the shadow's pipelines read it: position float32 × 3, part u32,
/// normal snorm8 × 4.
pub(crate) fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Uint32, offset: 12, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Snorm8x4, offset: 16, shader_location: 2 },
    ];
    wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &ATTRIBUTES }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    p: [f32; 3],
    part: u32,
    n: [i8; 4],
}

/// A part's look: its colour, linear RGB and alpha (below 1: drawn see-through), and whether a section's planes cut it
/// (false: a section's own caps, docs/adr/8131, cut where they are made, drawn whole in front of what the planes cut).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub color: [f32; 4],
    pub cut: bool,
}

impl Style {
    /// A part's look a section cuts.
    pub const fn of(color: [f32; 4]) -> Style {
        Style { color, cut: true }
    }

    /// As the shader reads it: a part no plane cuts has its alpha raised by 2.
    fn gpu(self) -> [f32; 4] {
        let [r, g, b, a] = self.color;
        [r, g, b, if self.cut { a } else { a + 2.0 }]
    }
}

/// What a frame draws: the camera and its view, the tiles' contents (resident ones are drawn, the others are uploaded
/// as the budgets allow, in the order given), the parts' styles (indexed by part), the light (towards it, in the shown
/// space) and its ambient share, the sky's and the fill's shares (docs/adr/8120; 0: the light alone), and the
/// half-spaces a section keeps (`[a, b, c, d]` keeping a·x + b·y + c·z + d ≥ 0 in the shown space, at most
/// [`MAX_PLANES`]; none: nothing is cut), and the box (low and high corners, the model's space) whose faces cast the
/// sun's shadows, the light being the sun (docs/adr/8177 §4; none: no shadow).
pub struct Frame<'a> {
    pub camera: &'a Camera,
    pub exaggeration: Exaggeration,
    pub viewport: Viewport,
    pub draws: &'a [&'a Content],
    pub styles: &'a [Style],
    pub light: [f64; 3],
    pub ambient: f32,
    pub sky: f32,
    pub fill: f32,
    pub clip: &'a [[f64; 4]],
    pub shadows: Option<[[f64; 3]; 2]>,
}

/// What a frame did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub tiles: usize,
    pub triangles: u64,
    pub uploaded: u64,
    pub evicted: usize,
    pub waiting: usize,
    pub resident_bytes: u64,
    pub resident_contents: usize,
}

struct Gpu {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    ranges: Vec<(u32, std::ops::Range<u32>)>,
    /// The bodies' edges as pairs of vertex indices, and how many indices; none when the quality draws none.
    lines: Option<(wgpu::Buffer, u32)>,
}

struct Draw {
    content: u64,
    slot: u32,
    range: std::ops::Range<u32>,
}

/// A view's depth buffer, made again when the target's size changes.
pub struct DepthTarget {
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl DepthTarget {
    /// The buffer's view, to attach to a render pass.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// The depth buffer for a target of `width` × `height` pixels, kept when it already is that size.
    pub fn ensure<'a>(this: &'a mut Option<DepthTarget>, device: &wgpu::Device, width: u32, height: u32) -> &'a wgpu::TextureView {
        let want = (width.max(1), height.max(1));
        if this.as_ref().is_none_or(|d| d.size != want) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("kentos.scene3d.depth"),
                size: wgpu::Extent3d { width: want.0, height: want.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            *this = Some(DepthTarget { view: texture.create_view(&wgpu::TextureViewDescriptor::default()), size: want });
        }
        match this {
            Some(d) => &d.view,
            None => unreachable!("made above"),
        }
    }
}

/// The scene's pipelines, its resident contents and this frame's draws.
pub struct Scene3dRenderer {
    quality: Quality,
    format: wgpu::TextureFormat,
    opaque: wgpu::RenderPipeline,
    see_through: wgpu::RenderPipeline,
    edges: Option<wgpu::RenderPipeline>,
    passes: offscreen::Passes,
    targets: Option<offscreen::Targets>,
    /// Whether this frame's occlusion is drawn (a perspective at the highest quality).
    occluding: bool,
    edge_draws: Vec<(u64, u32)>,
    shadows: shadow::Shadows,
    /// Whether this frame casts the sun's shadows, and the shadow pass's view group (the view and the styles).
    shadowing: bool,
    shadow_bind: wgpu::BindGroup,
    view_layout: wgpu::BindGroupLayout,
    tile_layout: wgpu::BindGroupLayout,
    view: wgpu::Buffer,
    styles: wgpu::Buffer,
    styles_capacity: u64,
    view_bind: wgpu::BindGroup,
    tiles: wgpu::Buffer,
    tiles_capacity: u64,
    tile_bind: wgpu::BindGroup,
    gpu: HashMap<u64, Gpu>,
    residency: Residency,
    pub budgets: Budgets,
    opaque_draws: Vec<Draw>,
    see_through_draws: Vec<Draw>,
    /// Whether [`Scene3dRenderer::load`] began this frame.
    begun: bool,
}

fn styles_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kentos.scene3d.styles"),
        size: capacity * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn tiles_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kentos.scene3d.tiles"),
        size: capacity * STRIDE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl Scene3dRenderer {
    /// The fastest quality's renderer, drawing into the host's pass ([`Scene3dRenderer::render`]).
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::with_quality(device, format, Quality::Fast)
    }

    /// A renderer of `quality`: its pipelines take the quality's samples.
    pub fn with_quality(device: &wgpu::Device, format: wgpu::TextureFormat, quality: Quality) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.scene3d"),
            source: wgpu::ShaderSource::Wgsl(SCENE3D_WGSL.into()),
        });
        let block = [
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<ViewBlock>() as u64),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                // The vertex stage reads whether a part is cut: a section's cap is drawn nearer, as edges are
                // (docs/adr/8131).
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            },
        ];
        // The faces read the sun's map and compare against it (docs/adr/8177 §4); the shadow pass that draws the map
        // binds the view and the styles alone.
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.scene3d.view"),
            entries: &[
                block[0],
                block[1],
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.scene3d.shadowView"),
            entries: &block,
        });
        let tile_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.scene3d.tile"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.scene3d"),
            bind_group_layouts: &[&view_layout, &tile_layout],
            push_constant_ranges: &[],
        });
        let samples = quality.samples();
        // At the highest quality the faces write their veil too (offscreen.rs): the most alpha of the see-through
        // faces over a pixel; the edges leave it as it is.
        let veiled = quality.occlusion();
        let veil = |write: bool| wgpu::ColorTargetState {
            format: offscreen::VEIL_FORMAT,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Max },
                alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Max },
            }),
            write_mask: if write { wgpu::ColorWrites::ALL } else { wgpu::ColorWrites::empty() },
        };
        let make = |see_through: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if see_through { "kentos.scene3d.seeThrough" } else { "kentos.scene3d.opaque" }),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("meshVs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[vertex_layout()],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..wgpu::PrimitiveState::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: !see_through,
                    depth_compare: wgpu::CompareFunction::Greater,
                    stencil: wgpu::StencilState::default(),
                    // A see-through face lying on an opaque one (a room's side on its wall's inner face, its floor on
                    // the slab) a hair nearer: drawn over it evenly, not in patches as their depths' last bits fall.
                    bias: if see_through {
                        wgpu::DepthBiasState { constant: 4, slope_scale: 2.0, clamp: 0.0 }
                    } else {
                        wgpu::DepthBiasState::default()
                    },
                }),
                multisample: wgpu::MultisampleState { count: samples, ..wgpu::MultisampleState::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(if veiled { "meshVeiledFs" } else { "meshFs" }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(if see_through {
                                wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING
                            } else {
                                wgpu::BlendState::REPLACE
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(veil(true)),
                    ][..if veiled { 2 } else { 1 }],
                }),
                multiview: None,
                cache: None,
            })
        };
        // The bodies' edges: lines over the opaque faces, testing their depth without writing it.
        let edges = quality.edges().then(|| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("kentos.scene3d.edges"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("edgeVs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[vertex_layout()],
                },
                primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::LineList, ..wgpu::PrimitiveState::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::GreaterEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState { count: samples, ..wgpu::MultisampleState::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("edgeFs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL }),
                        Some(veil(false)),
                    ][..if veiled { 2 } else { 1 }],
                }),
                multiview: None,
                cache: None,
            })
        });
        let view = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kentos.scene3d.view"),
            size: std::mem::size_of::<ViewBlock>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let styles = styles_buffer(device, 16);
        let shadows = shadow::Shadows::new(device, &module, &tile_layout, shadow_layout, quality);
        let view_bind = Self::view_bind(device, &view_layout, &view, &styles, &shadows);
        let shadow_bind = Self::shadow_bind(device, &shadows.layout, &view, &styles);
        let tiles = tiles_buffer(device, 64);
        let tile_bind = Self::tile_bind(device, &tile_layout, &tiles);
        Scene3dRenderer {
            quality,
            format,
            opaque: make(false),
            see_through: make(true),
            edges,
            passes: offscreen::Passes::new(device, format, quality),
            targets: None,
            occluding: false,
            edge_draws: Vec::new(),
            shadows,
            shadowing: false,
            shadow_bind,
            view_layout,
            tile_layout,
            view,
            styles,
            styles_capacity: 16,
            view_bind,
            tiles,
            tiles_capacity: 64,
            tile_bind,
            gpu: HashMap::new(),
            residency: Residency::default(),
            budgets: Budgets::default(),
            opaque_draws: Vec::new(),
            see_through_draws: Vec::new(),
            begun: false,
        }
    }

    fn view_bind(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        view: &wgpu::Buffer,
        styles: &wgpu::Buffer,
        shadows: &shadow::Shadows,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.scene3d.view"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: view.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: styles.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(shadows.bound()) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&shadows.sampler) },
            ],
        })
    }

    fn shadow_bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, view: &wgpu::Buffer, styles: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.scene3d.shadowView"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: view.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: styles.as_entire_binding() },
            ],
        })
    }

    fn tile_bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, tiles: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.scene3d.tile"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: tiles, offset: 0, size: wgpu::BufferSize::new(16) }),
            }],
        })
    }

    /// The quality it draws at.
    pub fn quality(&self) -> Quality {
        self.quality
    }

    /// The view's own targets for `width` × `height` pixels ([`Scene3dRenderer::render_offscreen`]), made again when
    /// the size changes.
    pub fn ensure_targets(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        offscreen::Targets::ensure(&mut self.targets, device, &self.passes, self.format, self.quality, width, height);
    }

    /// Where the viewport lays the view's targets: the corners of them it shows (0–1; all of them unless the view is
    /// partly off the window).
    pub fn place(&self, queue: &wgpu::Queue, uv: [f32; 4]) {
        self.passes.place(queue, uv, self.occluding);
    }

    /// Whether a content is on the GPU now: what the host's selection asks as `ready`.
    pub fn is_resident(&self, id: u64) -> bool {
        self.residency.is_resident(id)
    }

    /// Forgets a content whose source changed; it is uploaded again when a frame draws it.
    pub fn forget(&mut self, id: u64) {
        self.residency.forget(id);
        self.gpu.remove(&id);
    }

    /// Uploads what the budgets allow of `frame.draws` (in their order) and writes the frame's blocks; the draws not
    /// resident are left out of this frame.
    /// A new frame's uploads: what the budgets allow of `asked` (the contents a selection draws, then the ones it asks
    /// for, the most needed first). Called before [`Self::prepare`], the host selects again with what is now resident
    /// and draws that: a content asked for is never drawn the same frame as the parent it replaces unless the
    /// selection says so.
    pub fn load(&mut self, device: &wgpu::Device, asked: &[&Content]) -> Stats {
        self.residency.begin();
        self.begun = true;
        self.upload(device, asked)
    }

    fn upload(&mut self, device: &wgpu::Device, asked: &[&Content]) -> Stats {
        let mut stats = Stats::default();
        let ids: Vec<(u64, u64)> = asked.iter().map(|c| (c.id, c.bytes())).collect();
        let plan = self.residency.plan(&ids, self.budgets);
        for id in &plan.evict {
            self.gpu.remove(id);
        }
        stats.evicted = plan.evict.len();
        stats.waiting = plan.waiting.len();
        let upload: std::collections::HashSet<u64> = plan.upload.iter().copied().collect();
        for c in asked.iter().filter(|c| upload.contains(&c.id)) {
            let vertices: Vec<Vertex> =
                c.positions
                    .iter()
                    .zip(&c.parts)
                    .zip(c.normals.iter().chain(std::iter::repeat(&[0i8; 4])))
                    .map(|((p, part), n)| Vertex { p: *p, part: *part, n: *n })
                    .collect();
            let lines = self.edges.as_ref().map(|_| edges::feature_edges(c, edges::CREASE_DEGREES)).filter(|l| !l.is_empty()).map(|l| {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("kentos.scene3d.edges"),
                    contents: bytemuck::cast_slice(&l),
                    usage: wgpu::BufferUsages::INDEX,
                });
                (buffer, l.len() as u32)
            });
            let gpu = Gpu {
                vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("kentos.scene3d.vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("kentos.scene3d.indices"),
                    contents: bytemuck::cast_slice(&c.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                ranges: c.ranges.clone(),
                lines,
            };
            stats.uploaded += c.bytes();
            self.gpu.insert(c.id, gpu);
        }
        stats
    }

    /// Writes the frame's blocks for `frame.draws` that are resident; without a [`Self::load`] this frame, it first
    /// uploads what the budgets allow of them.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, frame: &Frame<'_>) -> Stats {
        let mut stats = if std::mem::replace(&mut self.begun, false) {
            Stats::default()
        } else {
            self.residency.begin();
            self.upload(device, frame.draws)
        };
        // The view.
        let aspect = frame.viewport.aspect();
        let light = math::normalize(frame.light);
        // A plane about the eye: a·(eye + r) + d = a·r + (a·eye + d), its constant the eye's distance from it.
        let eye = frame.camera.eye;
        let mut planes = [[0.0f32; 4]; MAX_PLANES];
        let used = frame.clip.len().min(MAX_PLANES);
        for (out, p) in planes.iter_mut().zip(frame.clip) {
            let w = p[0] * eye[0] + p[1] * eye[1] + p[2] * eye[2] + p[3];
            *out = [p[0] as f32, p[1] as f32, p[2] as f32, w as f32];
        }
        let perspective = matches!(frame.camera.projection, kentos_scene3d::Projection::Perspective { .. });
        // The sun's shadows: its projection fitted to the box as shown; the map made the first time.
        self.shadowing = frame.shadows.is_some();
        let (mut shadow, mut sun) = ([0.0f32; 16], [0.0f32; 4]);
        if let Some([low, high]) = frame.shadows {
            let (low, high) = (frame.exaggeration.shown(low), frame.exaggeration.shown(high));
            let p = shadow::projection(low, high, eye, light, self.shadows.side);
            (shadow, sun) = (p.clip, [1.0, p.texel_uv, p.texel_m, p.bias]);
            if self.shadows.ensure(device) {
                self.view_bind = Self::view_bind(device, &self.view_layout, &self.view, &self.styles, &self.shadows);
            }
        }
        let block = ViewBlock {
            clip: frame.camera.rotation_projection(aspect).to_f32(),
            light: [light[0] as f32, light[1] as f32, light[2] as f32, frame.ambient],
            look: [frame.exaggeration.factor as f32, used as f32, frame.sky, frame.fill],
            edge: [if perspective { 1.0 } else { 0.0 }, EDGE_NEAR, EDGE_DARK, self.quality.samples() as f32],
            planes,
            shadow,
            sun,
        };
        queue.write_buffer(&self.view, 0, bytemuck::bytes_of(&block));
        // The occlusion's lens: a perspective's (an orthographic view has no distance to measure it by).
        self.occluding = false;
        if let (Some(t), kentos_scene3d::Projection::Perspective { fov_y, near }) = (&self.targets, frame.camera.projection)
            && t.occludes()
        {
            let g = 1.0 / (fov_y / 2.0).tan();
            self.passes.lens(
                queue,
                offscreen::OcclusionBlock {
                    lens: [g as f32, aspect as f32, near as f32, 0.0],
                    shape: [offscreen::OCCLUSION_RADIUS as f32, offscreen::OCCLUSION_STRENGTH, t.size.0 as f32, t.size.1 as f32],
                },
            );
            self.occluding = true;
        }
        let n = frame.styles.len().max(1) as u64;
        if n > self.styles_capacity {
            self.styles_capacity = n.next_power_of_two();
            self.styles = styles_buffer(device, self.styles_capacity);
            self.view_bind = Self::view_bind(device, &self.view_layout, &self.view, &self.styles, &self.shadows);
            self.shadow_bind = Self::shadow_bind(device, &self.shadows.layout, &self.view, &self.styles);
        }
        let colors: Vec<[f32; 4]> = frame.styles.iter().map(|s| s.gpu()).collect();
        if !colors.is_empty() {
            queue.write_buffer(&self.styles, 0, bytemuck::cast_slice(&colors));
        }
        // The tiles drawn: their offsets, nearest first.
        let mut drawn: Vec<(&Content, [f32; 3], f64)> = frame
            .draws
            .iter()
            .filter(|c| self.gpu.contains_key(&c.id))
            .map(|c| {
                let off = frame.camera.offset(c.origin, &frame.exaggeration);
                let d = math::length(math::sub(frame.exaggeration.shown(c.origin), frame.camera.eye));
                (*c, off, d)
            })
            .collect();
        drawn.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.id.cmp(&b.0.id)));
        let need = drawn.len().max(1) as u64;
        if need > self.tiles_capacity {
            self.tiles_capacity = need.next_power_of_two();
            self.tiles = tiles_buffer(device, self.tiles_capacity);
            self.tile_bind = Self::tile_bind(device, &self.tile_layout, &self.tiles);
        }
        let mut staged = vec![0u8; (drawn.len() as u64 * STRIDE) as usize];
        self.opaque_draws.clear();
        self.see_through_draws.clear();
        self.edge_draws.clear();
        for (slot, (c, off, _)) in drawn.iter().enumerate() {
            let at = slot * STRIDE as usize;
            staged[at..at + 16].copy_from_slice(bytemuck::bytes_of(&[off[0], off[1], off[2], 0.0f32]));
            self.residency.touch(c.id);
            stats.tiles += 1;
            stats.triangles += c.indices.len() as u64 / 3;
            for (part, range) in &self.gpu[&c.id].ranges {
                let clear = frame.styles.get(*part as usize).is_some_and(|s| s.color[3] < 1.0);
                let d = Draw { content: c.id, slot: slot as u32, range: range.clone() };
                if clear { self.see_through_draws.push(d) } else { self.opaque_draws.push(d) }
            }
            if self.gpu[&c.id].lines.is_some() {
                self.edge_draws.push((c.id, slot as u32));
            }
        }
        self.see_through_draws.reverse();
        if !staged.is_empty() {
            queue.write_buffer(&self.tiles, 0, &staged);
        }
        (stats.resident_bytes, stats.resident_contents) = self.residency.resident();
        stats
    }

    /// Draws the prepared frame into `pass`, whose depth attachment is a [`DepthTarget`] cleared to [`DEPTH_CLEAR`]
    /// (its samples the quality's): the opaque faces, their edges, then the see-through faces.
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, &self.view_bind, &[]);
        self.faces(pass, &self.opaque, &self.opaque_draws);
        if let Some(edges) = &self.edges
            && !self.edge_draws.is_empty()
        {
            pass.set_pipeline(edges);
            for (content, slot) in &self.edge_draws {
                let Some((g, (lines, count))) = self.gpu.get(content).and_then(|g| Some((g, g.lines.as_ref()?))) else {
                    continue;
                };
                pass.set_vertex_buffer(0, g.vertices.slice(..));
                pass.set_index_buffer(lines.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_bind_group(1, &self.tile_bind, &[(u64::from(*slot) * STRIDE) as u32]);
                pass.draw_indexed(0..*count, 0, 0..1);
            }
        }
        self.faces(pass, &self.see_through, &self.see_through_draws);
    }

    fn faces(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: &wgpu::RenderPipeline, draws: &[Draw]) {
        if draws.is_empty() {
            return;
        }
        pass.set_pipeline(pipeline);
        let mut bound = None;
        for d in draws {
            let Some(g) = self.gpu.get(&d.content) else { continue };
            if bound != Some(d.content) {
                pass.set_vertex_buffer(0, g.vertices.slice(..));
                pass.set_index_buffer(g.indices.slice(..), wgpu::IndexFormat::Uint32);
                bound = Some(d.content);
            }
            pass.set_bind_group(1, &self.tile_bind, &[(u64::from(d.slot) * STRIDE) as u32]);
            pass.draw_indexed(d.range.clone(), 0, 0..1);
        }
    }

    /// Draws the sun's map of the prepared frame when it casts shadows (docs/adr/8177 §4): before the faces' pass, which
    /// reads it. [`Scene3dRenderer::render_offscreen`] draws it itself; a host drawing into its own pass
    /// ([`Scene3dRenderer::render`]) encodes it first.
    pub fn render_shadows(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(map) = self.shadows.map.as_ref().filter(|_| self.shadowing) else { return };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kentos.scene3d.shadow"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: map,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_bind_group(0, &self.shadow_bind, &[]);
        self.faces(&mut pass, &self.shadows.pipeline, &self.opaque_draws);
    }

    /// Draws the prepared frame into the view's own targets ([`Scene3dRenderer::ensure_targets`]) and lays them over
    /// `output` at `viewport` (pixels: x, y, width, height) within `scissor` (x0, y0, x1, y1), darkened by the
    /// occlusion when the quality asks; the sun's map first when the frame casts shadows.
    pub fn render_offscreen(&self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView, viewport: [f32; 4], scissor: [u32; 4]) {
        let Some(t) = &self.targets else { return };
        self.render_shadows(encoder);
        {
            let mut pass = t.scene_pass(encoder);
            self.render(&mut pass);
        }
        t.finish(&self.passes, encoder, output, viewport, scissor);
    }
}
