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
//! The kept picture: the host redraws its whole window for any event (a
//! pointer move, a button's hover), and a large drawing costs the GPU far
//! more than the interface. A view asked to keep its picture
//! (`FrameInput::keep_picture`, the desktop's drawing area) draws into its
//! own targets and remembers what the resolved picture shows: the parts'
//! ids, the frame uniform, the ground and the styled layers' batches and
//! images. A frame with the same key composes the kept picture and draws
//! nothing else of the scene. What changes with the pointer (the hovered
//! object, `FrameInput::overlays`) is left out of it and drawn over it in
//! every frame, straight into the host's frame at its resolution, as the
//! web draws its hover apart from the drawing's layers.
//!
//! The scene cache: a view keeps the GPU buffers of the scene parts it drew
//! last, by part id. A frame with the same parts uploads only its 64-byte
//! uniform; a new part is uploaded once, in chunks the device accepts, and
//! the buffers it replaces are released. A view left undrawn for
//! [`KEEP_IDLE_FRAMES`] frames is dropped with its buffers and targets
//! ([`Renderer::trim`]).

use std::collections::HashMap;
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::Vec2;
use crate::camera::Camera;
use crate::compose::{
    ComposeFrame, ComposeTargets, Composer, DrawBatches, Into as ComposeInto, PicturePass, Planner,
};
use crate::layout::{self, FrameUniform, PipelineSpec};
use crate::scene::{LayerRanges, ScenePart};
use crate::settings::RenderSettings;
use crate::shader;
use crate::stats::FrameStats;
use crate::styled::{ImageSource, StyledFrame, StyledGpu, StyledScene, ViewStyled};
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
    /// Keep the picture between frames (the desktop's drawing area): the
    /// view draws into its own targets even single-sampled at full
    /// resolution, and a frame whose picture would be the last one's
    /// composes the kept picture without drawing the scene again.
    pub keep_picture: bool,
    /// With `keep_picture`: how many of the last parts change often (the
    /// hovered object). They are left out of the kept picture and drawn over
    /// it in every frame, straight into the host's frame at its resolution.
    pub overlays: usize,
}

/// Büyüteç (docs/adr/0181 §5): a view's scene seen through a second camera,
/// drawn into the host's pass where the host's viewport is (the magnifier's
/// window), single-sampled.
#[derive(Clone, Copy, Debug)]
pub struct LensInput<'a> {
    pub camera: &'a Camera,
    /// The origin the view's parts are offsets from.
    pub origin: Vec2,
    /// The window in device pixels.
    pub size_px: [f32; 2],
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
    /// The styled layers' pipelines and atlas, made with the first styled frame (docs/adr/0090).
    styled: Option<StyledGpu>,
    /// The composing pipelines, made with the first composed frame (docs/adr/0226).
    composer: Option<Composer>,
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
            styled: None,
            composer: None,
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
        let own = samples > 1 || scaled || frame.keep_picture;
        state.viewport = [
            frame.origin_px[0],
            frame.origin_px[1],
            frame.size_px[0].max(1.0),
            frame.size_px[1].max(1.0),
        ];
        // A picture shown pixel for pixel lands on whole device pixels, its corners rounded as Iced
        // rounds the primitive's clip. Composed over an area a fraction of a pixel off, the nearest
        // sampler reads the picture at its pixels' edges: rows and columns come twice or not at all,
        // and a slanted line breaks into beads (the symbol designer's preview in its centred dialog,
        // any area at a fractional scale). A picture scaled up is laid over the area as it is.
        state.placed = if scaled {
            state.viewport
        } else {
            whole_pixels(frame.origin_px, frame.size_px)
        };
        let (size_px, scale_factor) = if scaled {
            (
                [
                    (frame.size_px[0] / frame.scale_factor as f32).round(),
                    (frame.size_px[1] / frame.scale_factor as f32).round(),
                ],
                1.0,
            )
        } else {
            ([state.placed[2], state.placed[3]], frame.scale_factor)
        };
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

        let (drawn_size, drawn_scale) = if state.own {
            (size_px, scale_factor)
        } else {
            (frame.size_px, frame.scale_factor)
        };
        // The picture's camera follows it onto whole pixels: the drawing stays on the host's pixels
        // (the overlays and the canvas drawn over it agree to the pixel).
        let camera = if state.own && !scaled {
            camera_at(frame.camera, state.viewport, state.placed, scale_factor)
        } else {
            *frame.camera
        };
        state.drawn = Drawn {
            size_px: drawn_size,
            scale_factor: drawn_scale,
            center: [
                camera.center.x - frame.origin.x,
                camera.center.y - frame.origin.y,
            ],
            origin: [frame.origin.x, frame.origin.y],
            // The scene's numbers are the map's (docs/adr/0583 §3).
            scale: camera.map_px(),
            screen_scale: camera.screen_scale(),
            twist: camera.twist,
            map: camera.map,
        };
        let uniform = camera.frame_uniform(
            frame.origin,
            drawn_size,
            drawn_scale,
            settings,
            self.srgb_target,
        );
        queue.write_buffer(&state.uniform, 0, bytemuck::bytes_of(&uniform));
        let wide = FrameUniform {
            line_width: settings.highlight_width,
            ..uniform
        };
        queue.write_buffer(&state.highlight.0, 0, bytemuck::bytes_of(&wide));
        uploaded += std::mem::size_of::<FrameUniform>() as u64;
        // What the picture shows and how: the parts under the overlays, the
        // frame and the ground. The styled layers add theirs (prepare_styled).
        state.keep = frame.keep_picture && state.own;
        state.overlays = if state.keep {
            frame.overlays.min(state.parts.len())
        } else {
            0
        };
        let mut key = DefaultHasher::new();
        for part in &state.parts[..state.parts.len() - state.overlays] {
            part.id.hash(&mut key);
        }
        bytemuck::bytes_of(&uniform).hash(&mut key);
        settings.highlight_width.to_bits().hash(&mut key);
        state.clear.map(f32::to_bits).hash(&mut key);
        state.want = key.finish();
        state.want_styled = None;
        if state.overlays > 0 {
            // The overlays go straight into the host's frame: its size and scale; they are the
            // highlights, at their width.
            let overlay = FrameUniform {
                line_width: settings.highlight_width,
                ..frame.camera.frame_uniform(
                    frame.origin,
                    frame.size_px,
                    frame.scale_factor,
                    settings,
                    self.srgb_target,
                )
            };
            let (buffer, _) = state
                .overlay
                .get_or_insert_with(|| frame_binding(device, frame_layout, "kentos.cad2d.overlay"));
            queue.write_buffer(buffer, 0, bytemuck::bytes_of(&overlay));
            uploaded += std::mem::size_of::<FrameUniform>() as u64;
        }
        state.stats = state.count();
        state.stats.uploaded_bytes = uploaded;
        state.stats.samples = state.targets.as_ref().map_or(1, |t| t.samples);
        state.stats.target_bytes = state.targets.as_ref().map_or(0, |t| t.bytes);
        state.stats.picture_kept = state.picture_kept();
        if failure.is_some() {
            state.error.clone_from(&failure);
        }
        failure.map_or(Ok(()), Err)
    }

    /// After [`Renderer::prepare`] of the same frame: the view's styled layers
    /// (docs/adr/0090). Uploads the layers it has not drawn (by id), drops the
    /// ones it no longer draws, decides what shows and places its images in
    /// the atlas, drawn from `images`. They are drawn over the first
    /// `scene.under` parts and under the rest. Returns whether an image waits
    /// for another frame (the host then asks for one).
    pub fn prepare_styled(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: ViewId,
        scene: &StyledScene,
        images: &dyn ImageSource,
    ) -> bool {
        let format = self.format;
        let gpu = self
            .styled
            .get_or_insert_with(|| StyledGpu::new(device, format));
        let Some(state) = self.views.get_mut(&view) else {
            return false;
        };
        let samples = state.targets.as_ref().map_or(1, |t| t.samples);
        let styled = state
            .styled
            .get_or_insert_with(|| ViewStyled::new(device, gpu));
        let d = state.drawn;
        let pending = gpu.prepare(
            device,
            queue,
            styled,
            scene,
            &StyledFrame {
                center: d.center,
                origin: d.origin,
                scale: d.scale,
                dpr: d.scale_factor,
                size_px: d.size_px,
                scale_denominator: d.screen_scale,
                twist: d.twist,
                map: d.map,
            },
            samples,
            images,
        );
        let (drawn, bytes) = styled.counts();
        let mut key = DefaultHasher::new();
        styled.key(&mut key);
        state.want_styled = Some(key.finish());
        state.stats.draw_calls += drawn;
        state.stats.resident_bytes += bytes;
        // Composed layers and effect groups (docs/adr/0226): planned now, recorded in render.
        state.compose = None;
        if let Some(targets) = &state.targets
            && needs_compose(styled)
        {
            let composer = self
                .composer
                .get_or_insert_with(|| Composer::new(device, format));
            composer.ensure(device, targets.samples);
            let fresh = state
                .compose_targets
                .as_ref()
                .is_some_and(|c| c.size == targets.size && c.samples == targets.samples);
            if !fresh {
                state.compose_targets =
                    Some(ComposeTargets::new(format, targets.size, targets.samples));
            }
            if let Some(ct) = state.compose_targets.as_mut() {
                let (_, px_per_m, dpr, _, _) = styled.frame_facts();
                let mut planner = Planner::new(composer, ct, device, px_per_m, dpr);
                plan_composed(&mut planner, styled);
                state.compose = Some(planner.finish(queue));
                state.stats.target_bytes += ct.bytes;
            }
        }
        state.stats.picture_kept = state.picture_kept();
        pending
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
        state.draw(
            pass,
            pipes,
            self.styled.as_ref().map(|g| (g, 1)),
            state.parts.len(),
        );
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
        // A kept picture that is still the one this frame shows is composed as it is.
        let key = state.picture_key();
        if !(state.keep && targets.holds(key))
            && let (Some(frame), Some(ct), Some(composer), Some(gpu), Some(styled)) = (
                &state.compose,
                &state.compose_targets,
                &self.composer,
                &self.styled,
                &state.styled,
            )
        {
            // A composed frame (docs/adr/0226): the ground first, the passes, the highlights last; the
            // passes into the picture run on in one render pass where nothing else comes between.
            let drawer = Batches {
                gpu,
                view: styled,
                samples: targets.samples,
            };
            let under = styled.under.min(state.parts.len());
            let picture =
                |enc: &mut wgpu::CommandEncoder,
                 at: PicturePass,
                 draw: &mut dyn FnMut(&mut wgpu::RenderPass<'_>)| {
                    let clear = at.first.then_some(wgpu::Color { r, g, b, a });
                    let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("kentos.cad2d.picture"),
                        color_attachments: &[Some(targets.attachment_kept(clear, at.resolve))],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    if at.first {
                        pass.set_bind_group(0, &state.bind_group, &[]);
                        pass.set_pipeline(&pipes.background);
                        pass.draw(0..layout::BACKGROUND.vertex_count.unwrap_or(3), 0..1);
                        state.draw_parts(&mut pass, pipes, 0..under);
                    }
                    draw(&mut pass);
                    if at.last {
                        // The highlights, at their width.
                        pass.set_bind_group(0, &state.highlight.1, &[]);
                        state.draw_parts(
                            &mut pass,
                            pipes,
                            under..state.parts.len() - state.overlays,
                        );
                    }
                };
            let copy = |enc: &mut wgpu::CommandEncoder, to: &wgpu::Texture| {
                enc.copy_texture_to_texture(
                    targets.resolve_texture.as_image_copy(),
                    to.as_image_copy(),
                    wgpu::Extent3d {
                        width: targets.size[0],
                        height: targets.size[1],
                        depth_or_array_layers: 1,
                    },
                );
            };
            composer.record(
                encoder,
                ct,
                frame,
                targets.samples,
                &drawer,
                &picture,
                &copy,
            );
            if state.keep {
                targets.keep(key);
            }
        } else if !(state.keep && targets.holds(key)) {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("kentos.cad2d.picture"),
                color_attachments: &[Some(targets.attachment(wgpu::Color { r, g, b, a }))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            state.draw(
                &mut pass,
                pipes,
                self.styled.as_ref().map(|g| (g, targets.samples)),
                state.parts.len() - state.overlays,
            );
            if state.keep {
                targets.keep(key);
            }
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
        let [x, y, w, h] = state.placed;
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_scissor_rect(clip[0], clip[1], clip[2].max(1), clip[3].max(1));
        self.compose.draw(&mut pass, targets);
        // The overlays over the picture, at the host's resolution, in the host's area.
        if let (Some((_, bind)), Some(host)) = (&state.overlay, self.pipelines.get(&1))
            && state.overlays > 0
        {
            let [x, y, w, h] = state.viewport;
            pass.set_viewport(x, y, w, h, 0.0, 1.0);
            pass.set_bind_group(0, bind, &[]);
            let all = state.parts.len();
            state.draw_parts(&mut pass, host, all - state.overlays..all);
        }
    }

    /// Büyüteç (docs/adr/0181 §5): after [`Renderer::prepare`] and
    /// [`Renderer::prepare_styled`] of `view` in the same frame, its second
    /// camera: a frame uniform of its own for the plain parts and for the
    /// styled layers, and the box it sees. What shows is the view's own
    /// choice. False when the view has not been prepared.
    pub fn prepare_lens(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: ViewId,
        lens: &LensInput<'_>,
    ) -> bool {
        let frame_layout = &self.frame_layout;
        let Some(state) = self.views.get_mut(&view) else {
            return false;
        };
        let uniform = lens.camera.frame_uniform(
            lens.origin,
            lens.size_px,
            lens.scale_factor,
            lens.settings,
            self.srgb_target,
        );
        let (buffer, _) = state
            .lens
            .get_or_insert_with(|| frame_binding(device, frame_layout, "kentos.cad2d.lens"));
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&uniform));
        if let (Some(gpu), Some(styled)) = (self.styled.as_mut(), state.styled.as_mut()) {
            gpu.prepare_lens(
                device,
                queue,
                styled,
                &StyledFrame {
                    center: [
                        lens.camera.center.x - lens.origin.x,
                        lens.camera.center.y - lens.origin.y,
                    ],
                    origin: [lens.origin.x, lens.origin.y],
                    scale: lens.camera.map_px(),
                    dpr: lens.scale_factor,
                    size_px: lens.size_px,
                    scale_denominator: state.drawn.screen_scale,
                    twist: lens.camera.twist,
                    map: lens.camera.map,
                },
            );
        }
        true
    }

    /// Draws `view` through its lens into a pass whose viewport and scissor
    /// are the magnifier's window: the ground, every part and the styled
    /// layers that reach it.
    pub fn draw_lens(&self, pass: &mut wgpu::RenderPass<'_>, view: ViewId) {
        let (Some(state), Some(pipes)) = (self.views.get(&view), self.pipelines.get(&1)) else {
            return;
        };
        let Some((_, bind)) = &state.lens else {
            return;
        };
        pass.set_bind_group(0, bind, &[]);
        pass.set_pipeline(&pipes.background);
        pass.draw(0..layout::BACKGROUND.vertex_count.unwrap_or(3), 0..1);
        let all = state.parts.len();
        match (&self.styled, &state.styled) {
            (Some(gpu), Some(styled)) if !styled.layers.is_empty() => {
                let under = styled.under.min(all);
                state.draw_parts(pass, pipes, 0..under);
                gpu.draw_lens(pass, styled);
                pass.set_bind_group(0, bind, &[]);
                state.draw_parts(pass, pipes, under..all);
            }
            _ => state.draw_parts(pass, pipes, 0..all),
        }
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

/// Draws a stretch of the styled draw order into a composed frame's pass.
struct Batches<'a> {
    gpu: &'a StyledGpu,
    view: &'a ViewStyled,
    samples: u32,
}

impl DrawBatches for Batches<'_> {
    fn draw_batches(&self, pass: &mut wgpu::RenderPass<'_>, seq: Range<usize>) {
        self.gpu.draw_seq(pass, self.view, self.samples, seq);
    }
}

/// Whether a frame composes (docs/adr/0226): a composed layer, or an effect group drawn.
fn needs_compose(view: &ViewStyled) -> bool {
    !view.compose.is_empty()
        || view.seq.iter().any(|&(l, b)| {
            view.batch_facts(l, b)
                .is_some_and(|(drawn, _, e)| drawn && e.is_some())
        })
}

/// The composed frame's passes from the view's draw order (docs/adr/0226): the
/// plain stretches straight into the picture, each composed layer's stretch
/// into its own picture and laid over or mixed in, each effect group's into
/// its own with its effects.
fn plan_composed(planner: &mut Planner<'_>, view: &ViewStyled) {
    let seq = &view.seq;
    let composed_of = |l: u32| view.compose.iter().position(|(r, _)| r.contains(&l));
    let mut i = 0;
    while i < seq.len() {
        let c = composed_of(seq[i].0);
        let start = i;
        while i < seq.len() && composed_of(seq[i].0) == c {
            i += 1;
        }
        let Some(c) = c else {
            runs(planner, view, start..i, ComposeInto::Picture);
            continue;
        };
        let drawn = (start..i).any(|k| {
            view.batch_facts(seq[k].0, seq[k].1)
                .is_some_and(|(d, _, _)| d)
        });
        if !drawn {
            continue;
        }
        planner.layer_clear();
        runs(planner, view, start..i, ComposeInto::Layer);
        let draw = &view.compose[c].1;
        let (centre, px_per_m, dpr, size, twist) = view.frame_facts();
        let (ts, tc) = twist.sin_cos();
        let mut boxes: Vec<[f32; 7]> = Vec::new();
        for m in &draw.masks {
            let grow = m.grow * dpr as f32;
            for b in m.labels.iter() {
                // The turn as the screen turns (y down), the view's twist with it (docs/adr/0285 §2), its
                // sine and cosine made here in double precision.
                let (sin, cos) = (-(f64::from(b.turn).to_radians() - twist)).sin_cos();
                let (dx, dy) = (
                    (b.centre[0] - centre[0]) * px_per_m,
                    (b.centre[1] - centre[1]) * px_per_m,
                );
                let (u, v) = (dx * tc + dy * ts, dy * tc - dx * ts);
                boxes.push([
                    (f64::from(size[0]) / 2.0 + u) as f32,
                    (f64::from(size[1]) / 2.0 - v) as f32,
                    b.size[0] * dpr as f32 / 2.0,
                    b.size[1] * dpr as f32 / 2.0,
                    cos as f32,
                    sin as f32,
                    grow,
                ]);
            }
            if let Some(r) = &m.markers {
                view.marker_boxes(r.clone(), grow, &mut boxes);
            }
        }
        let mask = (!draw.masks.is_empty()).then(|| planner.boxes(&boxes));
        planner.layer(draw.opacity, draw.blend, mask);
    }
}

/// A stretch split into its plain runs and its effect groups' runs.
fn runs(planner: &mut Planner<'_>, view: &ViewStyled, range: Range<usize>, into: ComposeInto) {
    let seq = &view.seq;
    let facts = |k: usize| view.batch_facts(seq[k].0, seq[k].1);
    let mut k = range.start;
    while k < range.end {
        let start = k;
        match facts(k).and_then(|(_, level, e)| e.map(|e| (level, e.clone()))) {
            None => {
                while k < range.end && facts(k).is_none_or(|(_, _, e)| e.is_none()) {
                    k += 1;
                }
                planner.batches(start..k, into);
            }
            Some((level, effect)) => {
                // One symbol layer's group: the same level and the same effects, part after part.
                while k < range.end
                    && facts(k)
                        .is_some_and(|(_, l, e)| l == level && e.is_some_and(|e| **e == *effect))
                {
                    k += 1;
                }
                if (start..k).any(|j| facts(j).is_some_and(|(d, _, _)| d)) {
                    planner.group(start..k, &effect, into);
                }
            }
        }
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
    /// The frame uniform the highlights are drawn with: the main one at the
    /// highlight's width (Vurgu kalınlığı, docs/adr/0195).
    highlight: (wgpu::Buffer, wgpu::BindGroup),
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
    /// Where the view's own picture is laid: the area on whole pixels (see [`Renderer::prepare`]).
    placed: [f32; 4],
    clear: [f32; 4],
    /// What the last prepared frame is drawn at (the styled layers' frame).
    drawn: Drawn,
    /// The view's styled layers (docs/adr/0090), once it has had any.
    styled: Option<ViewStyled>,
    /// This frame keeps its picture (`FrameInput::keep_picture`, with targets).
    keep: bool,
    /// The picture's key from the plain parts and the frame (`prepare`) and
    /// from the styled layers (`prepare_styled`): the same key, the same pixels.
    want: u64,
    want_styled: Option<u64>,
    /// The last parts, drawn over the kept picture in every frame.
    overlays: usize,
    /// The host frame's uniform the overlays are drawn with, once needed.
    overlay: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    /// Büyüteç's frame uniform (docs/adr/0181 §5), once it is open.
    lens: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    /// The composing pictures and this frame's composed passes (docs/adr/0226).
    compose_targets: Option<ComposeTargets>,
    compose: Option<ComposeFrame>,
}

/// The target and camera a frame is drawn at.
#[derive(Clone, Copy, Debug, Default)]
struct Drawn {
    size_px: [f32; 2],
    scale_factor: f64,
    /// Camera centre relative to the origin, metres.
    center: [f64; 2],
    /// The origin in the world.
    origin: [f64; 2],
    /// Logical pixels per metre.
    scale: f64,
    /// The screen scale 1:N at 96 dpi.
    screen_scale: f64,
    /// The view's twist, radians (docs/adr/0285 §2).
    twist: f64,
    /// The map's system (docs/adr/0583 §3).
    map: Option<&'static kentos_geometry_core::crs::map::MapProjection>,
}

/// The area `origin`, `size` (device pixels) with its corners on whole pixels, as Iced snaps a
/// primitive's clip: x, y, width, height.
fn whole_pixels(origin: [f32; 2], size: [f32; 2]) -> [f32; 4] {
    let (x0, y0) = (origin[0].round(), origin[1].round());
    let (x1, y1) = ((origin[0] + size[0]).round(), (origin[1] + size[1]).round());
    [x0, y0, (x1 - x0).max(1.0), (y1 - y0).max(1.0)]
}

/// `camera` for a picture laid at `placed` instead of the host's `area` (device pixels, x, y,
/// width, height): its centre moved by what the picture's middle is off the area's, so a point
/// lands on the host's pixel it would have.
fn camera_at(camera: &Camera, area: [f32; 4], placed: [f32; 4], scale_factor: f64) -> Camera {
    let px_per_unit = camera.map_px() * scale_factor;
    if px_per_unit.is_nan() || px_per_unit <= 0.0 {
        return *camera;
    }
    let off =
        |i: usize| f64::from(area[i] - placed[i]) + f64::from(area[i + 2] - placed[i + 2]) / 2.0;
    // The offset is along the screen's axes: a twisted view turns it into the world (docs/adr/0285 §2).
    let (u, v) = (off(0) / px_per_unit, -off(1) / px_per_unit);
    let (dx, dy) = if camera.twist == 0.0 {
        (u, v)
    } else {
        let (s, c) = camera.twist.sin_cos();
        (u * c - v * s, u * s + v * c)
    };
    Camera {
        center: Vec2::new(camera.center.x - dx, camera.center.y - dy),
        ..*camera
    }
}

/// A frame uniform's buffer and its bind group.
fn frame_binding(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &'static str,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: std::mem::size_of::<FrameUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }],
    });
    (uniform, bind_group)
}

impl View {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let (uniform, bind_group) = frame_binding(device, layout, "kentos.cad2d.frame");
        Self {
            uniform,
            bind_group,
            highlight: frame_binding(device, layout, "kentos.cad2d.highlight"),
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
            placed: [0.0, 0.0, 1.0, 1.0],
            clear: [0.0, 0.0, 0.0, 1.0],
            drawn: Drawn::default(),
            styled: None,
            keep: false,
            want: 0,
            want_styled: None,
            overlays: 0,
            overlay: None,
            lens: None,
            compose_targets: None,
            compose: None,
        }
    }

    /// The key of the picture this frame shows.
    fn picture_key(&self) -> u64 {
        let mut key = DefaultHasher::new();
        self.want.hash(&mut key);
        self.want_styled.hash(&mut key);
        key.finish()
    }

    /// Whether this frame composes the picture kept from an earlier one.
    fn picture_kept(&self) -> bool {
        self.keep
            && self
                .targets
                .as_ref()
                .is_some_and(|t| t.holds(self.picture_key()))
    }

    /// The background, then every layer's fills, strokes and marks of the
    /// parts before `end`, with the given pipelines. Styled layers, when the
    /// view has them, draw over the first parts (the grid) and under the rest
    /// (the highlights).
    fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        pipes: &Pipelines,
        styled: Option<(&StyledGpu, u32)>,
        end: usize,
    ) {
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_pipeline(&pipes.background);
        pass.draw(0..layout::BACKGROUND.vertex_count.unwrap_or(3), 0..1);
        let all = 0..end.min(self.parts.len());
        match (styled, &self.styled) {
            (Some((gpu, samples)), Some(view)) if !view.layers.is_empty() => {
                let under = view.under.min(self.parts.len());
                self.draw_parts(pass, pipes, 0..under);
                gpu.draw(pass, view, samples);
                // The highlights, at their width.
                pass.set_bind_group(0, &self.highlight.1, &[]);
                self.draw_parts(pass, pipes, under..all.end);
            }
            _ => self.draw_parts(pass, pipes, all),
        }
    }

    /// The fills, strokes and marks of the parts in `parts`, kind by kind.
    fn draw_parts(&self, pass: &mut wgpu::RenderPass<'_>, pipes: &Pipelines, parts: Range<usize>) {
        if parts.is_empty() {
            return;
        }
        let passes: [(&wgpu::RenderPipeline, Kind); 3] = [
            (&pipes.fill, Kind::Fills),
            (&pipes.line, Kind::Segments),
            (&pipes.marker, Kind::Markers),
        ];
        for (pipeline, kind) in passes {
            pass.set_pipeline(pipeline);
            self.each_in(parts.clone(), kind, |chunks, range| {
                chunks.draw(pass, range, kind.vertices_per_instance());
            });
        }
    }

    /// Visits what a frame draws of one kind, in draw order: layer by layer,
    /// bottom first, each part's share of the layer in slot order.
    fn each(&self, kind: Kind, visit: impl FnMut(&Chunks, &Range<u32>)) {
        self.each_in(0..self.parts.len(), kind, visit);
    }

    /// [`View::each`] over some of the parts.
    fn each_in(
        &self,
        parts: Range<usize>,
        kind: Kind,
        mut visit: impl FnMut(&Chunks, &Range<u32>),
    ) {
        let parts = self.parts.get(parts).unwrap_or(&[]);
        let layers = parts.iter().map(|p| p.layers.len()).max().unwrap_or(0);
        for layer in 0..layers {
            for part in parts {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Where a world point lands in the host's frame, device pixels, drawn by `camera` into
    /// `area` (x, y, width, height) at `scale` device pixels a logical pixel.
    fn host_px(camera: &Camera, area: [f32; 4], p: Vec2, scale: f64) -> [f64; 2] {
        let px_per_unit = camera.map_px() * scale;
        [
            f64::from(area[0]) + f64::from(area[2]) / 2.0 + (p.x - camera.center.x) * px_per_unit,
            f64::from(area[1]) + f64::from(area[3]) / 2.0 - (p.y - camera.center.y) * px_per_unit,
        ]
    }

    #[test]
    fn a_picture_lands_on_whole_pixels_and_the_drawing_where_it_was() {
        // A centred dialog's area: half a pixel off, a fraction of a pixel wide.
        let area = [60.5, 160.5, 619.33, 630.0];
        let placed = whole_pixels([area[0], area[1]], [area[2], area[3]]);
        assert_eq!(placed, [61.0, 161.0, 619.0, 630.0]);
        let camera = Camera {
            map: None,
            map_ratio: 1.0,
            center: Vec2::new(1234.5, -87.25),
            scale: 3.7,
            width: 400.0,
            height: 420.0,
            twist: 0.0,
        };
        for scale in [1.0, 1.25] {
            let moved = camera_at(&camera, area, placed, scale);
            for p in [
                Vec2::new(1234.5, -87.25),
                Vec2::new(1300.0, -10.0),
                Vec2::new(1100.0, -200.0),
            ] {
                let [hx, hy] = host_px(&camera, area, p, scale);
                let [px, py] = host_px(&moved, placed, p, scale);
                assert!(
                    (hx - px).abs() < 1e-6 && (hy - py).abs() < 1e-6,
                    "{p:?} at {scale}: {hx}, {hy} against {px}, {py}"
                );
            }
        }
        // An area on whole pixels is its own place, its camera as it was.
        let whole = [10.0, 20.0, 300.0, 200.0];
        assert_eq!(whole_pixels([10.0, 20.0], [300.0, 200.0]), whole);
        assert_eq!(camera_at(&camera, whole, whole, 1.0), camera);
    }
}
