//! A view drawn in targets of its own and laid over the window (docs/adr/8120). The host's frame is one sample a
//! pixel; a view of a quality that asks for more draws at four into its own colour and depth, resolves them, and the
//! composite lays the result over the window premultiplied (the sky the host drew under the view shows where nothing
//! is). At the highest quality the depth the opaque faces left is kept and read once more: the ambient light's
//! occlusion, one value a pixel, darkens the colour as it is laid over, lifted by the veil the see-through faces left
//! (their alpha, in a second target the faces write).
//!
//! The targets are the view's size in pixels and made again when it changes; a view partly off the window lays the
//! part on it (`place`'s corners).

use bytemuck::{Pod, Zeroable};

use crate::{DEPTH_CLEAR, DEPTH_FORMAT, Quality};

/// The composite's and the occlusion's WGSL (`shaders/wgsl/scene3d/`).
pub const COMPOSITE_WGSL: &str = include_str!("../../../../shaders/wgsl/scene3d/composite.wgsl");
pub const OCCLUSION_WGSL: &str = include_str!("../../../../shaders/wgsl/scene3d/occlusion.wgsl");
const OCCLUSION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
/// The veil's format: the faces' second target at the highest quality.
pub const VEIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
/// The occlusion's radius (m) and strength: a room's corner, a column against its wall, a sill's shade.
pub const OCCLUSION_RADIUS: f64 = 0.6;
pub const OCCLUSION_STRENGTH: f32 = 0.6;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PlaceBlock {
    uv: [f32; 4],
    shade: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct OcclusionBlock {
    pub(crate) lens: [f32; 4],
    pub(crate) shape: [f32; 4],
}

/// The composite's pipeline and, at the highest quality, the occlusion's; their blocks.
pub(crate) struct Passes {
    composite: wgpu::RenderPipeline,
    composite_layout: wgpu::BindGroupLayout,
    occlusion: Option<(wgpu::RenderPipeline, wgpu::BindGroupLayout)>,
    place: wgpu::Buffer,
    pub(crate) lens: wgpu::Buffer,
}

fn uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(32),
        },
        count: None,
    }
}

fn texture(
    binding: u32,
    sample_type: wgpu::TextureSampleType,
    multisampled: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled,
        },
        count: None,
    }
}

/// A pipeline of one triangle over the viewport.
fn full(
    device: &wgpu::Device,
    label: &str,
    source: &str,
    entry: (&str, &str),
    layout: &wgpu::BindGroupLayout,
    target: wgpu::ColorTargetState,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some(entry.0),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some(entry.1),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(target)],
        }),
        multiview: None,
        cache: None,
    })
}

impl Passes {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        quality: Quality,
    ) -> Passes {
        let float = wgpu::TextureSampleType::Float { filterable: false };
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.scene3d.composite"),
            entries: &[
                texture(0, float, false),
                texture(1, float, false),
                uniform(2),
                texture(3, float, false),
            ],
        });
        let composite = full(
            device,
            "kentos.scene3d.composite",
            COMPOSITE_WGSL,
            ("compositeVs", "compositeFs"),
            &composite_layout,
            wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            },
        );
        let occlusion = quality.occlusion().then(|| {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("kentos.scene3d.occlusion"),
                entries: &[texture(0, wgpu::TextureSampleType::Depth, true), uniform(1)],
            });
            let pipeline = full(
                device,
                "kentos.scene3d.occlusion",
                OCCLUSION_WGSL,
                ("occlusionVs", "occlusionFs"),
                &layout,
                wgpu::ColorTargetState {
                    format: OCCLUSION_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                },
            );
            (pipeline, layout)
        });
        let block = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: 32,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Passes {
            composite,
            composite_layout,
            occlusion,
            place: block("kentos.scene3d.place"),
            lens: block("kentos.scene3d.lens"),
        }
    }

    /// Where the viewport lays the target (its corners, 0–1) and whether the occlusion darkens it.
    pub(crate) fn place(&self, queue: &wgpu::Queue, uv: [f32; 4], occluded: bool) {
        let block = PlaceBlock {
            uv,
            shade: [if occluded { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.place, 0, bytemuck::bytes_of(&block));
    }

    pub(crate) fn lens(&self, queue: &wgpu::Queue, block: OcclusionBlock) {
        queue.write_buffer(&self.lens, 0, bytemuck::bytes_of(&block));
    }
}

/// A view's targets at its size: the samples' colour and depth, the resolved colour, the occlusion and the veil (its
/// samples and resolved).
pub(crate) struct Targets {
    pub(crate) size: (u32, u32),
    color: wgpu::TextureView,
    resolved: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
    veil: Option<(wgpu::TextureView, wgpu::TextureView)>,
    occlusion: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    composite: wgpu::BindGroup,
}

fn target(
    device: &wgpu::Device,
    label: &str,
    size: (u32, u32),
    samples: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
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
}

impl Targets {
    /// The targets for a view of `width` × `height` pixels at `quality`, kept when they already are that size.
    pub(crate) fn ensure(
        this: &mut Option<Targets>,
        device: &wgpu::Device,
        passes: &Passes,
        format: wgpu::TextureFormat,
        quality: Quality,
        width: u32,
        height: u32,
    ) {
        let size = (width.max(1), height.max(1));
        if this.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let samples = quality.samples();
        let draw = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let read = wgpu::TextureUsages::TEXTURE_BINDING;
        let color = target(
            device,
            "kentos.scene3d.colour",
            size,
            samples,
            format,
            if samples > 1 { draw } else { draw | read },
        );
        let resolved = (samples > 1).then(|| {
            target(
                device,
                "kentos.scene3d.resolved",
                size,
                1,
                format,
                draw | read,
            )
        });
        let occluded = passes.occlusion.is_some() && samples > 1;
        let depth = target(
            device,
            "kentos.scene3d.depth",
            size,
            samples,
            DEPTH_FORMAT,
            if occluded { draw | read } else { draw },
        );
        let occlusion = passes
            .occlusion
            .as_ref()
            .filter(|_| occluded)
            .map(|(_, layout)| {
                let view = target(
                    device,
                    "kentos.scene3d.occlusion",
                    size,
                    1,
                    OCCLUSION_FORMAT,
                    draw | read,
                );
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kentos.scene3d.occlusion"),
                    layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&depth),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: passes.lens.as_entire_binding(),
                        },
                    ],
                });
                (view, bind)
            });
        let veil = occlusion.as_ref().map(|_| {
            (
                target(
                    device,
                    "kentos.scene3d.veil",
                    size,
                    samples,
                    VEIL_FORMAT,
                    draw,
                ),
                target(
                    device,
                    "kentos.scene3d.veilResolved",
                    size,
                    1,
                    VEIL_FORMAT,
                    draw | read,
                ),
            )
        });
        let shown = resolved.as_ref().unwrap_or(&color);
        // Without an occlusion the colour stands in for it and for the veil: the composite never reads them then.
        let shade = occlusion.as_ref().map_or(shown, |(view, _)| view);
        let lifted = veil.as_ref().map_or(shown, |(_, resolved)| resolved);
        let composite = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.scene3d.composite"),
            layout: &passes.composite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(shown),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(shade),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: passes.place.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(lifted),
                },
            ],
        });
        *this = Some(Targets {
            size,
            color,
            resolved,
            depth,
            veil,
            occlusion,
            composite,
        });
    }

    /// Whether the occlusion is drawn.
    pub(crate) fn occludes(&self) -> bool {
        self.occlusion.is_some()
    }

    /// The scene's pass: the view's colour cleared see-through, its veil none, its depth cleared far.
    pub(crate) fn scene_pass<'e>(
        &self,
        encoder: &'e mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'e> {
        let colour = Some(wgpu::RenderPassColorAttachment {
            view: &self.color,
            depth_slice: None,
            resolve_target: self.resolved.as_ref(),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: if self.resolved.is_some() {
                    wgpu::StoreOp::Discard
                } else {
                    wgpu::StoreOp::Store
                },
            },
        });
        let veil = self
            .veil
            .as_ref()
            .map(|(samples, resolved)| wgpu::RenderPassColorAttachment {
                view: samples,
                depth_slice: None,
                resolve_target: Some(resolved),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Discard,
                },
            });
        let attachments = [colour, veil];
        let used = if self.veil.is_some() { 2 } else { 1 };
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kentos.scene3d"),
            color_attachments: &attachments[..used],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(DEPTH_CLEAR),
                    store: if self.occlusion.is_some() {
                        wgpu::StoreOp::Store
                    } else {
                        wgpu::StoreOp::Discard
                    },
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    /// The occlusion (when drawn), then the view laid over `output` at `viewport` within `scissor`.
    pub(crate) fn finish(
        &self,
        passes: &Passes,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        viewport: [f32; 4],
        scissor: [u32; 4],
    ) {
        if let (Some((view, bind)), Some((pipeline, _))) = (&self.occlusion, &passes.occlusion) {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("kentos.scene3d.occlusion"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kentos.scene3d.composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let [vx, vy, vw, vh] = viewport;
        pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
        pass.set_scissor_rect(
            scissor[0],
            scissor[1],
            scissor[2] - scissor[0],
            scissor[3] - scissor[1],
        );
        pass.set_pipeline(&passes.composite);
        pass.set_bind_group(0, &self.composite, &[]);
        pass.draw(0..3, 0..1);
    }
}
