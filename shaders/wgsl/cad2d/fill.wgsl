// Pipeline `fill`: triangulated areas (polygon fills, solid hatches), one
// colour per triangle.

struct FillOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
};

@vertex
fn fill_vs(
    @location(0) hi: vec2<f32>,
    @location(1) lo: vec2<f32>,
    @location(2) color: vec4<f32>,
) -> FillOut {
    var out: FillOut;
    out.position = px_clip(world_px(hi, lo));
    out.color = target_color(color);
    return out;
}

@fragment
fn fill_fs(v: FillOut) -> @location(0) vec4<f32> {
    return v.color;
}
