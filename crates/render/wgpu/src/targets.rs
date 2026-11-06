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
//! - **A kept picture:** a view that keeps its picture between frames (the
//!   desktop's drawing area) draws the scene again only when what it shows
//!   changed; otherwise the resolved texture it holds is composed as it is
//!   (renderer.rs). While the view is panned, the kept picture moved by the
//!   pan's whole device pixels is the new one but for the strips the pan
//!   uncovered, drawn alone (docs/adr/1067): `scratch` holds the moved part
//!   meanwhile, and `held` says what and where the picture shows.
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
use std::sync::{Mutex, PoisonError};
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
    /// The zoomed frames' layout (docs/adr/1068): the source picture, a linear sampler, the sampling.
    pub(crate) zoom_layout: wgpu::BindGroupLayout,
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
            zoom_layout: crate::zoomed::layout(device),
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
    /// The resolved picture: composed into the host's frame, read by a layer's mix
    /// (docs/adr/0226 §4, copied from).
    pub(crate) resolve_texture: wgpu::Texture,
    pub(crate) resolve: wgpu::TextureView,
    /// The moved part of the kept picture while the strips a pan uncovered are drawn (docs/adr/1067 §2); while the
    /// wheel zooms, the last sharp picture the zoomed frames sample (docs/adr/1068).
    pub(crate) scratch: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    /// The zoomed frames' sampling of `scratch` (docs/adr/1068): its uniform and bind group.
    pub(crate) zoom_uniform: wgpu::Buffer,
    pub(crate) zoom_bind: wgpu::BindGroup,
    /// A wider sharp picture kept beside the source, for the ring a zoom out uncovers, and its sampling
    /// (docs/adr/1068 §2).
    pub(crate) wide: wgpu::Texture,
    pub(crate) wide_uniform: wgpu::Buffer,
    pub(crate) wide_bind: wgpu::BindGroup,
    /// Bytes of the textures, for the frame's statistics and the settings window.
    pub bytes: u64,
    /// The key of the picture the resolved texture holds, once drawn
    /// (renderer.rs, a kept picture); new targets hold none.
    kept: Mutex<Option<u64>>,
    /// What the kept picture shows but for the camera's place (a key of its content) and where it was drawn; none
    /// for a picture a pan may not move (docs/adr/1067 §2).
    held: Mutex<Option<(u64, Place)>>,
    /// What the kept picture shows but for the camera (the host's key) and where, when it is sharp (drawn whole or
    /// moved by whole pixels) and may be scaled while the wheel zooms; and the same of the sharp picture `scratch`
    /// holds (docs/adr/1068).
    sharp: Mutex<Option<(u64, Place)>>,
    source: Mutex<Option<(u64, Place)>>,
    wide_at: Mutex<Option<(u64, Place)>>,
}

/// Where a picture was drawn (docs/adr/1067 §2): the camera's centre from the origin, the origin, the picture pixels a
/// world unit, the picture's size and its turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub center: [f64; 2],
    pub origin: [f64; 2],
    pub px_per_unit: f64,
    pub size: [u32; 2],
    pub twist: f64,
    /// Device pixels a logical pixel: a zoom changes `px_per_unit`, a screen of another density this (docs/adr/1068).
    pub dpr: f64,
}

impl Place {
    /// The same frame but for the camera's centre and scale (docs/adr/1068): origin, size and device pixel ratio
    /// to the bit, neither turned.
    pub fn same_frame(&self, now: &Place) -> bool {
        self.origin.map(f64::to_bits) == now.origin.map(f64::to_bits)
            && self.size == now.size
            && self.dpr.to_bits() == now.dpr.to_bits()
            && self.twist == 0.0
            && now.twist == 0.0
    }

    /// The whole picture pixels a picture drawn at `self` moves by to be the picture at `now` (x right, y down); none
    /// when anything but the centre differs, the view is turned, or the move is not whole pixels (to a thousandth),
    /// is nothing, or uncovers the whole picture.
    pub fn shift_to(&self, now: &Place) -> Option<[i32; 2]> {
        let same = self.same_frame(now) && self.px_per_unit.to_bits() == now.px_per_unit.to_bits();
        if !same || !(self.px_per_unit > 0.0) {
            return None;
        }
        let dx = (self.center[0] - now.center[0]) * self.px_per_unit;
        let dy = (now.center[1] - self.center[1]) * self.px_per_unit;
        let (rx, ry) = (dx.round(), dy.round());
        let whole = (dx - rx).abs() < 1e-3 && (dy - ry).abs() < 1e-3;
        let inside = rx.abs() < f64::from(self.size[0]) && ry.abs() < f64::from(self.size[1]);
        (whole && inside && (rx != 0.0 || ry != 0.0)).then_some([rx as i32, ry as i32])
    }
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
        let resolve_texture = device.create_texture(&texture_descriptor(
            "kentos.cad2d.picture",
            size,
            1,
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
        ));
        let resolve = resolve_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let scratch = device.create_texture(&texture_descriptor(
            "kentos.cad2d.picture.moved",
            size,
            1,
            format,
            wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        ));
        let wide = device.create_texture(&texture_descriptor(
            "kentos.cad2d.picture.wide",
            size,
            1,
            format,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        ));
        let sampled = |texture: &wgpu::Texture| {
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kentos.cad2d.picture.zoomed"),
                size: std::mem::size_of::<crate::zoomed::ZoomUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("kentos.cad2d.picture.zoomed"),
                layout: &compose.zoom_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&compose.smooth) },
                    wgpu::BindGroupEntry { binding: 2, resource: uniform.as_entire_binding() },
                ],
            });
            (uniform, bind)
        };
        let (zoom_uniform, zoom_bind) = sampled(&scratch);
        let (wide_uniform, wide_bind) = sampled(&wide);
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
            resolve_texture,
            resolve,
            scratch,
            bind_group,
            zoom_uniform,
            zoom_bind,
            wide,
            wide_uniform,
            wide_bind,
            // The multisampled samples, the picture, the moved part and the wide picture.
            bytes: pixels * texel * (u64::from(samples.max(1)) + u64::from(samples > 1) + 2),
            kept: Mutex::new(None),
            held: Mutex::new(None),
            sharp: Mutex::new(None),
            source: Mutex::new(None),
            wide_at: Mutex::new(None),
        }
    }

    /// The wide picture's host key and place (docs/adr/1068 §2).
    pub fn wide_at(&self) -> Option<(u64, Place)> {
        *self.wide_at.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_wide_at(&self, wide: Option<(u64, Place)>) {
        *self.wide_at.lock().unwrap_or_else(PoisonError::into_inner) = wide;
    }

    /// The kept picture's host key and place while it is sharp (docs/adr/1068).
    pub fn sharp(&self) -> Option<(u64, Place)> {
        *self.sharp.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_sharp(&self, sharp: Option<(u64, Place)>) {
        *self.sharp.lock().unwrap_or_else(PoisonError::into_inner) = sharp;
    }

    /// The sharp picture `scratch` holds for zoomed frames, and where (docs/adr/1068).
    pub fn source(&self) -> Option<(u64, Place)> {
        *self.source.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_source(&self, source: Option<(u64, Place)>) {
        *self.source.lock().unwrap_or_else(PoisonError::into_inner) = source;
    }

    /// What the kept picture shows but for the camera's place, and where (docs/adr/1067 §2).
    pub fn held(&self) -> Option<(u64, Place)> {
        *self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The kept picture shows `content` drawn at `place`; none: a pan may not move it.
    pub fn hold(&self, held: Option<(u64, Place)>) {
        *self.held.lock().unwrap_or_else(PoisonError::into_inner) = held;
    }

    /// Whether the resolved texture holds the picture of `key`.
    pub fn holds(&self, key: u64) -> bool {
        *self.kept.lock().unwrap_or_else(PoisonError::into_inner) == Some(key)
    }

    /// The resolved texture now holds the picture of `key` (drawn this frame).
    pub fn keep(&self, key: u64) {
        *self.kept.lock().unwrap_or_else(PoisonError::into_inner) = Some(key);
    }

    /// The attachment of one of a composed frame's passes into the picture
    /// (docs/adr/0226): cleared to `clear` or loaded, the multisampled texture
    /// kept for the next pass, the picture resolved at its end when `resolve`.
    pub fn attachment_kept(
        &self,
        clear: Option<wgpu::Color>,
        resolve: bool,
    ) -> wgpu::RenderPassColorAttachment<'_> {
        let load = clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear);
        match &self.msaa {
            Some(msaa) => wgpu::RenderPassColorAttachment {
                view: msaa,
                depth_slice: None,
                // Resolved only when the picture is read: a pass between others keeps its samples alone.
                resolve_target: resolve.then_some(&self.resolve),
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

#[cfg(test)]
mod place_tests {
    use super::Place;

    fn at(x: f64, y: f64) -> Place {
        Place { center: [x, y], origin: [486_000.0, 4_420_000.0], px_per_unit: 2.0, size: [100, 80], twist: 0.0, dpr: 2.0 }
    }

    /// A pan by whole pixels moves the picture (x right, y down); a fraction, a turn, another scale or a move past
    /// the picture does not (docs/adr/1067 §1).
    #[test]
    fn only_a_pan_by_whole_pixels_moves_the_picture() {
        // The camera 4.5 m west and 2 m north: the picture moves 9 px right and 4 px down.
        assert_eq!(at(10.0, 5.0).shift_to(&at(5.5, 7.0)), Some([9, 4]));
        assert_eq!(at(10.0, 5.0).shift_to(&at(10.25, 5.0)), None, "half a pixel");
        assert_eq!(at(10.0, 5.0).shift_to(&at(10.0, 5.0)), None, "nothing to move");
        assert_eq!(at(10.0, 5.0).shift_to(&at(10.0 - 60.0, 5.0)), None, "past the picture");
        let turned = Place { twist: 0.1, ..at(5.5, 7.0) };
        assert_eq!(at(10.0, 5.0).shift_to(&turned), None);
        let scaled = Place { px_per_unit: 2.5, ..at(5.5, 7.0) };
        assert_eq!(at(10.0, 5.0).shift_to(&scaled), None);
    }
}
