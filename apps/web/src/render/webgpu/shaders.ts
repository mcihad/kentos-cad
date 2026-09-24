/**
 * WGSL twins of the WebGL2 shaders (render/webgl2/shaders.ts): the same
 * transform, dash test and point symbols, so both backends draw alike.
 * Group 0 holds the frame, group 1 the style of the batch being drawn.
 */
export const WGSL = /* wgsl */ `
struct Frame {
  offset: vec2f,     // camera centre, origin-relative
  scale: vec2f,      // clip units per metre (x, y)
  pxPerUnit: f32,    // device px per metre
  dpr: f32,
  viewport: vec2f,   // device px
};
struct Style {
  color: vec4f,
  dash: vec4f,       // on, off, on, off (CSS px); all zero = solid
  size: f32,         // point symbol diameter (CSS px)
  shape: u32,        // 0 ring, 1 cross, 2 triangle
  pad: vec2f,
};
@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> style: Style;

fn toClip(p: vec2f) -> vec4f {
  return vec4f((p - frame.offset) * frame.scale, 0.0, 1.0);
}

// ── Lines ──
struct LineOut {
  @builtin(position) pos: vec4f,
  @location(0) dist: f32,
};
@vertex fn lineVs(@location(0) p: vec2f, @location(1) d: f32) -> LineOut {
  var o: LineOut;
  o.pos = toClip(p);
  o.dist = d;
  return o;
}
@fragment fn lineFs(i: LineOut) -> @location(0) vec4f {
  let dash = style.dash * frame.dpr;
  let len = dash.x + dash.y + dash.z + dash.w;
  if (len > 0.0) {
    let t = (i.dist * frame.pxPerUnit) % len;
    let on = t < dash.x || (t >= dash.x + dash.y && t < dash.x + dash.y + dash.z);
    if (!on) { discard; }
  }
  return style.color;
}

// ── Fills ──
@vertex fn fillVs(@location(0) p: vec2f) -> @builtin(position) vec4f {
  return toClip(p);
}
@fragment fn fillFs() -> @location(0) vec4f {
  return style.color;
}

// ── Points: one instanced quad per symbol (WebGPU has no point size) ──
struct PointOut {
  @builtin(position) pos: vec4f,
  @location(0) local: vec2f, // device px from the centre, y up
};
@vertex fn pointVs(@builtin(vertex_index) vi: u32, @location(0) c: vec2f) -> PointOut {
  var corners = array<vec2f, 6>(
    vec2f(-0.5, -0.5), vec2f(0.5, -0.5), vec2f(0.5, 0.5),
    vec2f(-0.5, -0.5), vec2f(0.5, 0.5), vec2f(-0.5, 0.5),
  );
  let k = corners[vi];
  let sizePx = style.size * frame.dpr;
  let clip = toClip(c);
  var o: PointOut;
  o.pos = vec4f(clip.xy + k * sizePx * 2.0 / frame.viewport, 0.0, 1.0);
  o.local = k * sizePx;
  return o;
}

fn sdTriangle(q: vec2f, r: f32) -> f32 {
  let k = 1.7320508;
  var p = q;
  p.x = abs(p.x) - r;
  p.y = p.y + r / k;
  if (p.x + k * p.y > 0.0) { p = vec2f(p.x - k * p.y, -k * p.x - p.y) / 2.0; }
  p.x = p.x - clamp(p.x, -2.0 * r, 0.0);
  return -length(p) * sign(p.y);
}

@fragment fn pointFs(i: PointOut) -> @location(0) vec4f {
  let p = i.local;
  let sizePx = style.size * frame.dpr;
  let R = sizePx * 0.5 - 1.2 * frame.dpr;
  let lw = 0.65 * frame.dpr;
  var d: f32;
  if (style.shape == 1u) {
    let q = abs(p);
    d = min(length(vec2f(max(q.x - R, 0.0), q.y)), length(vec2f(q.x, max(q.y - R, 0.0))));
  } else if (style.shape == 2u) {
    d = abs(sdTriangle(p + vec2f(0.0, R * 0.18), R * 0.95));
  } else {
    d = abs(length(p) - R);
  }
  let stroke = 1.0 - smoothstep(lw - 0.6, lw + 0.6, d);
  let dotA = 1.0 - smoothstep(-0.6, 0.6, length(p) - 1.0 * frame.dpr);
  var a = stroke;
  if (style.shape != 1u) { a = max(stroke, dotA); }
  if (a < 0.02) { discard; }
  return vec4f(style.color.rgb, style.color.a * a);
}
`;
