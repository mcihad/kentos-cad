// Colours arrive as the drawing writes them: sRGB-encoded, straight alpha
// (vertex format unorm8x4). A target in an sRGB format encodes on write, so it
// is handed linear values; any other target takes the values as they are
// (the browser canvas and Iced's surface without gamma correction).

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn target_color(c: vec4<f32>) -> vec4<f32> {
    if (frame.srgb_target != 0u) {
        return vec4<f32>(srgb_to_linear(c.rgb), c.a);
    }
    return c;
}
