//! Point clouds on the GPU (docs/adr/0207 §6): each frame, each cloud in view
//! draws the octree nodes `geom::pointcloud::visible` picks (coarse first, a
//! budget of points) into a picture of its own with their depth, the highest
//! nearest (`shaders/wgsl/points`); the styled pass then shows that picture
//! over the cloud's plan in the drawing's order (`cloudFs`). Nodes stay on the
//! GPU, the least recently drawn let go first; a new look uploads a node
//! again (its colours). The host decodes and colours the nodes on its own
//! threads (`ImageSource::cloud_node`) and asks for a frame when they are made.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use kentos_geometry_core::geom::pointcloud::{Octree, visible};

use super::picture::ImageSource;

/// Points drawn in a frame at most (all clouds together).
pub const FRAME_BUDGET: u64 = 4_000_000;
/// Points uploaded in a frame at most; the rest the next.
pub const UPLOADS_PER_FRAME: u64 = 1_000_000;
/// Points kept on the GPU at most.
pub const RESIDENT_MOST: u64 = 6_000_000;
/// Bytes of a node draw's block (the contract's `Draw`).
const DRAW_BYTES: u64 = 48;

/// A cloud's octrees, one per file (none while a file is opened or indexed),
/// and its heights' range, which the depth spans.
#[derive(Clone, Debug, Default)]
pub struct CloudTrees {
    pub members: Vec<Option<Octree>>,
    pub z: [f64; 2],
}

/// A node's points ready to draw: positions from its centre, colours (RGBA, alpha 0 hidden).
#[derive(Clone, Debug, Default)]
pub struct CloudNode {
    pub center: [f64; 3],
    pub xyz: Vec<f32>,
    pub rgba: Vec<u8>,
}

impl CloudNode {
    pub fn count(&self) -> u32 {
        (self.xyz.len() / 3) as u32
    }
}

/// What a cloud batch's paint says: its files, look and colour, and the look's size and shape.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudPaint {
    pub cloud: String,
    pub look: String,
    pub color: String,
    /// `cloud`, `look` and `color` together: the picture's name.
    pub key: String,
    /// The host's name of the look a node is coloured for ([`CloudPaint::colouring`]).
    colouring: String,
    /// `cloud` and `colouring` hashed: a node's name on the GPU without a string.
    cloud_hash: u64,
    look_hash: u64,
    size: f64,
    metres: bool,
    round: bool,
}

impl CloudPaint {
    pub fn new(cloud: &str, look: &str, color: &str) -> CloudPaint {
        use kentos_geometry_core::api::json::Json;
        let parsed = Json::parse(look).ok();
        let get = |k: &str| parsed.as_ref().map(|j| j.get(k).clone());
        let size = match get("size") {
            Some(Json::Num(v)) if v.is_finite() && v > 0.0 => v,
            _ => 2.0,
        };
        let text = |k: &str| match get(k) {
            Some(Json::Str(s)) => Some(s),
            _ => None,
        };
        let colouring = format!("{look}|{color}");
        CloudPaint {
            cloud: cloud.to_owned(),
            look: look.to_owned(),
            color: color.to_owned(),
            key: format!("{cloud}|{look}|{color}"),
            cloud_hash: hash_of(cloud),
            look_hash: hash_of(&colouring),
            colouring,
            size,
            metres: text("sizeUnit").as_deref() == Some("m"),
            round: text("shape").as_deref() != Some("square"),
        }
    }

    /// The host's name of the look a node is coloured for: the look and the object's colour.
    pub fn colouring(&self) -> &str {
        &self.colouring
    }

    /// A point's width in device pixels.
    fn point_px(&self, px_per_m: f64, dpr: f64) -> f64 {
        if self.metres {
            (self.size * px_per_m).max(1.0)
        } else {
            (self.size * dpr).max(1.0)
        }
    }
}

/// A node on the GPU: its cloud's and look's hashes, its file and its place in the octree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct NodeId {
    cloud: u64,
    member: u32,
    node: u32,
    look: u64,
}

struct GpuNode {
    xyz: wgpu::Buffer,
    rgba: wgpu::Buffer,
    count: u32,
    center: [f64; 3],
    last: u64,
}

/// A cloud's picture: its colour and depth.
pub(crate) struct CloudTarget {
    _color: wgpu::Texture,
    pub(crate) view: Arc<wgpu::TextureView>,
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    size: [u32; 2],
}

/// The points pipeline and the nodes on the GPU.
pub(crate) struct PointsGpu {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: Option<(wgpu::Buffer, wgpu::BindGroup, u64)>,
    align: u64,
    nodes: HashMap<NodeId, GpuNode>,
    resident: u64,
    tick: u64,
    /// The picture's format: sRGB-encoded colours on a target that wants them linear are decoded by it.
    format: wgpu::TextureFormat,
    /// A frame's working lists, kept between frames: no frame allocates once they are grown.
    picked: Vec<u32>,
    /// Each node drawn this frame: its paint's place and its name.
    draws: Vec<(u32, NodeId)>,
    /// Each paint drawn this frame: its place and its heights' range.
    ranges: Vec<(u32, [f64; 2])>,
    blocks: Vec<u8>,
}

fn hash_of(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

impl PointsGpu {
    pub(crate) fn new(device: &wgpu::Device, srgb_target: bool) -> PointsGpu {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.points"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../../../../shaders/wgsl/points/points.wgsl").into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.points.draw"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(DRAW_BYTES),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.points"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let format = if srgb_target {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        const XYZ: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
        const RGBA: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![1 => Unorm8x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kentos.points"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("pointsVs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &XYZ,
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: 4,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &RGBA,
                    },
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("pointsFs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let align = DRAW_BYTES.next_multiple_of(
            u64::from(device.limits().min_uniform_buffer_offset_alignment).max(16),
        );
        PointsGpu {
            pipeline,
            layout,
            uniform: None,
            align,
            nodes: HashMap::new(),
            resident: 0,
            tick: 0,
            format,
            picked: Vec::new(),
            draws: Vec::new(),
            ranges: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// A picture of `size` device pixels for a cloud.
    pub(crate) fn target(&self, device: &wgpu::Device, size: [u32; 2]) -> CloudTarget {
        let extent = wgpu::Extent3d {
            width: size[0].max(1),
            height: size[1].max(1),
            depth_or_array_layers: 1,
        };
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kentos.points.picture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kentos.points.depth"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        CloudTarget {
            view: Arc::new(color.create_view(&wgpu::TextureViewDescriptor::default())),
            depth_view: depth.create_view(&wgpu::TextureViewDescriptor::default()),
            _color: color,
            _depth: depth,
            size,
        }
    }

    /// Draws the clouds `paints` (their pictures in `targets`, made or resized
    /// as the view is) for a frame whose camera is at `center`, `px_per_m`
    /// device pixels a metre, `size` device pixels; returns whether a node
    /// waits for the host and whether any picture changed.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        paints: &[CloudPaint],
        targets: &mut HashMap<String, CloudTarget>,
        center: [f64; 2],
        px_per_m: f64,
        dpr: f64,
        size: [f32; 2],
        images: &dyn ImageSource,
    ) -> bool {
        let size = [size[0].max(1.0) as u32, size[1].max(1.0) as u32];
        self.tick += 1;
        let tick = self.tick;
        targets.retain(|k, t| paints.iter().any(|p| p.key == *k) && t.size == size);
        images.cloud_frame();
        let (hw, hh) = (
            f64::from(size[0]) / 2.0 / px_per_m,
            f64::from(size[1]) / 2.0 / px_per_m,
        );
        let view_box = [
            center[0] - hw,
            center[1] - hh,
            center[0] + hw,
            center[1] + hh,
        ];
        let mut waiting = false;
        let mut budget = FRAME_BUDGET;
        let mut uploads = UPLOADS_PER_FRAME;
        self.draws.clear();
        self.ranges.clear();
        for (pi, paint) in paints.iter().enumerate() {
            let Some(trees) = images.cloud_trees(&paint.cloud) else {
                continue;
            };
            let point_px = paint.point_px(px_per_m, dpr);
            for (m, tree) in trees.members.iter().enumerate() {
                let Some(tree) = tree else {
                    continue;
                };
                visible(tree, view_box, px_per_m, point_px, budget, &mut self.picked);
                for &i in &self.picked {
                    let count = tree.counts[i as usize];
                    budget = budget.saturating_sub(count);
                    if count == 0 {
                        continue;
                    }
                    let id = NodeId {
                        cloud: paint.cloud_hash,
                        member: m as u32,
                        node: i,
                        look: paint.look_hash,
                    };
                    if let Some(n) = self.nodes.get_mut(&id) {
                        n.last = tick;
                        self.draws.push((pi as u32, id));
                        continue;
                    }
                    if uploads == 0 {
                        waiting = true;
                        continue;
                    }
                    match images.cloud_node(&paint.cloud, m as u32, i, paint.colouring()) {
                        Some(node) if node.count() > 0 => {
                            let count = node.count();
                            uploads = uploads.saturating_sub(u64::from(count));
                            let xyz = upload(
                                device,
                                queue,
                                "kentos.points.xyz",
                                bytemuck::cast_slice(&node.xyz),
                            );
                            let rgba = upload(device, queue, "kentos.points.rgba", &node.rgba);
                            self.resident += u64::from(count);
                            self.nodes.insert(
                                id,
                                GpuNode {
                                    xyz,
                                    rgba,
                                    count,
                                    center: node.center,
                                    last: tick,
                                },
                            );
                            self.draws.push((pi as u32, id));
                        }
                        Some(_) => {}
                        None => waiting = true,
                    }
                }
            }
            self.ranges.push((pi as u32, trees.z));
        }
        // The least recently drawn nodes go while too many points are kept (rare: an allocation is fine here).
        if self.resident > RESIDENT_MOST {
            let mut old: Vec<(u64, NodeId, u32)> = self
                .nodes
                .iter()
                .filter(|(_, n)| n.last != tick)
                .map(|(k, n)| (n.last, *k, n.count))
                .collect();
            old.sort_by_key(|(last, _, _)| *last);
            for (_, k, count) in old {
                if self.resident <= RESIDENT_MOST {
                    break;
                }
                self.nodes.remove(&k);
                self.resident = self.resident.saturating_sub(u64::from(count));
            }
        }
        // Every draw's block at its dynamic offset, in the draws' order.
        self.blocks.clear();
        for &(pi, id) in &self.draws {
            let paint = &paints[pi as usize];
            let Some(n) = self.nodes.get(&id) else {
                continue;
            };
            let z = self
                .ranges
                .iter()
                .find(|(p, _)| *p == pi)
                .map_or([0.0, 1.0], |(_, z)| *z);
            let point_px = paint.point_px(px_per_m, dpr);
            let range = z[1] - z[0];
            let inv = if range > 0.0 { 1.0 / range } else { 1.0 };
            let block: [f32; 12] = [
                (n.center[0] - center[0]) as f32,
                (n.center[1] - center[1]) as f32,
                (n.center[2] - z[0]) as f32,
                0.0,
                (px_per_m * 2.0 / f64::from(size[0].max(1))) as f32,
                (px_per_m * 2.0 / f64::from(size[1].max(1))) as f32,
                (point_px / f64::from(size[0].max(1))) as f32,
                (point_px / f64::from(size[1].max(1))) as f32,
                inv as f32,
                if paint.round { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ];
            let at = self.blocks.len();
            self.blocks.extend_from_slice(bytemuck::cast_slice(&block));
            self.blocks.resize(at + self.align as usize, 0);
        }
        if !self.blocks.is_empty() {
            let fits = self
                .uniform
                .as_ref()
                .is_some_and(|(b, _, _)| b.size() >= self.blocks.len() as u64);
            if !fits {
                let bytes = (self.blocks.len() as u64).next_power_of_two();
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("kentos.points.draws"),
                    size: bytes,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kentos.points.draws"),
                    layout: &self.layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(DRAW_BYTES),
                        }),
                    }],
                });
                self.uniform = Some((buffer, bind, bytes));
            }
            if let Some((buffer, _, _)) = &self.uniform {
                queue.write_buffer(buffer, 0, &self.blocks);
            }
        }
        // Each cloud's picture: cleared, then its nodes (a cloud whose files are not open yet clears to nothing).
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("kentos.points"),
        });
        let mut at = 0u64;
        let mut next = 0usize;
        for &(pi, _) in &self.ranges {
            let paint = &paints[pi as usize];
            let target = targets
                .entry(paint.key.clone())
                .or_insert_with(|| self.target(device, size));
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("kentos.points.picture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            while let Some(&(dp, id)) = self.draws.get(next)
                && dp == pi
            {
                next += 1;
                let Some(n) = self.nodes.get(&id) else {
                    continue;
                };
                if let Some((_, bind, _)) = &self.uniform {
                    pass.set_bind_group(0, bind, &[at as u32]);
                    pass.set_vertex_buffer(0, n.xyz.slice(..));
                    pass.set_vertex_buffer(1, n.rgba.slice(..));
                    pass.draw(0..6, 0..n.count);
                }
                at += self.align;
            }
        }
        queue.submit(std::iter::once(encoder.finish()));
        waiting
    }

    /// Points on the GPU (statistics).
    pub(crate) fn resident(&self) -> u64 {
        self.resident
    }
}

fn upload(device: &wgpu::Device, queue: &wgpu::Queue, label: &str, bytes: &[u8]) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (bytes.len() as u64).max(4).next_multiple_of(4),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, bytes);
    buffer
}
