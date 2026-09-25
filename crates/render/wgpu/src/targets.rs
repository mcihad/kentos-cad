//! A view's own targets (TODOS.md AA-01, AA-02; docs/adr/0023): what a view
//! draws into when the host's single-sampled pass is not enough.
//!
//! - **Multisampling:** the view draws into a multisampled colour texture
//!   that resolves into a single-sampled one of the same size.
//! - **Resolution:** with `hi_dpi` off the view draws one pixel per logical
//!   pixel and is scaled up when composed.
//! - **Composition:** a full-view triangle samples the resolved texture into
//!   the host's frame, inside the drawing area's viewport and scissor: a
//!   GPU-to-GPU draw, nothing read back to the CPU (REN-02 holds).
//!
//! The textures are made for a size and a sample count and made again when
//! either changes. A replaced texture is dropped: wgpu keeps it alive until
//! the submissions that use it have finished, so nothing in flight loses
//! its target.
//!
//! Which sample counts a format takes is asked of the device itself
//! ([`probe_sample_counts`]): each candidate texture is created once, 1 × 1,
//! inside a validation scope.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

/// Counts beyond 1 a view may ask for; the device says which it takes.
pub const CANDIDATE_SAMPLES: [u32; 4] = [2, 4, 8, 16];

/// The composition shader: a full-view triangle that samples the resolved picture.
const COMPOSE_WGSL: &str = r"
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var picture_sampler: sampler;

struct Out {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex fn vs(@builtin(vertex_index) i: u32) -> Out {
    let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.position = vec4f(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2f(p.x, 1.0 - p.y);
    return out;
}

@fragment fn fs(input: Out) -> @location(0) vec4f {
    return textureSampleLevel(picture, picture_sampler, input.uv, 0.0);
}
";

/// The composition shader's source, for the WGSL checks.
pub fn compose_source() -> &'static str {
    COMPOSE_WGSL
}

/// The sample counts a colour target of `format` takes on `device`,
/// ascending, 1 first (TODOS.md AA-01). Iced's device asks for no
/// adapter-specific format features, so the WebGPU guarantees apply there
/// (1 and 4); a device that has them answers with its own set.
pub fn probe_sample_counts(device: &wgpu::Device, format: wgpu::TextureFormat) -> Vec<u32> {
    let mut counts = vec![1];
    for samples in CANDIDATE_SAMPLES {
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let texture = device.create_texture(&texture_descriptor(
            "kentos.cad2d.probe",
            [1, 1],
            samples,
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        ));
        let refused = pop_error_now(device).is_some();
        texture.destroy();
        if !refused {
            counts.push(samples);
        }
    }
    counts
}

/// The count a view draws with: the largest supported one not above
/// `requested`, else the smallest (the rule of the settings' device
/// constraint, `kentos_contracts::nearest_allowed`).
pub fn supported_samples(counts: &[u32], requested: u32) -> u32 {
    counts
        .iter()
        .copied()
        .filter(|&c| c <= requested)
        .max()
        .or_else(|| counts.iter().copied().min())
        .unwrap_or(1)
}

/// wgpu's answer to the innermost error scope, taken at once: native
/// backends answer synchronously, so the future is ready on its first poll.
pub(crate) fn pop_error_now(device: &wgpu::Device) -> Option<wgpu::Error> {
    let future = pin!(device.pop_error_scope());
    match future.poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(error) => error,
        Poll::Pending => None,
    }
}

fn texture_descriptor(
    label: &'static str,
    size: [u32; 2],
    samples: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
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
    }
}

/// The composition pipeline and its samplers, one per renderer.
pub struct Compose {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    /// Pixel for pixel (the picture is the area's size).
    exact: wgpu::Sampler,
    /// Scaled up (one pixel per logical pixel on a denser screen).
    smooth: wgpu::Sampler,
}

impl Compose {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.cad2d.compose"),
            source: wgpu::ShaderSource::Wgsl(COMPOSE_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.cad2d.compose"),
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
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.cad2d.compose"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kentos.cad2d.compose"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                // The picture is opaque (its background covers it): it replaces what is under it.
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let sampler = |label, filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                ..wgpu::SamplerDescriptor::default()
            })
        };
        Self {
            pipeline,
            layout,
            exact: sampler("kentos.cad2d.compose.exact", wgpu::FilterMode::Nearest),
            smooth: sampler("kentos.cad2d.compose.smooth", wgpu::FilterMode::Linear),
        }
    }

    /// Draws `targets`' picture into `pass`, whose viewport and scissor are the area's.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, targets: &Targets) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &targets.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// A view's targets for one size and sample count.
pub struct Targets {
    /// Pixels of the picture.
    pub size: [u32; 2],
    pub samples: u32,
    /// Scaled up when composed (the picture is smaller than the area).
    pub scaled: bool,
    msaa: Option<wgpu::TextureView>,
    resolve: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    /// Bytes of the textures, for the frame's statistics and the settings window.
    pub bytes: u64,
}

impl Targets {
    /// Makes the textures. The caller wraps this in error scopes: a device
    /// out of memory refuses them, and the view goes back to what worked.
    pub fn new(
        device: &wgpu::Device,
        compose: &Compose,
        format: wgpu::TextureFormat,
        size: [u32; 2],
        samples: u32,
        scaled: bool,
    ) -> Self {
        let msaa = (samples > 1).then(|| {
            device
                .create_texture(&texture_descriptor(
                    "kentos.cad2d.msaa",
                    size,
                    samples,
                    format,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                ))
                .create_view(&wgpu::TextureViewDescriptor::default())
        });
        let resolve = device
            .create_texture(&texture_descriptor(
                "kentos.cad2d.picture",
                size,
                1,
                format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ))
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.cad2d.compose"),
            layout: &compose.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&resolve),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(if scaled {
                        &compose.smooth
                    } else {
                        &compose.exact
                    }),
                },
            ],
        });
        let texel = u64::from(format.block_copy_size(None).unwrap_or(4));
        let pixels = u64::from(size[0]) * u64::from(size[1]);
        Self {
            size,
            samples,
            scaled,
            msaa,
            resolve,
            bind_group,
            bytes: pixels * texel * (u64::from(samples.max(1)) + u64::from(samples > 1)),
        }
    }

    /// The colour attachment the view's own pass draws into: the multisampled
    /// texture resolving into the picture, or the picture itself.
    pub fn attachment(&self, clear: wgpu::Color) -> wgpu::RenderPassColorAttachment<'_> {
        match &self.msaa {
            Some(msaa) => wgpu::RenderPassColorAttachment {
                view: msaa,
                depth_slice: None,
                resolve_target: Some(&self.resolve),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    // Only the resolved picture is kept.
                    store: wgpu::StoreOp::Discard,
                },
            },
            None => wgpu::RenderPassColorAttachment {
                view: &self.resolve,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            },
        }
    }
}
