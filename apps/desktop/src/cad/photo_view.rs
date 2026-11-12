//! Photos in the shell (docs/adr/0225 §2): an object's attachments in
//! Öznitelikler, the hover card, the Bilgi tool's card and the photo viewer.
//! A photo is decoded once, off the UI thread (`pictures::decode`: the
//! workspace's PNG and JPEG decoders), made at most 2048 pixels on its longer
//! side and given its mip chain (box filtered), then drawn by a small wgpu
//! shader widget as a textured quad, fitted into its box (`Fit::Contain`) or
//! filling it (`Fit::Cover`, cropped): a thumbnail of a 12-megapixel photo is
//! as smooth as the viewer's. The web lets the browser draw its `<img>`.
//! The decoded photos are kept by key (`asset:<id>`, `file:<path>`), the
//! newest `KEEP`; the GPU keeps a texture while it is drawn and lets it go
//! after `IDLE_FRAMES` frames without it.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{container, shader};
use iced::{Element, Length, Rectangle, mouse, wgpu};
use kentos_render_wgpu::styled::Bitmap;

/// The longest side a photo is kept at, in pixels.
pub const LONGEST: u32 = 2048;
/// How many decoded photos are kept.
const KEEP: usize = 24;
/// Frames a texture is kept without being drawn.
const IDLE_FRAMES: u64 = 600;

static NEXT_VIEW: AtomicU64 = AtomicU64::new(1);
static NEXT_PICTURE: AtomicU64 = AtomicU64::new(1);

/// A photo as it is drawn: its size and its mip chain, the first level its
/// pixels (RGBA, sRGB), each next one half the size.
pub struct Picture {
    /// Its texture's key in the pipeline: one per picture made.
    id: u64,
    pub width: u32,
    pub height: u32,
    levels: Vec<(u32, u32, Vec<u8>)>,
}

impl fmt::Debug for Picture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Picture")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// `rgba` (`w` × `h`) at half its size as the GPU's mip levels are sized
/// (`max(1, n / 2)`): each pixel the mean of the pixels under it, an odd last
/// row or column folded into the last one.
fn half(w: u32, h: u32, rgba: &[u8]) -> (u32, u32, Vec<u8>) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    // The source span of an output pixel along a side of `n` made `m`.
    let span = |i: u32, n: u32, m: u32| {
        let start = (i * 2).min(n - 1);
        let end = if i + 1 == m { n } else { (i * 2 + 2).min(n) };
        start..end
    };
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let mut sum = [0u32; 4];
            let mut n = 0u32;
            for sy in span(y, h, nh) {
                for sx in span(x, w, nw) {
                    let i = ((sy * w + sx) * 4) as usize;
                    for (c, s) in sum.iter_mut().enumerate() {
                        *s += u32::from(rgba[i + c]);
                    }
                    n += 1;
                }
            }
            let o = ((y * nw + x) * 4) as usize;
            for (c, s) in sum.iter().enumerate() {
                out[o + c] = ((s + n / 2) / n.max(1)) as u8;
            }
        }
    }
    (nw, nh, out)
}

/// A decoded photo made at most `LONGEST` pixels on its longer side, with its mip chain.
pub fn picture(bitmap: &Bitmap) -> Picture {
    let (mut w, mut h) = (bitmap.width.max(1), bitmap.height.max(1));
    let mut rgba = bitmap.rgba.clone();
    while w.max(h) > LONGEST {
        (w, h, rgba) = half(w, h, &rgba);
    }
    let mut levels = vec![(w, h, rgba)];
    while let Some((lw, lh, data)) = levels.last()
        && (*lw > 1 || *lh > 1)
    {
        let next = half(*lw, *lh, data);
        levels.push(next);
    }
    Picture {
        id: NEXT_PICTURE.fetch_add(1, Ordering::Relaxed),
        width: w,
        height: h,
        levels,
    }
}

/// Where a photo's bytes are: an embedded image's `data:` address, a linked file.
#[derive(Clone, Debug)]
pub enum Source {
    Data(String),
    File(PathBuf),
}

/// A photo's bytes read and decoded; none when they cannot be.
pub fn decode(source: &Source) -> Option<Arc<Picture>> {
    let bytes = match source {
        Source::Data(url) => {
            let (_, data) = url.split_once(";base64,")?;
            kentos_sheet::template::base64_decode(data)?
        }
        Source::File(path) => std::fs::read(path)
            .ok()
            .filter(|b| b.len() <= crate::pictures::MOST_BYTES)?,
    };
    crate::pictures::decode(&bytes).map(|b| Arc::new(picture(&b)))
}

/// A photo's place in the store.
#[derive(Clone, Debug)]
pub enum Slot {
    Loading,
    Ready(Arc<Picture>),
    Missing,
}

/// The decoded photos, by key; the newest `KEEP`.
#[derive(Debug, Default)]
pub struct Photos {
    slots: HashMap<String, (Slot, u64)>,
    tick: u64,
}

impl Photos {
    /// The photo `key` names, as far as it is known.
    pub fn get(&self, key: &str) -> Option<&Slot> {
        self.slots.get(key).map(|(s, _)| s)
    }

    /// The photo, when it is decoded.
    pub fn ready(&self, key: &str) -> Option<Arc<Picture>> {
        match self.get(key) {
            Some(Slot::Ready(p)) => Some(p.clone()),
            _ => None,
        }
    }

    /// Marks `key` asked for; whether it was not known (it is to be decoded).
    pub fn ask(&mut self, key: &str) -> bool {
        self.tick += 1;
        let tick = self.tick;
        match self.slots.get_mut(key) {
            Some((_, used)) => {
                *used = tick;
                false
            }
            None => {
                self.slots.insert(key.to_owned(), (Slot::Loading, tick));
                true
            }
        }
    }

    /// The photo decoded (or not); the oldest decoded ones go past `KEEP`.
    pub fn done(&mut self, key: String, picture: Option<Arc<Picture>>) {
        self.tick += 1;
        let slot = picture.map_or(Slot::Missing, Slot::Ready);
        self.slots.insert(key, (slot, self.tick));
        let ready = self
            .slots
            .values()
            .filter(|(s, _)| matches!(s, Slot::Ready(_)))
            .count();
        if ready > KEEP {
            let mut ages: Vec<(u64, String)> = self
                .slots
                .iter()
                .filter(|(_, (s, _))| matches!(s, Slot::Ready(_)))
                .map(|(k, (_, t))| (*t, k.clone()))
                .collect();
            ages.sort();
            for (_, k) in ages.into_iter().take(ready - KEEP) {
                self.slots.remove(&k);
            }
        }
    }

    /// Forgets every photo (another drawing: its keys mean other bytes).
    pub fn clear(&mut self) {
        self.slots.clear();
    }
}

/// How a photo takes its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// Whole, centred, the box's background around it.
    Contain,
    /// The box filled, the photo's middle cropped to it.
    Cover,
}

/// A photo in a box `width` × `height` logical pixels; while it is decoded,
/// or when it cannot be, the box's quiet background (`missing` says why
/// beside it when given).
pub fn photo<'a, M: 'a>(
    picture: Option<Arc<Picture>>,
    width: f32,
    height: f32,
    fit: Fit,
) -> Element<'a, M> {
    match picture {
        Some(picture) => shader(Program { picture, fit })
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .into(),
        None => container(iced::widget::space())
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .style(|t: &iced::Theme| container::Style {
                background: Some(t.extended_palette().background.weak.color.into()),
                border: iced::Border {
                    radius: 3.0.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            })
            .into(),
    }
}

struct Program {
    picture: Arc<Picture>,
    fit: Fit,
}

/// A photo's view in the pipeline: one per widget on the screen.
pub struct State {
    id: u64,
}

impl Default for State {
    fn default() -> Self {
        State {
            id: NEXT_VIEW.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl<M> shader::Program<M> for Program {
    type State = State;
    type Primitive = Frame;

    fn draw(&self, state: &State, _cursor: mouse::Cursor, _bounds: Rectangle) -> Frame {
        Frame {
            id: state.id,
            picture: self.picture.clone(),
            fit: self.fit,
        }
    }
}

/// One frame of a photo, handed to the pipeline.
pub struct Frame {
    id: u64,
    picture: Arc<Picture>,
    fit: Fit,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Where a frame draws: its texture (by its picture's address), its crop's
/// slot, and its viewport and scissor in the target's pixels.
struct Prepared {
    texture: u64,
    slot: u32,
    viewport: [f32; 4],
    scissor: [u32; 4],
}

struct Texture {
    bind: wgpu::BindGroup,
    used: u64,
}

/// The crops are uniforms 256 bytes apart (wgpu's dynamic offset alignment).
const STRIDE: u64 = 256;

const PHOTO_WGSL: &str = r"
struct Crop { uv: vec4<f32> };
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(1) @binding(0) var<uniform> crop: Crop;
struct Out { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> Out {
  var corners = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
  let c = corners[i];
  var o: Out;
  o.pos = vec4(c.x * 2.0 - 1.0, 1.0 - c.y * 2.0, 0.0, 1.0);
  o.uv = mix(crop.uv.xy, crop.uv.zw, c);
  return o;
}
@fragment fn fs(o: Out) -> @location(0) vec4<f32> { return textureSample(tex, samp, o.uv); }
";

/// The photos' pipeline, its textures and this frame's crops.
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    crop_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    crops: wgpu::Buffer,
    crop_bind: wgpu::BindGroup,
    capacity: u32,
    /// This frame's crops, slot by slot (rewritten whole when the buffer grows).
    crop_data: Vec<[f32; 4]>,
    textures: HashMap<u64, Texture>,
    frame: HashMap<u64, Prepared>,
    tick: u64,
}

impl Pipeline {
    fn crop_buffer(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        capacity: u32,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kentos.photo.crops"),
            size: STRIDE * u64::from(capacity),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.photo.crops"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(16),
                }),
            }],
        });
        (buffer, bind)
    }

    /// The picture's texture, uploaded with its mip chain the first time.
    fn texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, picture: &Picture) -> u64 {
        let key = picture.id;
        let tick = self.tick;
        if let Some(t) = self.textures.get_mut(&key) {
            t.used = tick;
            return key;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kentos.photo"),
            size: wgpu::Extent3d {
                width: picture.width,
                height: picture.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: picture.levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, (w, h, data)) in picture.levels.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(*h),
                },
                wgpu::Extent3d {
                    width: *w,
                    height: *h,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.photo"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.textures.insert(key, Texture { bind, used: tick });
        key
    }
}

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.photo"),
            source: wgpu::ShaderSource::Wgsl(PHOTO_WGSL.into()),
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.photo.texture"),
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
        let crop_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.photo.crop"),
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
            label: Some("kentos.photo"),
            bind_group_layouts: &[&texture_layout, &crop_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kentos.photo"),
            layout: Some(&layout),
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
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kentos.photo"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let capacity = 16;
        let (crops, crop_bind) = Pipeline::crop_buffer(device, &crop_layout, capacity);
        Pipeline {
            pipeline,
            texture_layout,
            crop_layout,
            sampler,
            crops,
            crop_bind,
            capacity,
            crop_data: Vec::new(),
            textures: HashMap::new(),
            frame: HashMap::new(),
            tick: 0,
        }
    }

    fn trim(&mut self) {
        self.frame.clear();
        self.crop_data.clear();
        self.tick += 1;
        let tick = self.tick;
        self.textures
            .retain(|_, t| tick.saturating_sub(t.used) <= IDLE_FRAMES);
    }
}

/// Where a picture `pw` × `ph` goes in a box `bw` × `bh` (its viewport, in the box)
/// and the part of it shown (its crop, in texture coordinates).
fn fitted(fit: Fit, pw: f32, ph: f32, bw: f32, bh: f32) -> ([f32; 4], [f32; 4]) {
    let (pa, ba) = (pw / ph.max(1.0), bw / bh.max(1.0));
    match fit {
        Fit::Contain => {
            let (w, h) = if pa > ba {
                (bw, bw / pa)
            } else {
                (bh * pa, bh)
            };
            ([(bw - w) / 2.0, (bh - h) / 2.0, w, h], [0.0, 0.0, 1.0, 1.0])
        }
        Fit::Cover => {
            let crop = if pa > ba {
                let k = ba / pa;
                [(1.0 - k) / 2.0, 0.0, (1.0 + k) / 2.0, 1.0]
            } else {
                let k = pa / ba;
                [0.0, (1.0 - k) / 2.0, 1.0, (1.0 + k) / 2.0]
            };
            ([0.0, 0.0, bw, bh], crop)
        }
    }
}

impl shader::Primitive for Frame {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let texture = pipeline.texture(device, queue, &self.picture);
        let scale = viewport.scale_factor();
        let (bx, by, bw, bh) = (
            bounds.x * scale,
            bounds.y * scale,
            bounds.width * scale,
            bounds.height * scale,
        );
        let ([x, y, w, h], crop) = fitted(
            self.fit,
            self.picture.width as f32,
            self.picture.height as f32,
            bw,
            bh,
        );
        let size = viewport.physical_size();
        // The viewport stays inside the target (wgpu's rule): a box partly outside is clipped to it.
        let (vx, vy) = ((bx + x).max(0.0), (by + y).max(0.0));
        let (vw, vh) = (
            (bx + x + w).min(size.width as f32) - vx,
            (by + y + h).min(size.height as f32) - vy,
        );
        if vw <= 0.0 || vh <= 0.0 {
            return;
        }
        // The crop follows the part of the photo the target keeps.
        let (u0, v0, u1, v1) = (crop[0], crop[1], crop[2], crop[3]);
        let du = (u1 - u0) / w.max(1e-6);
        let dv = (v1 - v0) / h.max(1e-6);
        let crop = [
            u0 + (vx - (bx + x)) * du,
            v0 + (vy - (by + y)) * dv,
            u0 + (vx + vw - (bx + x)) * du,
            v0 + (vy + vh - (by + y)) * dv,
        ];
        let slot = pipeline.crop_data.len() as u32;
        pipeline.crop_data.push(crop);
        if slot >= pipeline.capacity {
            pipeline.capacity = (pipeline.capacity * 2).max(slot + 1);
            let (buffer, bind) =
                Pipeline::crop_buffer(device, &pipeline.crop_layout, pipeline.capacity);
            pipeline.crops = buffer;
            pipeline.crop_bind = bind;
            for (i, c) in pipeline.crop_data.iter().enumerate() {
                queue.write_buffer(&pipeline.crops, STRIDE * i as u64, &crop_bytes(c));
            }
        } else {
            queue.write_buffer(
                &pipeline.crops,
                STRIDE * u64::from(slot),
                &crop_bytes(&crop),
            );
        }
        let scissor = [
            bx.max(0.0) as u32,
            by.max(0.0) as u32,
            (bx + bw).min(size.width as f32).max(0.0) as u32,
            (by + bh).min(size.height as f32).max(0.0) as u32,
        ];
        pipeline.frame.insert(
            self.id,
            Prepared {
                texture,
                slot,
                viewport: [vx, vy, vw, vh],
                scissor,
            },
        );
    }

    fn draw(&self, _pipeline: &Pipeline, _render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        false
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let Some(p) = pipeline.frame.get(&self.id) else {
            return;
        };
        let Some(texture) = pipeline.textures.get(&p.texture) else {
            return;
        };
        // The scissor is the box within the clip.
        let x0 = p.scissor[0].max(clip_bounds.x);
        let y0 = p.scissor[1].max(clip_bounds.y);
        let x1 = p.scissor[2].min(clip_bounds.x + clip_bounds.width);
        let y1 = p.scissor[3].min(clip_bounds.y + clip_bounds.height);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kentos.photo"),
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
        let [vx, vy, vw, vh] = p.viewport;
        pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
        pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &texture.bind, &[]);
        pass.set_bind_group(
            1,
            &pipeline.crop_bind,
            &[(STRIDE * u64::from(p.slot)) as u32],
        );
        pass.draw(0..6, 0..1);
    }
}

/// A crop's bytes, as the uniform takes them (little endian, as the GPU reads them).
fn crop_bytes(c: &[f32; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, v) in c.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_photo_is_kept_small_with_its_mips() {
        let bitmap = Bitmap {
            width: 5000,
            height: 3000,
            rgba: vec![128; 5000 * 3000 * 4],
        };
        let p = picture(&bitmap);
        assert_eq!((p.width, p.height), (1250, 750));
        assert_eq!(p.levels.last().map(|l| (l.0, l.1)), Some((1, 1)));
        assert!(p.levels.iter().all(|(_, _, d)| d.iter().all(|&b| b == 128)));
    }

    /// The levels are sized as the GPU sizes a texture's mips (`max(1, n >> i)`), as many as it allows.
    #[test]
    fn the_mips_are_the_gpus_sizes() {
        for (w, h) in [(320, 240), (5, 3), (1, 7), (2048, 1365), (1, 1)] {
            let p = picture(&Bitmap {
                width: w,
                height: h,
                rgba: vec![7; (w * h * 4) as usize],
            });
            let count = 32 - w.max(h).leading_zeros();
            assert_eq!(p.levels.len() as u32, count, "{w}×{h}");
            for (i, (lw, lh, data)) in p.levels.iter().enumerate() {
                assert_eq!(
                    (*lw, *lh),
                    ((w >> i).max(1), (h >> i).max(1)),
                    "{w}×{h} level {i}"
                );
                assert_eq!(data.len() as u32, lw * lh * 4);
            }
        }
        // An odd last column is folded into the last pixel: the mean of all three.
        let (w, h, out) = half(3, 1, &[0, 0, 0, 255, 30, 30, 30, 255, 60, 60, 60, 255]);
        assert_eq!((w, h), (1, 1));
        assert_eq!(out, vec![30, 30, 30, 255]);
    }

    #[test]
    fn a_photo_fits_or_fills_its_box() {
        let (v, c) = fitted(Fit::Contain, 400.0, 300.0, 100.0, 100.0);
        assert_eq!(v, [0.0, 12.5, 100.0, 75.0]);
        assert_eq!(c, [0.0, 0.0, 1.0, 1.0]);
        let (v, c) = fitted(Fit::Cover, 400.0, 300.0, 100.0, 100.0);
        assert_eq!(v, [0.0, 0.0, 100.0, 100.0]);
        assert_eq!(c, [0.125, 0.0, 0.875, 1.0]);
    }

    #[test]
    fn the_store_keeps_the_newest() {
        let mut s = Photos::default();
        let tiny = Arc::new(picture(&Bitmap {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        }));
        for k in 0..30 {
            assert!(s.ask(&format!("k{k}")));
            s.done(format!("k{k}"), Some(tiny.clone()));
        }
        assert!(s.ready("k29").is_some());
        assert!(s.ready("k0").is_none());
        assert!(!s.ask("k29"));
    }
}
