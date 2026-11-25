// Pipeline `line`: one instance per straight segment (curves come
// tessellated), drawn as a triangle strip over the stroke's box. The fragment
// takes its coverage from the distance to the segment, so strokes are
// anti-aliased without multisampling and ends and joins are round.
//
// Where a segment meets the one before or after it (a path's corner, or a long
// segment cut into pieces), each draws its own side of the joint's bisector
// (docs/adr/9226, 9242): the union of the two, so a seen-through line is not
// drawn twice there. An end without a join stays round.

struct LineOut {
    @invariant @builtin(position) position: vec4<f32>,
    // Device pixels from the segment's start: along it (x) and across it (y).
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) seg_len: f32,
    @location(2) @interpolate(flat) half_width: f32,
    @location(3) @interpolate(flat) color: vec4<f32>,
    // The joins' sides: the normals of the start's and the end's bisectors in
    // the segment's own axes (along, across); zero where the end is whole.
    @location(4) @interpolate(flat) joins: vec4<f32>,
};

// How far past the stroke's edge the box reaches, in device pixels: room for the soft edge.
const FEATHER: f32 = 1.0;

// A join's turn from its code: 1024 steps a half turn, 1024 straight on; 0 is no join.
fn line_turn(code: u32) -> f32 {
    return (f32(code) - 1024.0) * (3.14159265 / 1024.0);
}

@vertex
fn line_vs(
    @builtin(vertex_index) corner: u32,
    @location(0) a_hi: vec2<f32>,
    @location(1) a_lo: vec2<f32>,
    @location(2) b_hi: vec2<f32>,
    @location(3) b_lo: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) joins: u32,
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
    // A segment too short for its two bisectors to keep apart within its box
    // draws its ends whole, as before.
    let cs = joins & 2047u;
    let ce = (joins >> 11u) & 2047u;
    let ts = line_turn(cs);
    let te = line_turn(ce);
    let spread = select(0.0, abs(tan(0.5 * ts)), cs != 0u) + select(0.0, abs(tan(0.5 * te)), ce != 0u);
    var sides = vec4<f32>(0.0);
    if (seg_len > reach * spread + 1.0) {
        if (cs != 0u) {
            sides = vec4<f32>(normalize(vec2<f32>(1.0 + cos(ts), -sin(ts))), sides.zw);
        }
        if (ce != 0u) {
            sides = vec4<f32>(sides.xy, normalize(vec2<f32>(1.0 + cos(te), sin(te))));
        }
    }
    out.joins = sides;
    return out;
}

@fragment
fn line_fs(v: LineOut) -> @location(0) vec4<f32> {
    // The segment before draws behind the start's bisector, the one after beyond
    // the end's; a hundredth of a pixel on the bisector is drawn by both, so no
    // pixel is left out.
    if (dot(v.local, v.joins.xy) < -0.01) {
        discard;
    }
    if (dot(vec2<f32>(v.local.x - v.seg_len, v.local.y), v.joins.zw) > 0.01) {
        discard;
    }
    let nearest = clamp(v.local.x, 0.0, v.seg_len);
    let dist = length(vec2<f32>(v.local.x - nearest, v.local.y));
    // A box filter one pixel wide across the stroke's edge.
    let coverage = clamp(v.half_width + 0.5 - dist, 0.0, 1.0);
    if (coverage <= 0.0) {
        discard;
    }
    return vec4<f32>(v.color.rgb, v.color.a * coverage);
}
