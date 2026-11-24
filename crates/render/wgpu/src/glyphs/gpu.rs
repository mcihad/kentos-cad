//! The glyph pipeline (docs/adr/9212 §3): the atlas's pages as one texture array, a set of texts as a layer of
//! instances, drawn in one call with premultiplied colour over what the frame drew before.

use std::collections::HashMap;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use kentos_native_style::uniform::StyledFrameUniform;
use wgpu::util::DeviceExt;

use super::atlas::{GlyphAtlas, Lookup};
use super::sdf::{EM_PX, SPREAD_PX};
use super::{GlyphText, GlyphUnit};
use crate::Rgba8;
use crate::styled::StyledFrame;

/// A glyph as the shader reads it (`glyphVs` in `glyphs.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
struct Instance {
    /// The anchor from the layer's origin, metres: the float32 high part, then the low part.
    at: [f32; 4],
    /// The glyph's origin from the anchor, logical px, for a text of pixels (a world text's is in its anchor).
    offset: [f32; 2],
    /// The em square into metres or logical px, by columns.
    linear: [f32; 4],
    /// The field's box from the glyph's origin in ems: left, bottom, width, height.
    quad: [f32; 4],
    /// The field's rectangle on its page, 0..1, y down.
    uv: [f32; 4],
    color: [u8; 4],
    halo: [u8; 4],
    halo_px: f32,
    /// The page (low 16 bits), the pixel unit ([`PX_UNIT`]) and turning with the view ([`TURNS`]).
    flags: u32,
}

const ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
    0 => Float32x4, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4,
    5 => Unorm8x4, 6 => Unorm8x4, 7 => Float32, 8 => Uint32
];
const PX_UNIT: u32 = 1 << 16;
const TURNS: u32 = 1 << 17;
/// A layer's origin is its first anchor on a grid this coarse: its numbers stay small, and whole in float32.
const ORIGIN_GRID: f64 = 65536.0;

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

/// The shader with the field's constants written before it.
fn source() -> String {
    format!(
        "const EM_PX: f32 = {EM_PX:?};\nconst SPREAD_PX: f32 = {SPREAD_PX:?};\n{}",
        include_str!("glyphs.wgsl")
    )
}

/// The device's side of the glyphs: the pipelines a sample count, the pages' texture and the atlas they come from.
pub struct GlyphGpu {
    atlas: GlyphAtlas,
    format: wgpu::TextureFormat,
    srgb_target: bool,
    module: wgpu::ShaderModule,
    bind_layout: wgpu::BindGroupLayout,
    layout: wgpu::PipelineLayout,
    pipelines: HashMap<u32, wgpu::RenderPipeline>,
    sampler: wgpu::Sampler,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    layers: u32,
    /// Bumped when the texture is made again for more pages: the layers' bind groups follow it.
    generation: u64,
}

/// A set of texts on the GPU: their glyphs' instances and the frame they are drawn in.
pub struct GlyphLayer {
    origin: [f64; 2],
    instances: Option<wgpu::Buffer>,
    count: u32,
    frame: wgpu::Buffer,
    bind: wgpu::BindGroup,
    bound: u64,
    /// Glyphs left out because their fields were not made before the deadline: the host builds the layer again.
    pub pending: bool,
}

impl GlyphLayer {
    /// The glyphs it draws.
    pub fn len(&self) -> u32 {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Bytes of its instances.
    pub fn bytes(&self) -> u64 {
        self.instances.as_ref().map_or(0, wgpu::Buffer::size)
    }
}

fn pages_texture(device: &wgpu::Device, side: u32, layers: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kentos.glyphs.pages"),
        size: wgpu::Extent3d { width: side, height: side, depth_or_array_layers: layers },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("kentos.glyphs.pages"),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..wgpu::TextureViewDescriptor::default()
    });
    (texture, view)
}

#[allow(clippy::too_many_arguments)]
fn write(queue: &wgpu::Queue, texture: &wgpu::Texture, page: u32, x: u32, y: u32, width: u32, height: u32, data: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: page },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width), rows_per_image: Some(height) },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
}

fn srgb_to_linear(c: u8) -> u8 {
    let c = f32::from(c) / 255.0;
    let l = if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    (l * 255.0).round() as u8
}

impl GlyphGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> GlyphGpu {
        GlyphGpu::with_atlas(device, format, GlyphAtlas::default())
    }

    /// With an atlas of its own (tests take one of small pages to fill several).
    pub fn with_atlas(device: &wgpu::Device, format: wgpu::TextureFormat, atlas: GlyphAtlas) -> GlyphGpu {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kentos.glyphs"),
            source: wgpu::ShaderSource::Wgsl(source().into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kentos.glyphs"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<StyledFrameUniform>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.glyphs"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kentos.glyphs"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let (texture, view) = pages_texture(device, atlas.page_side(), 1);
        GlyphGpu {
            atlas,
            format,
            srgb_target: format.is_srgb(),
            module,
            bind_layout,
            layout,
            pipelines: HashMap::new(),
            sampler,
            texture,
            view,
            layers: 1,
            generation: 0,
        }
    }

    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    /// Makes the pipeline of a sample count ahead of drawing (a pass cannot make it).
    pub fn ensure(&mut self, device: &wgpu::Device, samples: u32) {
        if self.pipelines.contains_key(&samples) {
            return;
        }
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kentos.glyphs"),
            layout: Some(&self.layout),
            vertex: wgpu::VertexState {
                module: &self.module,
                entry_point: Some("glyphVs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &ATTRIBUTES,
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: samples, ..wgpu::MultisampleState::default() },
            fragment: Some(wgpu::FragmentState {
                module: &self.module,
                entry_point: Some("glyphFs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: Some(PREMULTIPLIED),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        self.pipelines.insert(samples, pipeline);
    }

    /// A colour as the target takes it: on a gamma-correct target in linear light.
    fn shown(&self, c: Rgba8) -> [u8; 4] {
        let [r, g, b, a] = c.0;
        if self.srgb_target { [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), a] } else { [r, g, b, a] }
    }

    fn bind(&self, device: &wgpu::Device, frame: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kentos.glyphs"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: frame.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        })
    }

    /// The texts' glyphs as a layer; fields not yet made are made until `deadline`, those left wait (`pending`).
    pub fn layer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texts: &[GlyphText],
        deadline: Option<Instant>,
    ) -> GlyphLayer {
        let origin = texts.first().map_or([0.0; 2], |t| {
            [(t.at[0] / ORIGIN_GRID).round() * ORIGIN_GRID, (t.at[1] / ORIGIN_GRID).round() * ORIGIN_GRID]
        });
        let mut instances = Vec::new();
        for text in texts {
            let (color, halo) = (self.shown(text.color), self.shown(text.halo));
            let unit = match text.unit {
                GlyphUnit::World => 0,
                GlyphUnit::Px { turns } => PX_UNIT | if turns { TURNS } else { 0 },
            };
            for g in &text.glyphs {
                let Lookup::Ready(slot) = self.atlas.lookup(g.key, deadline) else {
                    continue;
                };
                let [a, b, c, d, e, f] = g.transform;
                // A world text's glyph is placed from the anchor in float64; a pixel text's offset is in pixels.
                let (anchor, offset) = if unit == 0 {
                    ([text.at[0] + e, text.at[1] + f], [0.0; 2])
                } else {
                    (text.at, [e as f32, f as f32])
                };
                let rel = [anchor[0] - origin[0], anchor[1] - origin[1]];
                let hi = [rel[0] as f32, rel[1] as f32];
                instances.push(Instance {
                    at: [hi[0], hi[1], (rel[0] - f64::from(hi[0])) as f32, (rel[1] - f64::from(hi[1])) as f32],
                    offset,
                    linear: [a as f32, b as f32, c as f32, d as f32],
                    quad: slot.em,
                    uv: slot.uv,
                    color,
                    halo,
                    halo_px: text.halo_px,
                    flags: slot.page | unit,
                });
            }
        }
        let pending = self.atlas.take_pending();
        self.upload(device, queue);
        let frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kentos.glyphs.frame"),
            size: std::mem::size_of::<StyledFrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        GlyphLayer {
            origin,
            instances: (!instances.is_empty()).then(|| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("kentos.glyphs.instances"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            }),
            count: instances.len() as u32,
            bind: self.bind(device, &frame),
            frame,
            bound: self.generation,
            pending,
        }
    }

    /// The fields laid since the last upload into the texture; the texture made again, from the pages kept on the
    /// CPU, when a page more is needed.
    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let pages = self.atlas.page_count();
        let uploads = self.atlas.take_uploads();
        if pages > self.layers {
            let side = self.atlas.page_side();
            self.layers = pages.next_power_of_two();
            (self.texture, self.view) = pages_texture(device, side, self.layers);
            self.generation += 1;
            for (page, bytes) in self.atlas.pages().iter().enumerate() {
                write(queue, &self.texture, page as u32, 0, 0, side, side, bytes);
            }
            return;
        }
        for u in uploads {
            write(queue, &self.texture, u.page, u.x, u.y, u.width, u.height, &u.data);
        }
    }

    /// The layer's frame: the camera from its origin (bound again to the texture if it was made again).
    pub fn prepare(&self, device: &wgpu::Device, queue: &wgpu::Queue, layer: &mut GlyphLayer, frame: &StyledFrame) {
        if layer.bound != self.generation {
            layer.bind = self.bind(device, &layer.frame);
            layer.bound = self.generation;
        }
        let center = [
            frame.center[0] + frame.origin[0] - layer.origin[0],
            frame.center[1] + frame.origin[1] - layer.origin[1],
        ];
        let uniform = crate::styled::gpu::frame_uniform(&StyledFrame { center, ..*frame });
        queue.write_buffer(&layer.frame, 0, bytemuck::bytes_of(&uniform));
    }

    /// Draws the layer; its sample count's pipeline must be made ([`GlyphGpu::ensure`]).
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, layer: &GlyphLayer, samples: u32) {
        let (Some(instances), Some(pipeline)) = (&layer.instances, self.pipelines.get(&samples)) else {
            return;
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &layer.bind, &[]);
        pass.set_vertex_buffer(0, instances.slice(..));
        pass.draw(0..6, 0..layer.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instance_is_as_the_shader_reads_it() {
        assert_eq!(std::mem::size_of::<Instance>(), 88);
        let last = ATTRIBUTES.last().expect("attributes");
        assert_eq!(last.offset + last.format.size(), 88);
    }

    /// The shader parses and validates with the field's constants written before it.
    #[test]
    fn the_shader_validates() {
        let module = naga::front::wgsl::parse_str(&source()).expect("parses");
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
            .validate(&module)
            .expect("validates");
    }
}
