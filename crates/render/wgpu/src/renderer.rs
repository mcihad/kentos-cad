//! Device resources of the drawing pipeline (TODOS.md REN-01, REN-02): the
//! pipelines of the shared WGSL, a frame uniform per view, and the scene cache.
//!
//! Who owns what (REN-02), as the desktop uses it from Iced's shader widget:
//! - The device and queue are the host's (Iced's). Nothing here opens a
//!   device, so the drawing and the interface share one GPU context and one
//!   frame.
//! - The render pass is the host's too. Its colour target is the window's
//!   frame, its viewport the drawing area, its scissor the area's visible
//!   part; the renderer never changes them. It draws straight into that pass:
//!   no offscreen texture, no copy, nothing read back to the CPU.
//! - No depth buffer: a 2D drawing is drawn in order (fills, then strokes,
//!   then marks, each bottom layer first, as the web does).
//! - No multisampling in that pass (the host's target is single-sampled);
//!   strokes and marks are anti-aliased in the shader. MSAA with its own
//!   target and resolve belongs to AA-01..03.
//! - DPI and resize: the host gives the area's size in device pixels and the
//!   scale factor with every frame; only the uniform follows them.
//!
//! The scene cache: a view keeps the GPU buffers of the scene parts it drew
//! last, by part id. A frame with the same parts uploads only its 64-byte
//! uniform; a new part is uploaded once, in chunks the device accepts, and
//! the buffers it replaces are released. A view left undrawn for
//! [`KEEP_IDLE_FRAMES`] frames is dropped with its buffers ([`Renderer::trim`]).

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::ops::Range;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::Vec2;
use crate::camera::Camera;
use crate::layout::{self, FrameUniform, PipelineSpec};
use crate::scene::{LayerRanges, ScenePart};
use crate::settings::RenderSettings;
use crate::shader;
use crate::stats::FrameStats;

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

/// What a frame of a view needs besides its scene parts.
#[derive(Clone, Copy, Debug)]
pub struct FrameInput<'a> {
    pub camera: &'a Camera,
    /// The origin the parts' offsets are from.
    pub origin: Vec2,
    /// The drawing area in device pixels.
    pub size_px: [f32; 2],
    /// Device pixels per logical pixel.
    pub scale_factor: f64,
    pub settings: &'a RenderSettings,
}

pub struct Renderer {
    srgb_target: bool,
    frame_layout: wgpu::BindGroupLayout,
    background: wgpu::RenderPipeline,
    fill: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    marker: wgpu::RenderPipeline,
    views: HashMap<ViewId, View>,
    chunk_bytes: u64,
}

impl fmt::Debug for Renderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Renderer")
            .field("srgb_target", &self.srgb_target)
            .field("views", &self.views.len())
            .finish_non_exhaustive()
    }
}

impl Renderer {
    /// Builds the pipelines for targets of `format` on the host's device.
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
        let make = |spec: &PipelineSpec| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(spec.name),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
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
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
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
        let background = make(&layout::BACKGROUND);
        let fill = make(&layout::FILL);
        let line = make(&layout::LINE);
        let marker = make(&layout::MARKER);
        if let Some(error) = pop_error_now(device) {
            return Err(RenderError(error.to_string()));
        }
        Ok(Self {
            srgb_target: format.is_srgb(),
            frame_layout,
            background,
            fill,
            line,
            marker,
            views: HashMap::new(),
            chunk_bytes: device.limits().max_buffer_size.min(MAX_CHUNK_BYTES),
        })
    }

    /// Before a frame of `view`: uploads the parts the view has not drawn
    /// yet (by id, in slot order) and writes the frame uniform. Parts it drew
    /// last time cost nothing. An upload the device refuses (out of memory)
    /// leaves that part undrawn and is reported once.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: ViewId,
        parts: &[&ScenePart],
        frame: &FrameInput<'_>,
    ) -> Result<(), RenderError> {
        let uniform = frame.camera.frame_uniform(
            frame.origin,
            frame.size_px,
            frame.scale_factor,
            frame.settings,
            self.srgb_target,
        );
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
        queue.write_buffer(&state.uniform, 0, bytemuck::bytes_of(&uniform));
        state.stats = state.count();
        state.stats.uploaded_bytes = uploaded;
        if failure.is_some() {
            state.error.clone_from(&failure);
        }
        failure.map_or(Ok(()), Err)
    }

    /// Draws `view` into a pass whose viewport and scissor are the drawing
    /// area: the background, then every layer's fills, strokes and marks.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, view: ViewId) {
        let Some(state) = self.views.get(&view) else {
            return;
        };
        pass.set_bind_group(0, &state.bind_group, &[]);
        pass.set_pipeline(&self.background);
        pass.draw(0..layout::BACKGROUND.vertex_count.unwrap_or(3), 0..1);
        let passes: [(&wgpu::RenderPipeline, Kind); 3] = [
            (&self.fill, Kind::Fills),
            (&self.line, Kind::Segments),
            (&self.marker, Kind::Markers),
        ];
        for (pipeline, kind) in passes {
            pass.set_pipeline(pipeline);
            state.each(kind, |chunks, range| {
                chunks.draw(pass, range, kind.vertices_per_instance());
            });
        }
    }

    /// End of a frame: views left undrawn long enough are released with their buffers.
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

    /// Views holding GPU buffers.
    pub fn view_count(&self) -> usize {
        self.views.len()
    }
}

/// wgpu's answer to the innermost error scope, taken at once: native
/// backends answer synchronously, so the future is ready on its first poll.
fn pop_error_now(device: &wgpu::Device) -> Option<wgpu::Error> {
    let future = pin!(device.pop_error_scope());
    match future.poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(error) => error,
        Poll::Pending => None,
    }
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
