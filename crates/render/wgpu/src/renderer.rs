//! Device resources of the drawing pipeline (TODOS.md REN-01, REN-02,
//! AA-01, AA-02): the pipelines of the shared WGSL for each sample count, a
//! frame uniform per view, the scene cache and a view's own targets.
//!
//! Who owns what (REN-02), as the desktop uses it from Iced's shader widget:
//! - The device and queue are the host's (Iced's). Nothing here opens a
//!   device, so the drawing and the interface share one GPU context and one
//!   frame.
//! - Single-sampled at full resolution (the default), a view draws straight
//!   into the host's render pass ([`Renderer::draw`]): its colour target is
//!   the window's frame, its viewport the drawing area, its scissor the
//!   area's visible part; the renderer never changes them.
//! - Multisampled, or at one pixel per logical pixel on a denser screen
//!   (`RenderSettings::samples`, `hi_dpi`), a view draws into its own targets
//!   and composes the resolved picture into the host's frame
//!   ([`Renderer::render`], `targets`): GPU to GPU, nothing read back.
//! - No depth buffer: a 2D drawing is drawn in order (fills, then strokes,
//!   then marks, each bottom layer first, as the web does).
//! - DPI and resize: the host gives the area's size in device pixels and the
//!   scale factor with every frame.
//!
//! Sample counts (AA-01): the device is asked which counts the host's format
//! takes ([`targets::probe_sample_counts`]); a view asked for another count
//! draws with the nearest one below. Pipelines are built per count and kept.
//! When a view's targets or a count's pipelines cannot be made (the device is
//! out of memory), the view draws on with the last count that worked and the
//! failure is kept for the host to report (AA-02).
//!
//! The scene cache: a view keeps the GPU buffers of the scene parts it drew
//! last, by part id. A frame with the same parts uploads only its 64-byte
//! uniform; a new part is uploaded once, in chunks the device accepts, and
//! the buffers it replaces are released. A view left undrawn for
//! [`KEEP_IDLE_FRAMES`] frames is dropped with its buffers and targets
//! ([`Renderer::trim`]).

use std::collections::HashMap;
use std::fmt;
use std::ops::Range;

use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::Vec2;
use crate::camera::Camera;
use crate::layout::{self, FrameUniform, PipelineSpec};
use crate::scene::{LayerRanges, ScenePart};
use crate::settings::RenderSettings;
use crate::shader;
use crate::stats::FrameStats;
use crate::targets::{self, Compose, Targets, pop_error_now, supported_samples};

/// A drawing area's key: its uniform and cached buffers are its own.
pub type ViewId = u64;

/// Largest buffer the scene cache makes; a larger part is split.
const MAX_CHUNK_BYTES: u64 = 64 << 20;
/// Frames a view may go undrawn before its buffers are released.
pub const KEEP_IDLE_FRAMES: u32 = 2;

/// The pipeline could not be built or a part not uploaded; the text is wgpu's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderError(pub String);

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}

/// A sample count a view could not draw with (AA-02): it draws with `working`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleFailure {
    pub requested: u32,
    pub working: u32,
    /// wgpu's reason.
    pub error: String,
}

/// What a frame of a view needs besides its scene parts.
#[derive(Clone, Copy, Debug)]
pub struct FrameInput<'a> {
    pub camera: &'a Camera,
    /// The origin the parts' offsets are from.
    pub origin: Vec2,
    /// The drawing area in device pixels.
    pub size_px: [f32; 2],
    /// Where the area's top-left corner is in the host's frame, device
    /// pixels: where a view with its own targets composes its picture.
    pub origin_px: [f32; 2],
    /// Device pixels per logical pixel.
    pub scale_factor: f64,
    pub settings: &'a RenderSettings,
}

/// The pipelines of one sample count.
struct Pipelines {
    background: wgpu::RenderPipeline,
    fill: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    marker: wgpu::RenderPipeline,
}

pub struct Renderer {
    format: wgpu::TextureFormat,
    srgb_target: bool,
    module: wgpu::ShaderModule,
    frame_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    /// By sample count; the host's (1) is built with the renderer.
    pipelines: HashMap<u32, Pipelines>,
    /// The counts the device takes for the host's format, ascending, 1 first.
    sample_counts: Vec<u32>,
    compose: Compose,
    views: HashMap<ViewId, View>,
    chunk_bytes: u64,
}

impl fmt::Debug for Renderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Renderer")
            .field("srgb_target", &self.srgb_target)
            .field("sample_counts", &self.sample_counts)
            .field("views", &self.views.len())
            .finish_non_exhaustive()
    }
}

impl Renderer {
    /// Builds the pipelines for targets of `format` on the host's device and
    /// asks the device which sample counts the format takes.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self, RenderError> {
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.cad2d"),
            source: wgpu::ShaderSource::Wgsl(shader::source().into()),
        });
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.cad2d.frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        std::mem::size_of::<FrameUniform>() as u64
                    ),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.cad2d"),
            bind_group_layouts: &[&frame_layout],
            push_constant_ranges: &[],
        });
        let host = build_pipelines(device, &module, &pipeline_layout, format, 1);
        let compose = Compose::new(device, format);
        if let Some(error) = pop_error_now(device) {
            return Err(RenderError(error.to_string()));
        }
        Ok(Self {
            format,
            srgb_target: format.is_srgb(),
            module,
            frame_layout,
            pipeline_layout,
            pipelines: HashMap::from([(1, host)]),
            sample_counts: targets::probe_sample_counts(device, format),
            compose,
            views: HashMap::new(),
            chunk_bytes: device.limits().max_buffer_size.min(MAX_CHUNK_BYTES),
        })
    }

    /// The sample counts this device takes for the host's format, ascending,
    /// 1 first (TODOS.md AA-01).
    pub fn sample_counts(&self) -> &[u32] {
        &self.sample_counts
    }

    /// Tests only: counts to believe instead of the device's, so a count the
    /// device refuses can be asked for and the fallback seen (AA-02).
    #[doc(hidden)]
    pub fn believe_sample_counts(&mut self, counts: Vec<u32>) {
        self.sample_counts = counts;
    }

    /// Before a frame of `view`: uploads the parts the view has not drawn
    /// yet (by id, in slot order), makes its own targets when it needs them
    /// (a new size or sample count; see [`Renderer::render`]) and writes the
    /// frame uniform. Parts it drew last time cost nothing. An upload the
    /// device refuses (out of memory) leaves that part undrawn and is
    /// reported once; targets it refuses bring back the last count that
    /// worked (`sample_failure`).
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: ViewId,
        parts: &[&ScenePart],
        frame: &FrameInput<'_>,
    ) -> Result<(), RenderError> {
        let frame_layout = &self.frame_layout;
        let chunk_bytes = self.chunk_bytes;
        let state = self
            .views
            .entry(view)
            .or_insert_with(|| View::new(device, frame_layout));
        state.idle = 0;
        state.frames += 1;

        let mut uploaded = std::mem::size_of::<FrameUniform>() as u64;
        let mut failure = None;
        state.parts.truncate(parts.len());
        for (slot, part) in parts.iter().enumerate() {
            if state.parts.get(slot).is_some_and(|p| p.id == part.id) {
                continue;
            }
            device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
            let mut gpu = GpuPart::upload(device, part, chunk_bytes);
            if let Some(error) = pop_error_now(device) {
                // Kept by id, empty: the refused part is not tried again every frame.
                gpu = GpuPart::empty(part.id);
                failure = Some(RenderError(format!(
                    "çizim verisi GPU'ya yüklenemedi ({} bayt): {error}",
                    part.byte_size()
                )));
            }
            uploaded += gpu.bytes;
            match state.parts.get_mut(slot) {
                Some(old) => *old = gpu,
                None => state.parts.push(gpu),
            }
        }

        // What the picture is drawn at: the host's pass, or the view's own targets.
        let settings = frame.settings;
        let asked = supported_samples(&self.sample_counts, settings.samples.max(1));
        let samples = match &state.failed {
            // A count that failed on this view is not tried again; the one that worked is used.
            Some(f) if asked >= f.requested => f.working,
            _ => asked,
        };
        let scaled = !settings.hi_dpi && frame.scale_factor > 1.0;
        let own = samples > 1 || scaled;
        let (size_px, scale_factor) = if scaled {
            (
                [
                    (frame.size_px[0] / frame.scale_factor as f32).round(),
                    (frame.size_px[1] / frame.scale_factor as f32).round(),
                ],
                1.0,
            )
        } else {
            (frame.size_px, frame.scale_factor)
        };
        state.viewport = [
            frame.origin_px[0],
            frame.origin_px[1],
            frame.size_px[0].max(1.0),
            frame.size_px[1].max(1.0),
        ];
        state.clear = settings.background.to_f32();
        if own {
            let size = [size_px[0].max(1.0) as u32, size_px[1].max(1.0) as u32];
            let current = state
                .targets
                .as_ref()
                .is_some_and(|t| t.size == size && t.samples == samples && t.scaled == scaled);
            if !current {
                let made = make_targets(
                    device,
                    &mut self.pipelines,
                    (&self.module, &self.pipeline_layout, self.format),
                    &self.compose,
                    size,
                    samples,
                    scaled,
                );
                match made {
                    Ok(targets) => {
                        state.targets = Some(targets);
                        state.working = samples;
                    }
                    Err(error) => {
                        let working = if samples == state.working {
                            1
                        } else {
                            state.working
                        };
                        state.failed = Some(SampleFailure {
                            requested: samples,
                            working,
                            error,
                        });
                        // What worked before, at this size: the host's pass when that is enough.
                        state.targets = (working > 1 || scaled)
                            .then(|| {
                                make_targets(
                                    device,
                                    &mut self.pipelines,
                                    (&self.module, &self.pipeline_layout, self.format),
                                    &self.compose,
                                    size,
                                    working,
                                    scaled,
                                )
                                .ok()
                            })
                            .flatten();
                        state.working = working;
                    }
                }
            }
        } else {
            // Dropped: wgpu frees the textures once the frames using them are done.
            state.targets = None;
            state.working = 1;
        }
        state.own = state.targets.is_some();

        let uniform = frame.camera.frame_uniform(
            frame.origin,
            if state.own { size_px } else { frame.size_px },
            if state.own {
                scale_factor
            } else {
                frame.scale_factor
            },
            settings,
            self.srgb_target,
        );
        queue.write_buffer(&state.uniform, 0, bytemuck::bytes_of(&uniform));
        state.stats = state.count();
        state.stats.uploaded_bytes = uploaded;
        state.stats.samples = state.targets.as_ref().map_or(1, |t| t.samples);
        state.stats.target_bytes = state.targets.as_ref().map_or(0, |t| t.bytes);
        if failure.is_some() {
            state.error.clone_from(&failure);
        }
        failure.map_or(Ok(()), Err)
    }

    /// Whether `view` draws into its own targets this frame: the host then
    /// calls [`Renderer::render`] instead of [`Renderer::draw`].
    pub fn owns_targets(&self, view: ViewId) -> bool {
        self.views.get(&view).is_some_and(|v| v.own)
    }

    /// Draws `view` into a pass whose viewport and scissor are the drawing
    /// area: the background, then every layer's fills, strokes and marks.
    /// Only for a view drawn single-sampled at full resolution.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, view: ViewId) {
        let (Some(state), Some(pipes)) = (self.views.get(&view), self.pipelines.get(&1)) else {
            return;
        };
        state.draw(pass, pipes);
    }

    /// Draws `view` into its own targets and composes the picture into
    /// `target` (the host's frame) inside the area, clipped to `clip`
    /// (x, y, width, height in device pixels). The host's frame keeps what is
    /// outside the area.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip: [u32; 4],
        view: ViewId,
    ) {
        let Some(state) = self.views.get(&view) else {
            return;
        };
        let Some(targets) = &state.targets else {
            return;
        };
        let Some(pipes) = self.pipelines.get(&targets.samples) else {
            return;
        };
        let [r, g, b, a] = state.clear.map(f64::from);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("kentos.cad2d.picture"),
                color_attachments: &[Some(targets.attachment(wgpu::Color { r, g, b, a }))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            state.draw(&mut pass, pipes);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kentos.cad2d.compose"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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
        let [x, y, w, h] = state.viewport;
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_scissor_rect(clip[0], clip[1], clip[2].max(1), clip[3].max(1));
        self.compose.draw(&mut pass, targets);
    }

    /// End of a frame: views left undrawn long enough are released with their buffers and targets.
    pub fn trim(&mut self) {
        self.views.retain(|_, view| {
            view.idle += 1;
            view.idle <= KEEP_IDLE_FRAMES
        });
    }

    /// What the last frame of `view` drew and uploaded.
    pub fn stats(&self, view: ViewId) -> Option<FrameStats> {
        self.views.get(&view).map(|v| v.stats)
    }

    /// The last upload `view` could not make, if any.
    pub fn error(&self, view: ViewId) -> Option<&RenderError> {
        self.views.get(&view).and_then(|v| v.error.as_ref())
    }

    /// The sample count `view` could not draw with, if any (it draws with the last that worked).
    pub fn sample_failure(&self, view: ViewId) -> Option<&SampleFailure> {
        self.views.get(&view).and_then(|v| v.failed.as_ref())
    }

    /// Views holding GPU buffers.
    pub fn view_count(&self) -> usize {
        self.views.len()
    }
}

/// The pipelines of the shared WGSL for targets of `format` and `samples` per pixel.
fn build_pipelines(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    samples: u32,
) -> Pipelines {
    let make = |spec: &PipelineSpec| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(spec.name),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some(spec.vertex),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: spec.buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: spec.topology,
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
                    blend: spec.blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        })
    };
    Pipelines {
        background: make(&layout::BACKGROUND),
        fill: make(&layout::FILL),
        line: make(&layout::LINE),
        marker: make(&layout::MARKER),
    }
}

/// A view's targets for `size` and `samples`, and the count's pipelines if
/// they are new, under error scopes: what the device refuses is dropped and
/// its reason returned.
fn make_targets(
    device: &wgpu::Device,
    pipelines: &mut HashMap<u32, Pipelines>,
    (module, layout, format): (
        &wgpu::ShaderModule,
        &wgpu::PipelineLayout,
        wgpu::TextureFormat,
    ),
    compose: &Compose,
    size: [u32; 2],
    samples: u32,
    scaled: bool,
) -> Result<Targets, String> {
    device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let new_pipelines = (!pipelines.contains_key(&samples))
        .then(|| build_pipelines(device, module, layout, format, samples));
    let targets = Targets::new(device, compose, format, size, samples, scaled);
    let validation = pop_error_now(device);
    let memory = pop_error_now(device);
    if let Some(error) = validation.or(memory) {
        return Err(error.to_string());
    }
    if let Some(p) = new_pipelines {
        pipelines.insert(samples, p);
    }
    Ok(targets)
}

struct View {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    parts: Vec<GpuPart>,
    stats: FrameStats,
    frames: u64,
    /// Trims since the view was last prepared.
    idle: u32,
    error: Option<RenderError>,
    /// The view's own targets, when it draws into them.
    targets: Option<Targets>,
    /// Whether this frame draws into `targets` (`render`) rather than the host's pass (`draw`).
    own: bool,
    /// The last sample count the view's targets were made with.
    working: u32,
    failed: Option<SampleFailure>,
    /// The area in the host's frame, device pixels: x, y, width, height.
    viewport: [f32; 4],
    clear: [f32; 4],
}

impl View {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kentos.cad2d.frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.cad2d.frame"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            uniform,
            bind_group,
            parts: Vec::new(),
            stats: FrameStats::default(),
            frames: 0,
            idle: 0,
            error: None,
            targets: None,
            own: false,
            working: 1,
            failed: None,
            viewport: [0.0, 0.0, 1.0, 1.0],
            clear: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// The background, then every layer's fills, strokes and marks, with the given pipelines.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, pipes: &Pipelines) {
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_pipeline(&pipes.background);
        pass.draw(0..layout::BACKGROUND.vertex_count.unwrap_or(3), 0..1);
        let passes: [(&wgpu::RenderPipeline, Kind); 3] = [
            (&pipes.fill, Kind::Fills),
            (&pipes.line, Kind::Segments),
            (&pipes.marker, Kind::Markers),
        ];
        for (pipeline, kind) in passes {
            pass.set_pipeline(pipeline);
            self.each(kind, |chunks, range| {
                chunks.draw(pass, range, kind.vertices_per_instance());
            });
        }
    }

    /// Visits what a frame draws of one kind, in draw order: layer by layer,
    /// bottom first, each part's share of the layer in slot order.
    fn each(&self, kind: Kind, mut visit: impl FnMut(&Chunks, &Range<u32>)) {
        let layers = self.parts.iter().map(|p| p.layers.len()).max().unwrap_or(0);
        for layer in 0..layers {
            for part in &self.parts {
                if let Some(ranges) = part.layers.get(layer) {
                    let (chunks, range) = kind.select(part, ranges);
                    visit(chunks, range);
                }
            }
        }
    }

    /// The counts of a frame, found the way `draw` walks the parts.
    fn count(&self) -> FrameStats {
        let mut stats = FrameStats {
            draw_calls: 1,
            frames: self.frames,
            resident_bytes: std::mem::size_of::<FrameUniform>() as u64
                + self.parts.iter().map(|p| p.bytes).sum::<u64>(),
            ..FrameStats::default()
        };
        for kind in [Kind::Fills, Kind::Segments, Kind::Markers] {
            self.each(kind, |chunks, range| {
                let (calls, elements) = chunks.count(range);
                stats.draw_calls += calls;
                match kind {
                    Kind::Fills => stats.triangles += elements / 3,
                    Kind::Segments => stats.segments += elements,
                    Kind::Markers => stats.markers += elements,
                }
            });
        }
        stats
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Fills,
    Segments,
    Markers,
}

impl Kind {
    fn select<'a>(
        self,
        part: &'a GpuPart,
        ranges: &'a LayerRanges,
    ) -> (&'a Chunks, &'a Range<u32>) {
        match self {
            Kind::Fills => (&part.fills, &ranges.fills),
            Kind::Segments => (&part.segments, &ranges.segments),
            Kind::Markers => (&part.markers, &ranges.markers),
        }
    }

    /// Instanced kinds draw this many vertices per element; fills draw their vertices.
    fn vertices_per_instance(self) -> Option<u32> {
        match self {
            Kind::Fills => None,
            Kind::Segments => Some(layout::LINE.vertex_count.unwrap_or(4)),
            Kind::Markers => Some(layout::MARKER.vertex_count.unwrap_or(4)),
        }
    }
}

/// A scene part on the GPU.
struct GpuPart {
    id: u64,
    fills: Chunks,
    segments: Chunks,
    markers: Chunks,
    layers: Vec<LayerRanges>,
    bytes: u64,
}

impl GpuPart {
    fn upload(device: &wgpu::Device, part: &ScenePart, chunk_bytes: u64) -> Self {
        Self {
            id: part.id,
            fills: Chunks::upload(device, "kentos.cad2d.fills", &part.fills, chunk_bytes),
            segments: Chunks::upload(device, "kentos.cad2d.segments", &part.segments, chunk_bytes),
            markers: Chunks::upload(device, "kentos.cad2d.markers", &part.markers, chunk_bytes),
            layers: part.layers.clone(),
            bytes: part.byte_size(),
        }
    }

    fn empty(id: u64) -> Self {
        Self {
            id,
            fills: Chunks::default(),
            segments: Chunks::default(),
            markers: Chunks::default(),
            layers: Vec::new(),
            bytes: 0,
        }
    }
}

/// An array split over vertex buffers no larger than the device allows; each
/// buffer holds a range of the array's elements (whole triangles for fills).
#[derive(Default)]
struct Chunks(Vec<(wgpu::Buffer, Range<u32>)>);

impl Chunks {
    fn upload<T: Pod>(device: &wgpu::Device, label: &str, items: &[T], chunk_bytes: u64) -> Self {
        let size = std::mem::size_of::<T>() as u64;
        if items.is_empty() || size == 0 {
            return Self::default();
        }
        // A multiple of three elements per buffer keeps fill triangles whole.
        let per_chunk = ((chunk_bytes / size) / 3 * 3).max(3) as usize;
        let chunks = items
            .chunks(per_chunk)
            .enumerate()
            .map(|(i, chunk)| {
                let start = i * per_chunk;
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(chunk),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                (buffer, start as u32..(start + chunk.len()) as u32)
            })
            .collect();
        Self(chunks)
    }

    /// The part of each buffer `range` covers, in buffer-local element indices.
    fn spans<'a>(
        &'a self,
        range: &'a Range<u32>,
    ) -> impl Iterator<Item = (&'a wgpu::Buffer, Range<u32>)> {
        self.0.iter().filter_map(move |(buffer, held)| {
            let start = range.start.max(held.start);
            let end = range.end.min(held.end);
            (start < end).then(|| (buffer, start - held.start..end - held.start))
        })
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, range: &Range<u32>, per_instance: Option<u32>) {
        for (buffer, local) in self.spans(range) {
            pass.set_vertex_buffer(0, buffer.slice(..));
            match per_instance {
                Some(vertices) => pass.draw(0..vertices, local),
                None => pass.draw(local, 0..1),
            }
        }
    }

    /// Draw calls and elements `draw` issues for `range`.
    fn count(&self, range: &Range<u32>) -> (u32, u64) {
        self.spans(range)
            .fold((0, 0), |(calls, elements), (_, local)| {
                (calls + 1, elements + u64::from(local.end - local.start))
            })
    }
}
