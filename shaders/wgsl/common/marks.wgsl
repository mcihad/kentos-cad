// Distance functions of the point marks, in device pixels (the web's point
// symbols: apps/web/src/render/webgpu/shaders.ts, pointFs).

// Signed distance to an equilateral triangle of circumradius-like size r, pointing up.
fn sd_triangle(q: vec2<f32>, r: f32) -> f32 {
    let k = 1.7320508;
    var p = q;
    p.x = abs(p.x) - r;
    p.y = p.y + r / k;
    if (p.x + k * p.y > 0.0) {
        p = vec2<f32>(p.x - k * p.y, -k * p.x - p.y) / 2.0;
    }
    p.x = p.x - clamp(p.x, -2.0 * r, 0.0);
    return -length(p) * sign(p.y);
}

// Coverage (0..1) of mark `shape` (0 ring, 1 cross, 2 triangle) of diameter
// `size` at `p`, both in device pixels; `dpi` scales the stroke.
fn mark_coverage(p: vec2<f32>, size: f32, shape: u32, dpi: f32) -> f32 {
    let r = size * 0.5 - 1.2 * dpi;
    let stroke_width = 0.65 * dpi;
    var d: f32;
    if (shape == 1u) {
        let q = abs(p);
        d = min(length(vec2<f32>(max(q.x - r, 0.0), q.y)), length(vec2<f32>(q.x, max(q.y - r, 0.0))));
    } else if (shape == 2u) {
        d = abs(sd_triangle(p + vec2<f32>(0.0, r * 0.18), r * 0.95));
    } else {
        d = abs(length(p) - r);
    }
    let stroke = 1.0 - smoothstep(stroke_width - 0.6, stroke_width + 0.6, d);
    // Rings and triangles also mark their exact spot with a dot.
    let centre_dot = 1.0 - smoothstep(-0.6, 0.6, length(p) - dpi);
    if (shape == 1u) {
        return stroke;
    }
    return max(stroke, centre_dot);
}
