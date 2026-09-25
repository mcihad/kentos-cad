//! The GPU data layout of the shared WGSL, version [`LAYOUT_VERSION`]
//! (`shaders/wgsl/cad2d.layout.json`, TODOS.md REN-04). These are the Rust
//! twins of the contract: the frame uniform, one vertex or instance struct per
//! pipeline, their attribute lists and the pipelines themselves. Tests hold
//! them to the JSON and to the WGSL as naga lays it out
//! (`tests/wgsl_contract.rs`); a browser builds the same pipelines from the
//! JSON (`scripts/wgsl/browser-check.mjs`). Changing a field here means
//! changing all three and raising the version.

use bytemuck::{Pod, Zeroable};

/// Version of the binding, vertex and uniform layout (the JSON's `version`,
/// the WGSL's `LAYOUT_VERSION`).
pub const LAYOUT_VERSION: u32 = 1;

/// The frame uniform (group 0, binding 0; WGSL `Frame`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct FrameUniform {
    /// Camera centre relative to the scene origin (world units), high and low parts.
    pub center_hi: [f32; 2],
    pub center_lo: [f32; 2],
    /// The drawing area in device pixels.
    pub viewport: [f32; 2],
    /// Device pixels per world unit.
    pub px_per_unit: f32,
    /// Device pixels per logical pixel.
    pub dpi: f32,
    /// The area's colour: sRGB, straight alpha.
    pub background: [f32; 4],
    /// Stroke width in logical pixels.
    pub line_width: f32,
    /// 1 when the target format is sRGB (the shaders then hand it linear values).
    pub srgb_target: u32,
    /// Up to the struct's 16-byte alignment (WGSL pads the same way).
    pub _pad: [u32; 2],
}

/// A vertex of a fill triangle (pipeline `fill`, step mode vertex).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct FillVertex {
    pub hi: [f32; 2],
    pub lo: [f32; 2],
    /// sRGB, straight alpha.
    pub color: [u8; 4],
}

/// A straight segment (pipeline `line`, step mode instance).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SegmentInstance {
    pub a_hi: [f32; 2],
    pub a_lo: [f32; 2],
    pub b_hi: [f32; 2],
    pub b_lo: [f32; 2],
    /// sRGB, straight alpha.
    pub color: [u8; 4],
}

/// A point mark (pipeline `marker`, step mode instance).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MarkerInstance {
    pub hi: [f32; 2],
    pub lo: [f32; 2],
    /// sRGB, straight alpha.
    pub color: [u8; 4],
    /// Diameter in logical pixels.
    pub size: f32,
    /// One of [`marker_shape`].
    pub shape: u32,
}

/// The mark a point layer draws (`PointSymbol`; the web's point symbols).
pub mod marker_shape {
    pub const RING: u32 = 0;
    pub const CROSS: u32 = 1;
    pub const TRIANGLE: u32 = 2;
}

pub const FILL_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4];

pub const SEGMENT_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Unorm8x4
];

pub const MARKER_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4, 3 => Float32, 4 => Uint32
];

pub const FILL_BUFFER: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<FillVertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &FILL_ATTRIBUTES,
};

pub const SEGMENT_BUFFER: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<SegmentInstance>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &SEGMENT_ATTRIBUTES,
};

pub const MARKER_BUFFER: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<MarkerInstance>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &MARKER_ATTRIBUTES,
};

/// A pipeline of the module as the layout file describes it.
#[derive(Clone, Debug)]
pub struct PipelineSpec {
    pub name: &'static str,
    pub vertex: &'static str,
    pub fragment: &'static str,
    pub topology: wgpu::PrimitiveTopology,
    /// Vertices per draw (or per instance) when the pipeline has no vertex buffer of its own.
    pub vertex_count: Option<u32>,
    /// `None`: the colour replaces what is there.
    pub blend: Option<wgpu::BlendState>,
    pub buffers: &'static [wgpu::VertexBufferLayout<'static>],
}

pub const BACKGROUND: PipelineSpec = PipelineSpec {
    name: "background",
    vertex: "background_vs",
    fragment: "background_fs",
    topology: wgpu::PrimitiveTopology::TriangleList,
    vertex_count: Some(3),
    blend: None,
    buffers: &[],
};

pub const FILL: PipelineSpec = PipelineSpec {
    name: "fill",
    vertex: "fill_vs",
    fragment: "fill_fs",
    topology: wgpu::PrimitiveTopology::TriangleList,
    vertex_count: None,
    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
    buffers: &[FILL_BUFFER],
};

pub const LINE: PipelineSpec = PipelineSpec {
    name: "line",
    vertex: "line_vs",
    fragment: "line_fs",
    topology: wgpu::PrimitiveTopology::TriangleStrip,
    vertex_count: Some(4),
    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
    buffers: &[SEGMENT_BUFFER],
};

pub const MARKER: PipelineSpec = PipelineSpec {
    name: "marker",
    vertex: "marker_vs",
    fragment: "marker_fs",
    topology: wgpu::PrimitiveTopology::TriangleStrip,
    vertex_count: Some(4),
    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
    buffers: &[MARKER_BUFFER],
};

/// Every pipeline of the module, in the layout file's order.
pub const PIPELINES: [PipelineSpec; 4] = [BACKGROUND, FILL, LINE, MARKER];
