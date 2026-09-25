// Pipeline `marker`: one instance per point object, a screen-sized mark
// (ring, cross or triangle; the layer's point style) drawn from distance
// functions on a small quad.

struct MarkerOut {
    @builtin(position) position: vec4<f32>,
    // Device pixels from the mark's centre, y up.
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) size: f32,
    @location(2) @interpolate(flat) shape: u32,
    @location(3) @interpolate(flat) color: vec4<f32>,
};

@vertex
fn marker_vs(
    @builtin(vertex_index) corner: u32,
    @location(0) hi: vec2<f32>,
    @location(1) lo: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) size: f32,
    @location(4) shape: u32,
) -> MarkerOut {
    let size_px = size * frame.dpi;
    let reach = 0.5 * size_px + 1.0;
    let offset = vec2<f32>(
        select(-reach, reach, (corner & 1u) != 0u),
        select(-reach, reach, (corner & 2u) != 0u),
    );

    var out: MarkerOut;
    out.position = px_clip(world_px(hi, lo) + offset);
    out.local = offset;
    out.size = size_px;
    out.shape = shape;
    out.color = target_color(color);
    return out;
}

@fragment
fn marker_fs(v: MarkerOut) -> @location(0) vec4<f32> {
    let coverage = mark_coverage(v.local, v.size, v.shape, frame.dpi);
    if (coverage < 0.02) {
        discard;
    }
    return vec4<f32>(v.color.rgb, v.color.a * coverage);
}
