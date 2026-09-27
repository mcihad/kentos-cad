//! A picture of pixels on the screen, for the SVG editor's tracing
//! reference and Bitmap izle's source (the web drew them with `<image>`):
//! Iced's shader widget draws one textured quad, placed and turned by the
//! caller, at an opacity, clipped to the widget. Iced's canvas draws no
//! pictures without the `image` feature (a new package; not needed for one
//! quad), so the quad is drawn here as the drawing area draws its layers.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::shader::{self, Viewport};
use iced::{Rectangle, mouse, wgpu};

/// Straight-alpha RGBA pixels, with an id the GPU keeps its texture by.
#[derive(Debug)]
pub struct Pixels {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

static NEXT: AtomicU64 = AtomicU64::new(1);

impl Pixels {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Pixels {
        Pixels {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            rgba,
        }
    }
}

/// Where the picture goes in the widget: its four corners (widget pixels), in the
/// order top left, top right, bottom left, bottom right, so it may turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad(pub [[f32; 2]; 4]);

/// The widget's program: one picture at a place and opacity.
pub struct Raster {
    pub pixels: Arc<Pixels>,
    pub quad: Quad,
    pub opacity: f32,
}

/// A widget on the screen: its textures are its own (two widgets may show one picture).
pub struct View(u64);

impl Default for View {
    fn default() -> View {
        View(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl<M> shader::Program<M> for Raster {
    type State = View;
    type Primitive = Primitive;

    fn draw(&self, state: &View, _cursor: mouse::Cursor, bounds: Rectangle) -> Primitive {
        Primitive {
            view: state.0,
            pixels: self.pixels.clone(),
            quad: self.quad,
            opacity: self.opacity,
            size: [bounds.width, bounds.height],
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    view: u64,
    pixels: Arc<Pixels>,
    quad: Quad,
    opacity: f32,
    size: [f32; 2],
}

/// One picture's texture and its place's buffer.
struct Slot {
    bind: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    used: bool,
}

pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    srgb: bool,
    slots: HashMap<(u64, u64), Slot>,
}

const SHADER: &str = r"
struct U { a: vec4<f32>, b: vec4<f32>, o: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
  var uv = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
  let c = uv[i];
  // Corners: a = (top left, top right), b = (bottom left, bottom right), in clip space.
  let top = mix(u.a.xy, u.a.zw, c.x);
  let bottom = mix(u.b.xy, u.b.zw, c.x);
  var v: V;
  v.pos = vec4(mix(top, bottom, c.y), 0.0, 1.0);
  v.uv = c;
  return v;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
  let c = textureSample(tex, samp, v.uv);
  let a = c.a * u.o.x;
  return vec4(c.rgb * a, a);
}
";

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Pipeline {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.svgedit.raster"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.svgedit.raster"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.svgedit.raster"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kentos.svgedit.raster"),
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
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kentos.svgedit.raster"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        Pipeline {
            pipeline,
            layout,
            sampler,
            srgb: format.is_srgb(),
            slots: HashMap::new(),
        }
    }

    fn trim(&mut self) {
        // Pictures no widget drew this frame let their textures go.
        self.slots.retain(|_, s| std::mem::take(&mut s.used));
    }
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        let px = &self.pixels;
        if px.width == 0 || px.height == 0 || px.rgba.len() != (px.width * px.height * 4) as usize {
            return;
        }
        let srgb = pipeline.srgb;
        let layout = &pipeline.layout;
        let sampler = &pipeline.sampler;
        let slot = pipeline.slots.entry((px.id, self.view)).or_insert_with(|| {
            let size = wgpu::Extent3d {
                width: px.width,
                height: px.height,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("kentos.svgedit.raster"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: if srgb {
                    wgpu::TextureFormat::Rgba8UnormSrgb
                } else {
                    wgpu::TextureFormat::Rgba8Unorm
                },
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &px.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(px.width * 4),
                    rows_per_image: Some(px.height),
                },
                size,
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kentos.svgedit.raster"),
                size: 48,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("kentos.svgedit.raster"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });
            Slot {
                bind,
                uniform,
                used: false,
            }
        });
        slot.used = true;
        // Widget pixels to clip space (the pass's viewport is the widget).
        let [w, h] = self.size;
        let clip = |p: [f32; 2]| [p[0] / w.max(1.0) * 2.0 - 1.0, 1.0 - p[1] / h.max(1.0) * 2.0];
        let q = self.quad.0;
        let (tl, tr, bl, br) = (clip(q[0]), clip(q[1]), clip(q[2]), clip(q[3]));
        let data: [f32; 12] = [
            tl[0],
            tl[1],
            tr[0],
            tr[1],
            bl[0],
            bl[1],
            br[0],
            br[1],
            self.opacity,
            0.0,
            0.0,
            0.0,
        ];
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_buffer(&slot.uniform, 0, &bytes);
    }

    fn draw(&self, pipeline: &Pipeline, pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(slot) = pipeline.slots.get(&(self.pixels.id, self.view)) else {
            return true;
        };
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &slot.bind, &[]);
        pass.draw(0..6, 0..1);
        true
    }
}
