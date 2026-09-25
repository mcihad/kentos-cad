// Pipeline `line`: one instance per straight segment (curves come
// tessellated), drawn as a triangle strip over the stroke's box. The fragment
// takes its coverage from the distance to the segment, so strokes are
// anti-aliased without multisampling and ends and joins are round.

struct LineOut {
    @builtin(position) position: vec4<f32>,
    // Device pixels from the segment's start: along it (x) and across it (y).
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) seg_len: f32,
    @location(2) @interpolate(flat) half_width: f32,
    @location(3) @interpolate(flat) color: vec4<f32>,
};

// How far past the stroke's edge the box reaches, in device pixels: room for the soft edge.
const FEATHER: f32 = 1.0;

@vertex
fn line_vs(
    @builtin(vertex_index) corner: u32,
    @location(0) a_hi: vec2<f32>,
    @location(1) a_lo: vec2<f32>,
    @location(2) b_hi: vec2<f32>,
    @location(3) b_lo: vec2<f32>,
    @location(4) color: vec4<f32>,
) -> LineOut {
    let a = world_px(a_hi, a_lo);
    let b = world_px(b_hi, b_lo);
    let d = b - a;
    let seg_len = length(d);
    var dir = vec2<f32>(1.0, 0.0);
    if (seg_len > 1e-6) {
        dir = d / seg_len;
    }
    let normal = vec2<f32>(-dir.y, dir.x);
    let half_width = 0.5 * frame.line_width * frame.dpi;
    let reach = half_width + FEATHER;
    let along = select(-reach, seg_len + reach, (corner & 2u) != 0u);
    let across = select(-reach, reach, (corner & 1u) != 0u);

    var out: LineOut;
    out.position = px_clip(a + dir * along + normal * across);
    out.local = vec2<f32>(along, across);
    out.seg_len = seg_len;
    out.half_width = half_width;
    out.color = target_color(color);
    return out;
}

@fragment
fn line_fs(v: LineOut) -> @location(0) vec4<f32> {
    let nearest = clamp(v.local.x, 0.0, v.seg_len);
    let dist = length(vec2<f32>(v.local.x - nearest, v.local.y));
    // A box filter one pixel wide across the stroke's edge.
    let coverage = clamp(v.half_width + 0.5 - dist, 0.0, 1.0);
    if (coverage <= 0.0) {
        discard;
    }
    return vec4<f32>(v.color.rgb, v.color.a * coverage);
}
