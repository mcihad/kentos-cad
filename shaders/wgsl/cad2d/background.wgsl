// Pipeline `background`: the drawing area's colour, one triangle over the
// whole render-pass viewport (the area); the pass's scissor clips it.

@vertex
fn background_vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn background_fs() -> @location(0) vec4<f32> {
    return target_color(frame.background);
}
