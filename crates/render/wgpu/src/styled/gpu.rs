//! The styled layers on the GPU (the web's `webgpu/styledRenderer.ts`):
//! the six pipelines of the shared styled WGSL per sample count, the atlas
//! texture, and each layer's buffers: its batches' numbers in one vertex
//! buffer and their style blocks in one uniform buffer read at a dynamic
//! offset, so a batch costs a bind and a draw, never a buffer of its own.
//!
//! A frame first decides what shows (scale range, view box, legibility) and
//! places its images in the atlas, writing moved rectangles into the style
//! blocks; then it only issues draw calls, switching pipelines when they change.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use kentos_native_style::batches::{BatchKind, FillPaintBatch, StyledLayer};
use wgpu::util::DeviceExt;

use super::atlas::{Atlas, PAGE};
use super::picture::ImageSource;
use super::shader;
use super::uniform::{STYLE_BYTES, StyleBlock, StyledFrameUniform, style_block};

/// A styled layer to draw, with the id the renderer keeps its buffers by.
#[derive(Debug)]
pub struct StyledLayerPart {
    /// Unique per build: a layer the view has not drawn under this id is uploaded.
    pub id: u64,
    pub layer: StyledLayer,
}

/// The styled layers of a view, bottom first, and how many of the view's
/// plain parts are drawn beneath them (the grid; the highlights come after).
#[derive(Clone, Debug, Default)]
pub struct StyledScene {
    pub layers: Vec<Arc<StyledLayerPart>>,
    pub under: usize,
    /// The draw order as `(layer, batch)` pairs, when a large layer comes in
    /// parts whose batches interleave (the desktop's parts, drawn in the
    /// order of the layer built whole: `batches::merged_order`); none draws
    /// every layer's batches in turn.
    pub order: Option<Arc<Vec<(u32, u32)>>>,
}

/// What a frame of styled layers needs.
#[derive(Clone, Copy, Debug)]
pub struct StyledFrame {
    /// Camera centre relative to the layers' origin, metres.
    pub center: [f64; 2],
    /// Logical pixels per metre.
    pub scale: f64,
    /// Device pixels per logical pixel of the target drawn into.
    pub dpr: f64,
    /// The target's size in device pixels.
    pub size_px: [f32; 2],
    /// The screen scale 1:N at 96 dpi (the rules' scale ranges).
    pub scale_denominator: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pipe {
    Stroke,
    Solid,
    Hatch,
    Pattern,
    Tile,
    Marker,
    Gradient,
    Image,
}

struct Pipelines {
    stroke: wgpu::RenderPipeline,
    solid: wgpu::RenderPipeline,
    hatch: wgpu::RenderPipeline,
    pattern: wgpu::RenderPipeline,
    tile: wgpu::RenderPipeline,
    marker: wgpu::RenderPipeline,
    gradient: wgpu::RenderPipeline,
    image: wgpu::RenderPipeline,
}

impl Pipelines {
    fn get(&self, p: Pipe) -> &wgpu::RenderPipeline {
        match p {
            Pipe::Stroke => &self.stroke,
            Pipe::Solid => &self.solid,
            Pipe::Hatch => &self.hatch,
            Pipe::Pattern => &self.pattern,
            Pipe::Tile => &self.tile,
            Pipe::Marker => &self.marker,
            Pipe::Gradient => &self.gradient,
            Pipe::Image => &self.image,
        }
    }
}

const STRAIGHT: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

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

const STROKE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x2];
const AREA_ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
const MARKER_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32];

/// Floats per stroke segment, fill vertex and marker (the contract's strides).
pub const STROKE_FLOATS: usize = 6;
pub const AREA_FLOATS: usize = 2;
pub const MARKER_FLOATS: usize = 5;

const STROKE_BUFFER: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: (STROKE_FLOATS * 4) as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &STROKE_ATTRIBUTES,
};
const AREA_BUFFER: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: (AREA_FLOATS * 4) as u64,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &AREA_ATTRIBUTES,
};
const MARKER_BUFFER: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: (MARKER_FLOATS * 4) as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &MARKER_ATTRIBUTES,
};

/// A styled pipeline as the contract describes it (`styled.layout.json` → `pipelines`).
#[derive(Clone, Debug)]
pub struct StyledPipelineSpec {
    pub name: &'static str,
    pub vertex: &'static str,
    pub fragment: &'static str,
    /// Vertices per instance of an instanced pipeline.
    pub vertex_count: Option<u32>,
    pub blend: wgpu::BlendState,
    pub buffer: wgpu::VertexBufferLayout<'static>,
}

/// The contract's pipelines, in its order: stroke, solid, hatch, pattern, tile, marker, gradient, image.
pub const STYLED_PIPELINES: [StyledPipelineSpec; 8] = [
    StyledPipelineSpec {
        name: "stroke",
        vertex: "strokeVs",
        fragment: "strokeFs",
        vertex_count: Some(6),
        blend: STRAIGHT,
        buffer: STROKE_BUFFER,
    },
    StyledPipelineSpec {
        name: "solid",
        vertex: "areaVs",
        fragment: "solidFs",
        vertex_count: None,
        blend: STRAIGHT,
        buffer: AREA_BUFFER,
    },
    StyledPipelineSpec {
        name: "hatch",
        vertex: "areaVs",
        fragment: "hatchFs",
        vertex_count: None,
        blend: STRAIGHT,
        buffer: AREA_BUFFER,
    },
    StyledPipelineSpec {
        name: "pattern",
        vertex: "areaVs",
        fragment: "patternFs",
        vertex_count: None,
        blend: PREMULTIPLIED,
        buffer: AREA_BUFFER,
    },
    StyledPipelineSpec {
        name: "tile",
        vertex: "areaVs",
        fragment: "tileFs",
        vertex_count: None,
        blend: PREMULTIPLIED,
        buffer: AREA_BUFFER,
    },
    StyledPipelineSpec {
        name: "marker",
        vertex: "markerVs",
        fragment: "markerFs",
        vertex_count: Some(6),
        blend: PREMULTIPLIED,
        buffer: MARKER_BUFFER,
    },
    StyledPipelineSpec {
        name: "gradient",
        vertex: "areaVs",
        fragment: "gradientFs",
        vertex_count: None,
        blend: STRAIGHT,
        buffer: AREA_BUFFER,
    },
    StyledPipelineSpec {
        name: "image",
        vertex: "areaVs",
        fragment: "imageFs",
        vertex_count: None,
        blend: PREMULTIPLIED,
        buffer: AREA_BUFFER,
    },
];

/// The styled pipelines, layouts and atlas of one device.
pub struct StyledGpu {
    format: wgpu::TextureFormat,
    module: wgpu::ShaderModule,
    pub(crate) frame_layout: wgpu::BindGroupLayout,
    style_layout: wgpu::BindGroupLayout,
    layout: wgpu::PipelineLayout,
    pipelines: HashMap<u32, Pipelines>,
    texture: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    pub(crate) atlas: Atlas,
    /// The drawing's pictures (docs/adr/0192 §3).
    pictures: super::pictures::PictureTextures,
    /// The device's alignment of dynamic uniform offsets.
    align: u64,
    /// Colours are sRGB-encoded as written; an sRGB target would encode them twice.
    srgb_target: bool,
}

impl std::fmt::Debug for StyledGpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StyledGpu")
            .field("format", &self.format)
            .field("samples", &self.pipelines.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl StyledGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> StyledGpu {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.styled"),
            source: wgpu::ShaderSource::Wgsl(shader::source().into()),
        });
        let uniform = |size: u64, dynamic: bool| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: wgpu::BufferSize::new(size),
            },
            count: None,
        };
        // Group 0: the frame and the atlas, as the contract's version 2 binds them
        // (Iced's device takes two groups). Group 1: the batch's style.
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.styled.frame"),
            entries: &[
                uniform(std::mem::size_of::<StyledFrameUniform>() as u64, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let style_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.styled.style"),
            entries: &[uniform(STYLE_BYTES as u64, true)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.styled"),
            bind_group_layouts: &[&frame_layout, &style_layout],
            push_constant_ranges: &[],
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kentos.styled.atlas"),
            size: wgpu::Extent3d {
                width: PAGE,
                height: PAGE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kentos.styled.atlas"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let atlas_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Blocks sit at multiples of the device's dynamic offset alignment.
        let min_align = u64::from(device.limits().min_uniform_buffer_offset_alignment).max(16);
        let align = (STYLE_BYTES as u64).next_multiple_of(min_align);
        StyledGpu {
            pictures: super::pictures::PictureTextures::new(device),
            format,
            module,
            frame_layout,
            style_layout,
            layout,
            pipelines: HashMap::new(),
            texture,
            atlas_view,
            sampler,
            atlas: Atlas::new(),
            align,
            srgb_target: format.is_srgb(),
        }
    }

    /// The pipelines for `samples` per pixel, made the first time that count draws.
    fn pipelines(&mut self, device: &wgpu::Device, samples: u32) -> &Pipelines {
        let (module, layout, format) = (&self.module, &self.layout, self.format);
        self.pipelines.entry(samples).or_insert_with(|| {
            let pipe = |spec: &StyledPipelineSpec| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(spec.name),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: Some(spec.vertex),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: std::slice::from_ref(&spec.buffer),
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: None,
                        ..wgpu::PrimitiveState::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        ..wgpu::MultisampleState::default()
                    },
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: Some(spec.fragment),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(spec.blend),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                    cache: None,
                })
            };
            let [stroke, solid, hatch, pattern, tile, marker, gradient, image] = &STYLED_PIPELINES;
            Pipelines {
                stroke: pipe(stroke),
                solid: pipe(solid),
                hatch: pipe(hatch),
                pattern: pipe(pattern),
                tile: pipe(tile),
                marker: pipe(marker),
                gradient: pipe(gradient),
                image: pipe(image),
            }
        })
    }

    /// Makes the pipelines of a sample count ahead of drawing (a pass cannot make them).
    pub fn ensure(&mut self, device: &wgpu::Device, samples: u32) {
        let _ = self.pipelines(device, samples);
    }

    /// A style block as the target takes its colours.
    fn block(&self, b: &kentos_native_style::StyledBatch) -> StyleBlock {
        let mut block = style_block(b);
        if self.srgb_target {
            // Only on a gamma-correct surface (Iced picks a plain one, as the web's canvas is).
            for c in [0usize, 4] {
                for i in c..c + 3 {
                    block.f[i] = srgb_to_linear(block.f[i]);
                }
            }
        }
        block
    }

    /// A layer's buffers.
    fn upload(&self, device: &wgpu::Device, part: &Arc<StyledLayerPart>) -> GpuLayer {
        let layer = &part.layer;
        let mut batches = Vec::with_capacity(layer.batches.len());
        let mut blocks: Vec<u8> = Vec::with_capacity(layer.batches.len() * self.align as usize);
        for (i, b) in layer.batches.iter().enumerate() {
            let (pipe, floats, instanced) = match &b.kind {
                BatchKind::Stroke { .. } => (Pipe::Stroke, STROKE_FLOATS, true),
                BatchKind::Marker { .. } => (Pipe::Marker, MARKER_FLOATS, true),
                BatchKind::Fill { paint } => (
                    match paint {
                        FillPaintBatch::Solid { .. } => Pipe::Solid,
                        FillPaintBatch::Hatch { .. } => Pipe::Hatch,
                        FillPaintBatch::Gradient { .. } => Pipe::Gradient,
                        FillPaintBatch::Pattern { .. } => Pipe::Pattern,
                        FillPaintBatch::Tile { .. } => Pipe::Tile,
                        FillPaintBatch::Image { .. } => Pipe::Image,
                    },
                    AREA_FLOATS,
                    false,
                ),
            };
            let block = self.block(b);
            let offset = (i as u64 * self.align) as u32;
            blocks.extend_from_slice(&block.bytes());
            blocks.resize((i + 1) * self.align as usize, 0);
            let range = b.range.clone();
            let picture = match &b.kind {
                BatchKind::Fill {
                    paint: FillPaintBatch::Image { image, .. },
                } => Some(image.clone()),
                _ => None,
            };
            batches.push(GpuBatch {
                picture,
                pipe,
                bytes: (range.start as u64 * 4)..(range.end as u64 * 4),
                count: (range.len() / floats) as u32,
                instanced,
                block,
                offset,
                visible: false,
                placed: None,
            });
        }
        let vertex = (!layer.data.is_empty()).then(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("kentos.styled.data"),
                contents: bytemuck::cast_slice(&layer.data),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        if blocks.is_empty() {
            blocks.resize(self.align as usize, 0);
        }
        let styles = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("kentos.styled.styles"),
            contents: &blocks,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.styled.styles"),
            layout: &self.style_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &styles,
                    offset: 0,
                    size: wgpu::BufferSize::new(STYLE_BYTES as u64),
                }),
            }],
        });
        GpuLayer {
            id: part.id,
            source: part.clone(),
            vertex,
            styles,
            bind,
            batches,
            bytes: (layer.data.len() * 4 + blocks.len()) as u64,
        }
    }

    /// Before a frame: uploads layers the view has not drawn, drops the ones
    /// it no longer draws, decides what shows and places its images, writes
    /// the frame uniform. Returns whether an image waits for another frame.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &mut ViewStyled,
        scene: &StyledScene,
        frame: &StyledFrame,
        samples: u32,
        images: &dyn ImageSource,
    ) -> bool {
        self.ensure(device, samples);
        // Layers by id: kept ones stay, new ones are uploaded.
        let mut kept: HashMap<u64, GpuLayer> = view.layers.drain(..).map(|l| (l.id, l)).collect();
        for part in &scene.layers {
            let layer = kept
                .remove(&part.id)
                .unwrap_or_else(|| self.upload(device, part));
            view.layers.push(layer);
        }
        view.under = scene.under;
        view.order = scene.order.clone();
        let px_per_m = frame.scale * frame.dpr;
        let uniform = frame_uniform(frame);
        queue.write_buffer(&view.frame, 0, bytemuck::bytes_of(&uniform));
        view.uniform = uniform;
        let hw = f64::from(frame.size_px[0]) / 2.0 / px_per_m;
        let hh = f64::from(frame.size_px[1]) / 2.0 / px_per_m;
        let box_ = [
            frame.center[0] - hw,
            frame.center[1] - hh,
            frame.center[0] + hw,
            frame.center[1] + hh,
        ];
        self.atlas.begin_frame();
        // Twice at most: when the page starts over midway, every image is looked up again.
        for _ in 0..2 {
            let generation = self.atlas.generation;
            for layer in &mut view.layers {
                let batches = &layer.source.layer.batches;
                for (g, b) in layer.batches.iter_mut().zip(batches) {
                    g.visible = b.in_scale(frame.scale_denominator)
                        && b.in_view(box_, px_per_m, frame.dpr)
                        && b.legible(px_per_m, frame.dpr);
                    if !g.visible {
                        continue;
                    }
                    let Some(image) = b.image() else {
                        continue;
                    };
                    let Some(hit) =
                        self.atlas
                            .lookup(image, b.image_px(px_per_m, frame.dpr), images)
                    else {
                        g.visible = false;
                        continue;
                    };
                    if g.placed == Some((generation, hit.uv)) {
                        continue;
                    }
                    g.placed = Some((generation, hit.uv));
                    g.block.set_rect(hit.uv);
                    if matches!(b.kind, BatchKind::Marker { .. }) {
                        g.block.set_aspect(hit.aspect);
                    }
                    queue.write_buffer(&layer.styles, u64::from(g.offset), &g.block.bytes());
                }
            }
            if self.atlas.generation == generation {
                break;
            }
        }
        // The visible pictures' textures, each bound with the frame (docs/adr/0192 §3).
        let mut drawn = std::collections::HashSet::new();
        for layer in &view.layers {
            for g in &layer.batches {
                if let (true, Some(key)) = (g.visible, &g.picture) {
                    drawn.insert(key.clone());
                }
            }
        }
        for key in &drawn {
            let texture = self.pictures.view(device, queue, key, images);
            let fresh = view
                .pictures
                .get(key)
                .is_some_and(|(t, _)| Arc::ptr_eq(t, &texture));
            if !fresh {
                let bind = super::pictures::binding(
                    device,
                    &self.frame_layout,
                    &view.frame,
                    &texture,
                    &self.pictures.sampler,
                );
                view.pictures.insert(key.clone(), (texture, bind));
            }
        }
        view.pictures.retain(|k, _| drawn.contains(k));
        for up in self.atlas.take_uploads() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: up.x,
                        y: up.y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &up.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * up.width),
                    rows_per_image: Some(up.height),
                },
                wgpu::Extent3d {
                    width: up.width,
                    height: up.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.atlas.pending
    }

    /// Büyüteç (docs/adr/0181 §5): after [`StyledGpu::prepare`] of the same
    /// frame, the view's layers seen through a second camera, `frame`: its
    /// frame uniform in a buffer of its own, and the box it sees. What shows is
    /// the view's own choice (visibility, atlas images); single-sampled, into
    /// the host's pass.
    pub fn prepare_lens(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &mut ViewStyled,
        frame: &StyledFrame,
    ) {
        self.ensure(device, 1);
        let uniform = frame_uniform(frame);
        let (buffer, _) = view
            .lens
            .get_or_insert_with(|| frame_binding(device, self, "kentos.styled.lens"));
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&uniform));
        // The pictures the view binds, with the lens's frame too (docs/adr/0192 §3).
        let mut lens_pictures = std::mem::take(&mut view.lens_pictures);
        for (key, (texture, _)) in &view.pictures {
            let fresh = lens_pictures
                .get(key)
                .is_some_and(|(t, _)| Arc::ptr_eq(t, texture));
            if !fresh && let Some((buffer, _)) = &view.lens {
                let bind = super::pictures::binding(
                    device,
                    &self.frame_layout,
                    buffer,
                    texture,
                    &self.pictures.sampler,
                );
                lens_pictures.insert(key.clone(), (texture.clone(), bind));
            }
        }
        lens_pictures.retain(|k, _| view.pictures.contains_key(k));
        view.lens_pictures = lens_pictures;
        let px_per_m = frame.scale * frame.dpr;
        let hw = f64::from(frame.size_px[0]) / 2.0 / px_per_m;
        let hh = f64::from(frame.size_px[1]) / 2.0 / px_per_m;
        view.lens_view = Some((
            [
                frame.center[0] - hw,
                frame.center[1] - hh,
                frame.center[0] + hw,
                frame.center[1] + hh,
            ],
            px_per_m,
            frame.dpr,
        ));
    }

    /// Draws a view's visible batches, layer by layer in their order.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, view: &ViewStyled, samples: u32) {
        self.draw_through(pass, view, samples, &view.frame_bind, &view.pictures, None);
    }

    /// The same through the lens [`StyledGpu::prepare_lens`] set: only the
    /// batches that reach its box.
    pub fn draw_lens(&self, pass: &mut wgpu::RenderPass<'_>, view: &ViewStyled) {
        if let (Some((_, bind)), Some(seen)) = (&view.lens, view.lens_view) {
            self.draw_through(pass, view, 1, bind, &view.lens_pictures, Some(seen));
        }
    }

    fn draw_through(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        view: &ViewStyled,
        samples: u32,
        frame_bind: &wgpu::BindGroup,
        pictures: &HashMap<String, (Arc<wgpu::TextureView>, wgpu::BindGroup)>,
        seen: Option<([f64; 4], f64, f64)>,
    ) {
        let Some(pipes) = self.pipelines.get(&samples) else {
            return;
        };
        if view.layers.is_empty() {
            return;
        }
        pass.set_bind_group(0, frame_bind, &[]);
        let mut current: Option<Pipe> = None;
        let mut batch = |layer: &GpuLayer, i: usize| {
            let Some(vertex) = &layer.vertex else {
                return;
            };
            let Some(b) = layer.batches.get(i) else {
                return;
            };
            if !b.visible || b.count == 0 {
                return;
            }
            if let Some((box_, px_per_m, dpr)) = seen
                && !layer
                    .source
                    .layer
                    .batches
                    .get(i)
                    .is_some_and(|s| s.in_view(box_, px_per_m, dpr))
            {
                return;
            }
            // A picture draws with its own texture in group 0 (docs/adr/0192 §3).
            let picture = match &b.picture {
                Some(key) => match pictures.get(key) {
                    Some((_, bind)) => Some(bind),
                    None => return,
                },
                None => None,
            };
            if current != Some(b.pipe) {
                pass.set_pipeline(pipes.get(b.pipe));
                current = Some(b.pipe);
            }
            if let Some(bind) = picture {
                pass.set_bind_group(0, bind, &[]);
            }
            pass.set_bind_group(1, &layer.bind, &[b.offset]);
            pass.set_vertex_buffer(0, vertex.slice(b.bytes.clone()));
            if b.instanced {
                pass.draw(0..6, 0..b.count);
            } else {
                pass.draw(0..b.count, 0..1);
            }
            if picture.is_some() {
                pass.set_bind_group(0, frame_bind, &[]);
            }
        };
        match &view.order {
            Some(order) => {
                for &(l, b) in order.iter() {
                    if let Some(layer) = view.layers.get(l as usize) {
                        batch(layer, b as usize);
                    }
                }
            }
            None => {
                for layer in &view.layers {
                    for i in 0..layer.batches.len() {
                        batch(layer, i);
                    }
                }
            }
        }
    }
}

/// A frame's uniform: the camera centre in two float32 parts (docs/adr/0157
/// §2), so the shader takes it from each batch's tile without losing digits.
fn frame_uniform(frame: &StyledFrame) -> StyledFrameUniform {
    let hi = [frame.center[0] as f32, frame.center[1] as f32];
    StyledFrameUniform {
        offset: hi,
        offset_lo: [
            (frame.center[0] - f64::from(hi[0])) as f32,
            (frame.center[1] - f64::from(hi[1])) as f32,
        ],
        scale: [
            (2.0 * frame.scale / f64::from(frame.size_px[0].max(1.0)) * frame.dpr) as f32,
            (2.0 * frame.scale / f64::from(frame.size_px[1].max(1.0)) * frame.dpr) as f32,
        ],
        px_per_m: (frame.scale * frame.dpr) as f32,
        dpr: frame.dpr as f32,
        viewport: frame.size_px,
    }
}

/// A frame uniform's buffer and its bind group, the atlas beside it.
fn frame_binding(
    device: &wgpu::Device,
    gpu: &StyledGpu,
    label: &'static str,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let frame = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: std::mem::size_of::<StyledFrameUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: &gpu.frame_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&gpu.atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&gpu.sampler),
            },
        ],
    });
    (frame, bind)
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

struct GpuBatch {
    /// A picture's key (docs/adr/0192 §3): its texture is bound in group 0 for it.
    picture: Option<String>,
    pipe: Pipe,
    /// Bytes of the layer's vertex buffer this batch draws.
    bytes: Range<u64>,
    /// Vertices (fills) or instances (strokes, markers).
    count: u32,
    instanced: bool,
    block: StyleBlock,
    /// Its block's offset in the layer's uniform buffer.
    offset: u32,
    visible: bool,
    /// The atlas generation and rectangle last written into its block.
    placed: Option<(u64, [f32; 4])>,
}

/// A styled layer on the GPU.
pub struct GpuLayer {
    id: u64,
    source: Arc<StyledLayerPart>,
    vertex: Option<wgpu::Buffer>,
    styles: wgpu::Buffer,
    bind: wgpu::BindGroup,
    batches: Vec<GpuBatch>,
    bytes: u64,
}

/// A view's styled state: its frame uniform and its layers on the GPU.
pub struct ViewStyled {
    frame: wgpu::Buffer,
    frame_bind: wgpu::BindGroup,
    /// What `frame` holds.
    uniform: StyledFrameUniform,
    pub(crate) layers: Vec<GpuLayer>,
    /// How many of the view's plain parts draw beneath the styled layers.
    pub under: usize,
    /// The scene's draw order, when it has one (`StyledScene::order`).
    order: Option<Arc<Vec<(u32, u32)>>>,
    /// Büyüteç's frame uniform and what it sees: its box, device pixels per
    /// metre and pixel ratio (docs/adr/0181 §5).
    lens: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    lens_view: Option<([f64; 4], f64, f64)>,
    /// Each drawn picture bound with the frame uniform, and with the lens's.
    pictures: HashMap<String, (Arc<wgpu::TextureView>, wgpu::BindGroup)>,
    lens_pictures: HashMap<String, (Arc<wgpu::TextureView>, wgpu::BindGroup)>,
}

impl ViewStyled {
    pub fn new(device: &wgpu::Device, gpu: &StyledGpu) -> ViewStyled {
        let (frame, frame_bind) = frame_binding(device, gpu, "kentos.styled.frame");
        ViewStyled {
            frame,
            frame_bind,
            uniform: StyledFrameUniform::default(),
            layers: Vec::new(),
            under: 0,
            order: None,
            lens: None,
            lens_view: None,
            pictures: HashMap::new(),
            lens_pictures: HashMap::new(),
        }
    }

    /// What the last prepared frame of these layers draws: the layers, the
    /// batches shown and their images' places, the frame. The same key, the
    /// same pixels (a kept picture, renderer.rs).
    pub(crate) fn key(&self, hasher: &mut impl Hasher) {
        self.under.hash(hasher);
        self.order.as_deref().hash(hasher);
        bytemuck::bytes_of(&self.uniform).hash(hasher);
        for layer in &self.layers {
            layer.id.hash(hasher);
            for b in &layer.batches {
                b.visible.hash(hasher);
                if let Some((generation, uv)) = b.placed {
                    generation.hash(hasher);
                    uv.map(f32::to_bits).hash(hasher);
                }
            }
        }
    }

    /// Batches drawn in the last frame, and the bytes the layers hold on the GPU.
    pub fn counts(&self) -> (u32, u64) {
        let drawn = self
            .layers
            .iter()
            .flat_map(|l| l.batches.iter())
            .filter(|b| b.visible && b.count > 0)
            .count() as u32;
        (drawn, self.layers.iter().map(|l| l.bytes).sum())
    }
}
