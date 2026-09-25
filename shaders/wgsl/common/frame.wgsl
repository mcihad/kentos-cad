// KentOS CAD 2D drawing: the frame uniform and the world → screen transform
// every pipeline shares (docs/adr/0019). The binding, vertex and uniform
// layout is versioned in shaders/wgsl/cad2d.layout.json; the Rust structs
// (crates/render/wgpu/src/layout.rs) are tested against it and against this
// source.
//
// Coordinates reach the GPU as two float32 parts of an offset from the
// scene's local origin: hi = f32(d), lo = f32(d - hi), with d computed in
// float64 on the CPU. The camera centre comes the same way. Differences are
// taken part by part, so what becomes pixels is the float32 offset from the
// camera: at any zoom it is exact to far below a millimetre, however far the
// data lies from the origin. Absolute world coordinates never reach the GPU
// (CLAUDE.md §4.9).

const LAYOUT_VERSION: u32 = 1u;

struct Frame {
    // Camera centre relative to the scene origin (world units), high and low parts.
    center_hi: vec2<f32>,
    center_lo: vec2<f32>,
    // The drawing area in device pixels.
    viewport: vec2<f32>,
    // Device pixels per world unit (metre).
    px_per_unit: f32,
    // Device pixels per logical pixel (the display's scale factor).
    dpi: f32,
    // The area's colour: sRGB, straight alpha.
    background: vec4<f32>,
    // Stroke width in logical pixels.
    line_width: f32,
    // 1 when the target is an sRGB format: it encodes on write, so the shaders hand it linear values.
    srgb_target: u32,
};

@group(0) @binding(0) var<uniform> frame: Frame;

// Device pixels from the camera centre (y up) of a point given as hi/lo parts.
fn world_px(hi: vec2<f32>, lo: vec2<f32>) -> vec2<f32> {
    let d = (hi - frame.center_hi) + (lo - frame.center_lo);
    return d * frame.px_per_unit;
}

// Clip position of a point given in device pixels from the area's centre (y up).
fn px_clip(px: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(px * 2.0 / frame.viewport, 0.0, 1.0);
}
