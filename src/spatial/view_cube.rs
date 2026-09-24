//! ViewCube: model alanının köşesindeki 3B yön küpü.
//!
//! iced'in `shader` widget'ı için özel bir wgpu render pipeline'ı kullanır.
//! Küp; düşey eksende döndürülmüş model matrisi, perspektif kamera ve
//! Lambert + speküler aydınlatma ile çizilir. Derinlik testi kendi
//! `Depth32Float` dokusunda yapılır ve yalnızca widget'ın ekrandaki
//! dikdörtgenine yazılır; arka plan saydam kalır.

use iced::mouse;
use iced::wgpu;
use iced::wgpu::util::DeviceExt;
use iced::widget::shader::{self, Viewport};
use iced::{Element, Rectangle, Size};

use glam::{Mat4, Vec3};

/// Varsayılan kenar uzunluğu (piksel).
pub const SIZE: f32 = 96.0;

/// Döndürülebilir yön küpü.
///
/// ```ignore
/// ViewCube::new(self.rotation).size(96.0)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewCube {
    rotation: f32,
    size: f32,
}

impl ViewCube {
    /// `rotation`: düşey eksen etrafındaki dönüş (radyan).
    pub fn new(rotation: f32) -> Self {
        Self {
            rotation,
            size: SIZE,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Kenar uzunluğu (piksel).
    pub(crate) fn side(&self) -> f32 {
        self.size
    }
}

impl<'a, Message: 'a> From<ViewCube> for Element<'a, Message> {
    fn from(cube: ViewCube) -> Self {
        iced::widget::shader(Program {
            rotation: cube.rotation,
        })
        .width(cube.size)
        .height(cube.size)
        .into()
    }
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const FACE_COUNT: u32 = 6;
const VERTEX_COUNT: u32 = FACE_COUNT * 6;

/// Kamera ve model bilgilerini taşıyan sabit arabellek.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CubeUniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light_dir: [f32; 4],
    camera_pos: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CubeVertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
}

impl CubeVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CubeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Küpün altı yüzü: normal, renk ve dışarıdan bakıldığında saat yönünün
/// tersine sıralanmış dört köşe.
struct Face {
    normal: [f32; 3],
    color: [f32; 3],
    corners: [[f32; 3]; 4],
}

const CUBE_FACES: [Face; 6] = [
    // Ön (+Z)
    Face {
        normal: [0.0, 0.0, 1.0],
        color: [0.80, 0.83, 0.87],
        corners: [
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
    },
    // Arka (-Z)
    Face {
        normal: [0.0, 0.0, -1.0],
        color: [0.72, 0.75, 0.80],
        corners: [
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
        ],
    },
    // Sağ (+X)
    Face {
        normal: [1.0, 0.0, 0.0],
        color: [0.74, 0.77, 0.82],
        corners: [
            [1.0, -1.0, 1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [1.0, 1.0, 1.0],
        ],
    },
    // Sol (-X)
    Face {
        normal: [-1.0, 0.0, 0.0],
        color: [0.68, 0.71, 0.76],
        corners: [
            [-1.0, -1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [-1.0, 1.0, -1.0],
        ],
    },
    // Üst (+Y)
    Face {
        normal: [0.0, 1.0, 0.0],
        color: [0.52, 0.72, 0.95],
        corners: [
            [-1.0, 1.0, -1.0],
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, -1.0],
        ],
    },
    // Alt (-Y)
    Face {
        normal: [0.0, -1.0, 0.0],
        color: [0.50, 0.53, 0.58],
        corners: [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0],
            [-1.0, -1.0, 1.0],
        ],
    },
];

fn cube_vertices() -> Vec<CubeVertex> {
    let mut vertices = Vec::with_capacity(VERTEX_COUNT as usize);

    for face in &CUBE_FACES {
        let [a, b, c, d] = face.corners;

        for position in [[a, b, c], [a, c, d]] {
            for position in position {
                vertices.push(CubeVertex {
                    position,
                    normal: face.normal,
                    color: face.color,
                });
            }
        }
    }

    vertices
}

/// Küpü çizen shader programı.
struct Program {
    rotation: f32,
}

impl<Message> shader::Program<Message> for Program {
    type State = ();
    type Primitive = CubePrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        CubePrimitive {
            rotation: self.rotation,
        }
    }
}

/// Tek karelik küp çizimi.
#[derive(Debug)]
struct CubePrimitive {
    rotation: f32,
}

impl shader::Primitive for CubePrimitive {
    type Pipeline = CubePipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let aspect = (bounds.width / bounds.height.max(1.0)).max(0.1);
        // ViewCube gibi üstten, hafif eğik bakış; küp yalnızca düşey
        // eksende döner.
        let eye = Vec3::new(3.0, 2.5, 3.8);

        let projection = Mat4::perspective_rh(42f32.to_radians(), aspect, 0.1, 100.0);
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
        let model = Mat4::from_rotation_y(self.rotation);

        let uniforms = CubeUniforms {
            view_proj: (projection * view).to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            light_dir: [0.35, 0.80, 0.45, 0.0],
            camera_pos: [eye.x, eye.y, eye.z, 0.0],
        };

        pipeline.update(device, queue, &uniforms, viewport.physical_size());
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        pipeline.render(target, encoder, *clip_bounds);
    }
}

/// Küp için paylaşılan wgpu kaynakları.
struct CubePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    depth_view: wgpu::TextureView,
    depth_size: Size<u32>,
}

impl CubePipeline {
    fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniforms: &CubeUniforms,
        viewport_size: Size<u32>,
    ) {
        if self.depth_size != viewport_size {
            self.depth_view = create_depth_view(device, viewport_size);
            self.depth_size = viewport_size;
        }

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clip_bounds: Rectangle<u32>,
    ) {
        if clip_bounds.width == 0 || clip_bounds.height == 0 {
            return;
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cube.render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_viewport(
            clip_bounds.x as f32,
            clip_bounds.y as f32,
            clip_bounds.width as f32,
            clip_bounds.height as f32,
            0.0,
            1.0,
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..VERTEX_COUNT, 0..1);
    }
}

impl shader::Pipeline for CubePipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let vertices = cube_vertices();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube.vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cube.uniforms"),
            size: std::mem::size_of::<CubeUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cube.bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cube.bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cube.pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cube.shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[CubeVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            depth_view: create_depth_view(device, Size::new(1, 1)),
            depth_size: Size::new(1, 1),
        }
    }
}

fn create_depth_view(device: &wgpu::Device, size: Size<u32>) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cube.depth"),
        size: wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

const SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    light_dir: vec4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let world = uniforms.model * vec4<f32>(input.position, 1.0);

    let normal_matrix = mat3x3<f32>(
        uniforms.model[0].xyz,
        uniforms.model[1].xyz,
        uniforms.model[2].xyz,
    );

    var output: VertexOutput;
    output.clip_position = uniforms.view_proj * world;
    output.normal = normalize(normal_matrix * input.normal);
    output.color = input.color;
    output.world_position = world.xyz;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(uniforms.light_dir.xyz);
    let view_dir = normalize(uniforms.camera_pos.xyz - input.world_position);
    let half_dir = normalize(light_dir + view_dir);

    let diffuse = max(dot(normal, light_dir), 0.0);
    let specular = pow(max(dot(normal, half_dir), 0.0), 48.0);
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0);

    let ambient = vec3<f32>(0.22, 0.26, 0.32);
    let color = input.color * (ambient + diffuse)
        + vec3<f32>(specular * 0.45)
        + vec3<f32>(0.18, 0.30, 0.45) * rim * 0.6;

    return vec4<f32>(color, 1.0);
}
"#;
