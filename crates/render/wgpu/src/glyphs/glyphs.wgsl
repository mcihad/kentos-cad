// The drawing's glyphs (docs/adr/9212): one instanced quad a glyph over its signed distance field, the edge and the
// halo from one sample, smoothed over a device pixel. EM_PX and SPREAD_PX are written before this source
// (`glyphs/gpu.rs`), from the field's own constants.
struct Frame {
  offset: vec2f,     // camera centre from the glyph layer's origin, metres: the float32 high part
  scale: vec2f,      // unused here (the styled frame's layout)
  pxPerM: f32,       // device px per metre
  dpr: f32,
  viewport: vec2f,   // device px
  offsetLo: vec2f,   // the camera centre's low part
  twist: vec2f,      // the view's twist as cos − 1 and sin; zero with the north up
};
@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var pages: texture_2d_array<f32>;
@group(0) @binding(2) var pageSmp: sampler;

struct GlyphOut {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) @interpolate(flat) page: u32,
  @location(2) @interpolate(flat) color: vec4f,
  @location(3) @interpolate(flat) halo: vec4f,
  // The device px a field px spans, and the halo's width each side in device px.
  @location(4) @interpolate(flat) k: vec2f,
};

// The world turned by minus the view's twist about the camera's middle (docs/adr/0285 §2).
fn twisted(d: vec2f) -> vec2f {
  let c = 1.0 + frame.twist.x;
  return vec2f(d.x * c + d.y * frame.twist.y, d.y * c - d.x * frame.twist.y);
}

// An instance: its anchor's high and low parts from the layer's origin (at), the glyph's origin from the anchor in
// logical px for a text of pixels (offset), the em square into metres or logical px by columns (m), the field's box
// from the glyph's origin in ems (quad), its rectangle on its page (uv, y down), the colours, the halo's width in
// logical px and the flags: the page in the low 16 bits, the pixel unit (bit 16), turning with the view (bit 17).
@vertex fn glyphVs(@builtin(vertex_index) vi: u32, @location(0) at: vec4f, @location(1) offset: vec2f,
    @location(2) m: vec4f, @location(3) quad: vec4f, @location(4) uv: vec4f, @location(5) color: vec4f,
    @location(6) halo: vec4f, @location(7) haloPx: f32, @location(8) flags: u32) -> GlyphOut {
  var corners = array<vec2f, 6>(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 1.0));
  let c = corners[vi];
  let e = quad.xy + c * quad.zw;
  let px = (flags & 0x10000u) != 0u;
  var k = frame.pxPerM;
  if (px) { k = frame.dpr; }
  var d = (vec2f(m.x * e.x + m.z * e.y, m.y * e.x + m.w * e.y) + offset) * k;
  if (!px || (flags & 0x20000u) != 0u) { d = twisted(d); }
  // The anchor less the camera part by part: the high parts first, exact near the camera, then the low ones.
  let a = twisted(((at.xy - frame.offset) + (at.zw - frame.offsetLo)) * frame.pxPerM);
  var o: GlyphOut;
  o.pos = vec4f((a + d) / (0.5 * frame.viewport), 0.0, 1.0);
  o.uv = uv.xy + vec2f(c.x, 1.0 - c.y) * uv.zw;
  o.page = flags & 0xffffu;
  o.color = color;
  o.halo = halo;
  // A field px in device px: the em's device px over the field's (for a narrowed or leaning text the mean of its
  // axes). Never under the share that keeps the field's reach half a pixel: its box's edge stays clear of ink.
  let perField = max(sqrt(abs(m.x * m.w - m.z * m.y)) * k / EM_PX, 0.5 / SPREAD_PX);
  // The halo within the field's reach, its last half pixel left clear for the same reason.
  o.k = vec2f(perField, clamp(haloPx * frame.dpr, 0.0, SPREAD_PX * perField - 0.5));
  return o;
}

@fragment fn glyphFs(i: GlyphOut) -> @location(0) vec4f {
  let v = textureSampleLevel(pages, pageSmp, i.uv, i.page, 0.0).r;
  // Device px from the edge, inside positive.
  let d = (v - 0.5) * (2.0 * SPREAD_PX) * i.k.x;
  let ink = clamp(d + 0.5, 0.0, 1.0) * i.color.a;
  var out = vec4f(i.color.rgb * ink, ink);
  if (i.k.y > 0.0) {
    let h = clamp(d + i.k.y + 0.5, 0.0, 1.0) * i.halo.a;
    out += vec4f(i.halo.rgb * h, h) * (1.0 - ink);
  }
  if (out.a < 0.002) { discard; }
  return out;
}
