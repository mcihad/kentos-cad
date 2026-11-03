//! KentOS's 3D scene on wgpu (docs/adr/0910 §5, plan/3d.md D1, SCENE-RENDER-01, 02): the tiles `kentos-scene3d`
//! selects drawn with a depth buffer. It knows no Iced and no window: a host hands it the device, the queue and a
//! render pass whose colour target it owns and whose depth target is a [`DepthTarget`].
//!
//! - Each content (a tile's mesh) is uploaded once, as a vertex buffer (position float32 × 3, part u32, normal snorm8 × 4: 20 bytes)
//!   and an index buffer, and kept by [`Residency`] within two budgets (bytes resident, bytes uploaded a frame); the
//!   longest unused go to make room. A frame draws only what is resident: the host's selection keeps the parent in
//!   place until its children are (SCENE-STREAM-05).
//! - Per tile, a 16-byte block at a dynamic offset holds the tile's origin as shown minus the eye, computed in f64 by
//!   the camera and only then made float32 (SCENE-PREC-05, 08). The view block holds the projection times the
//!   rotation (no translation), the light and the exaggeration.
//! - Opaque ranges first, nearest tile first (early depth rejection), writing depth; the see-through ranges (a part
//!   whose style has alpha below 1) after, farthest first, testing depth without writing it, premultiplied.
//! - Depth is reversed, `Depth32Float`, cleared to [`DEPTH_CLEAR`] (0), compared `Greater` (plan/3d.md SCENE-CAM-05, 06).

pub mod residency;

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use kentos_scene3d::{Camera, Content, Exaggeration, Viewport, math};
use wgpu::util::DeviceExt;

pub use residency::{Budgets, Plan, Residency};

/// The scene's WGSL (`shaders/wgsl/scene3d/mesh.wgsl`) and its layout contract.
pub const SCENE3D_WGSL: &str = include_str!("../../../../../shaders/wgsl/scene3d/mesh.wgsl");
pub const SCENE3D_LAYOUT_JSON: &str = include_str!("../../../../../shaders/wgsl/scene3d.layout.json");
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
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    p: [f32; 3],
    part: u32,
    n: [i8; 4],
}

/// A part's look: its colour, linear RGB and alpha (below 1: drawn see-through).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub color: [f32; 4],
}

/// What a frame draws: the camera and its view, the tiles' contents (resident ones are drawn, the others are uploaded
/// as the budgets allow, in the order given), the parts' styles (indexed by part), the light (towards it, in the shown
/// space) and its ambient share.
pub struct Frame<'a> {
    pub camera: &'a Camera,
    pub exaggeration: Exaggeration,
    pub viewport: Viewport,
    pub draws: &'a [&'a Content],
    pub styles: &'a [Style],
    pub light: [f64; 3],
    pub ambient: f32,
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
    opaque: wgpu::RenderPipeline,
    see_through: wgpu::RenderPipeline,
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
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.scene3d"),
            source: wgpu::ShaderSource::Wgsl(SCENE3D_WGSL.into()),
        });
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.scene3d.view"),
            entries: &[
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
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
        let make = |see_through: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if see_through { "kentos.scene3d.seeThrough" } else { "kentos.scene3d.opaque" }),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("meshVs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 20,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Uint32, offset: 12, shader_location: 1 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Snorm8x4, offset: 16, shader_location: 2 },
                        ],
                    }],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..wgpu::PrimitiveState::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: !see_through,
                    depth_compare: wgpu::CompareFunction::Greater,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("meshFs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(if see_through {
                            wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING
                        } else {
                            wgpu::BlendState::REPLACE
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        };
        let view = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kentos.scene3d.view"),
            size: std::mem::size_of::<ViewBlock>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let styles = styles_buffer(device, 16);
        let view_bind = Self::view_bind(device, &view_layout, &view, &styles);
        let tiles = tiles_buffer(device, 64);
        let tile_bind = Self::tile_bind(device, &tile_layout, &tiles);
        Scene3dRenderer {
            opaque: make(false),
            see_through: make(true),
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

    fn view_bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, view: &wgpu::Buffer, styles: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.scene3d.view"),
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
        let block = ViewBlock {
            clip: frame.camera.rotation_projection(aspect).to_f32(),
            light: [light[0] as f32, light[1] as f32, light[2] as f32, frame.ambient],
            look: [frame.exaggeration.factor as f32, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.view, 0, bytemuck::bytes_of(&block));
        let n = frame.styles.len().max(1) as u64;
        if n > self.styles_capacity {
            self.styles_capacity = n.next_power_of_two();
            self.styles = styles_buffer(device, self.styles_capacity);
            self.view_bind = Self::view_bind(device, &self.view_layout, &self.view, &self.styles);
        }
        let colors: Vec<[f32; 4]> = frame.styles.iter().map(|s| s.color).collect();
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
        }
        self.see_through_draws.reverse();
        if !staged.is_empty() {
            queue.write_buffer(&self.tiles, 0, &staged);
        }
        (stats.resident_bytes, stats.resident_contents) = self.residency.resident();
        stats
    }

    /// Draws the prepared frame into `pass`, whose depth attachment is a [`DepthTarget`] cleared to [`DEPTH_CLEAR`].
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, &self.view_bind, &[]);
        for (pipeline, draws) in [(&self.opaque, &self.opaque_draws), (&self.see_through, &self.see_through_draws)] {
            if draws.is_empty() {
                continue;
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
    }
}
