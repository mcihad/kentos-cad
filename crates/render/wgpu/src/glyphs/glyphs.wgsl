// The drawing's glyphs (docs/adr/9212): one instanced quad a glyph over its signed distance field, the edge and the
// halo from one sample, smoothed over a device pixel. EM_PX and SPREAD_PX are written before this source
// (`glyphs/gpu.rs`), from the field's own constants.
struct Frame {
  offset: vec2f,     // camera centre from the glyph layer's origin, metres: the float32 high part
  grow: vec2f,       // x: how much the pixel texts grow while the view zooms (the styled frame's clip scale's place)
  pxPerM: f32,       // device px per metre
  dpr: f32,
  viewport: vec2f,   // device px
  offsetLo: vec2f,   // the camera centre's low part
  twist: vec2f,      // the view's twist as cos − 1 and sin; zero with the north up
};
@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var pages: texture_2d_array<f32>;
@group(0) @binding(2) var pageSmp: sampler;
// The glyphs' outlines (glyphs/vector.rs, docs/adr/9240): a glyph shown past VECTOR_EM_PX an em is read from them.
@group(0) @binding(3) var<storage, read> programs: array<f32>;

struct GlyphOut {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) @interpolate(flat) page: u32,
  @location(2) @interpolate(flat) color: vec4f,
  @location(3) @interpolate(flat) halo: vec4f,
  // The device px a field px spans, and the halo's width each side in device px.
  @location(4) @interpolate(flat) k: vec2f,
  // Read from its outline (docs/adr/9240): the point in ems from the glyph's origin (y up), where its program starts
  // plus one (0: from its field) and the halo's width in ems.
  @location(5) em: vec2f,
  @location(6) @interpolate(flat) program: u32,
  @location(7) @interpolate(flat) haloEm: f32,
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
    @location(6) halo: vec4f, @location(7) haloPx: f32, @location(8) flags: u32, @location(9) program: u32) -> GlyphOut {
  var corners = array<vec2f, 6>(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 1.0));
  let px = (flags & 0x10000u) != 0u;
  let solid = (flags & 0x40000u) != 0u;
  let triangle = (flags & 0x80000u) != 0u;
  // A triangle is the square's first half; its second collapses onto a corner.
  var c = corners[vi];
  if (triangle && vi >= 3u) { c = corners[0]; }
  let screen = (flags & 0x100000u) != 0u;
  var k = frame.pxPerM;
  if (px) { k = frame.dpr * frame.grow.x; }
  // A screen's mark neither grows with a gesture nor turns with the view.
  if (screen) { k = frame.dpr; }
  // A flat quad grows a device pixel each side, so its edges are smoothed outside it as well as in: its sides' lengths
  // in device pixels, and the unit square's corners moved out by a pixel of each.
  var lens = vec2f(1.0);
  if (solid && !triangle) {
    lens = max(vec2f(length(vec2f(m.x, m.y)) * quad.z, length(vec2f(m.z, m.w)) * quad.w) * k, vec2f(1e-3));
    c = c * (1.0 + 2.0 / lens) - 1.0 / lens;
  }
  let e = quad.xy + c * quad.zw;
  var d = (vec2f(m.x * e.x + m.z * e.y, m.y * e.x + m.w * e.y) + offset) * k;
  if (!px || (flags & 0x20000u) != 0u) { d = twisted(d); }
  // The anchor less the camera part by part: the high parts first, exact near the camera, then the low ones; a screen
  // anchor is the area's logical pixels from its top left, neither moved nor grown nor turned.
  var a = twisted(((at.xy - frame.offset) + (at.zw - frame.offsetLo)) * frame.pxPerM);
  if (screen) {
    a = vec2f(at.x * frame.dpr - 0.5 * frame.viewport.x, 0.5 * frame.viewport.y - at.y * frame.dpr);
  }
  var o: GlyphOut;
  o.pos = vec4f((a + d) / (0.5 * frame.viewport), 0.0, 1.0);
  o.em = e;
  o.program = 0u;
  o.haloEm = 0.0;
  o.page = flags & 0xcffffu;
  o.color = color;
  o.halo = halo;
  if (solid) {
    // A quad's place on its unit square and its sides in device pixels; a triangle covers what it covers.
    o.uv = c;
    o.k = lens;
    if (triangle) { o.uv = vec2f(0.5); o.k = vec2f(1e6); }
    return o;
  }
  o.uv = uv.xy + vec2f(c.x, 1.0 - c.y) * uv.zw;
  // A field px in device px: the em's device px over the field's (for a narrowed or leaning text the mean of its
  // axes). Never under the share that keeps the field's reach half a pixel: its box's edge stays clear of ink.
  let perField = max(sqrt(abs(m.x * m.w - m.z * m.y)) * k / EM_PX, 0.5 / SPREAD_PX);
  // The halo within the field's reach, its last half pixel left clear for the same reason.
  var haloK = frame.dpr;
  if (px) { haloK = k; }
  o.k = vec2f(perField, clamp(haloPx * haloK, 0.0, SPREAD_PX * perField - 0.5));
  // Past VECTOR_EM_PX device px an em the field would be magnified: the glyph is read from its outline, its halo as
  // wide as asked within the field's reach.
  let emPx = sqrt(abs(m.x * m.w - m.z * m.y)) * k;
  if (program != 0u && emPx > VECTOR_EM_PX) {
    o.program = program;
    o.haloEm = min(haloPx * haloK / emPx, HALO_EM);
  }
  return o;
}

fn vgAt(i: u32) -> f32 { return programs[i]; }
fn vgIdx(i: u32) -> u32 { return u32(programs[i] + 0.5); }

// Whether the point q (ems) is inside the glyph whose program starts at p, and its distance to the outline when it is
// not (a large number past the halo's reach): glyphs/vector.rs `read`, line for line.
fn vgRead(p: u32, q: vec2f) -> vec2f {
  let lo = vec2f(vgAt(p), vgAt(p + 1u));
  let hi = vec2f(vgAt(p + 2u), vgAt(p + 3u));
  if (q.x < lo.x || q.y < lo.y || q.x > hi.x || q.y > hi.y) { return vec2f(0.0, 1e9); }
  let nx = vgIdx(p + 4u);
  let ny = vgIdx(p + 5u);
  let bx = min(u32(max(floor((q.x - lo.x) / (hi.x - lo.x) * f32(nx)), 0.0)), nx - 1u);
  let by = min(u32(max(floor((q.y - lo.y) / (hi.y - lo.y) * f32(ny)), 0.0)), ny - 1u);
  let bin = by * nx + bx;
  let tBins = p + vgIdx(p + 6u);
  let tList = p + vgIdx(p + 7u);
  let tris = p + vgIdx(p + 8u);
  let tLast = vgIdx(tBins + bin + 1u);
  for (var n = vgIdx(tBins + bin); n < tLast; n++) {
    let t = tris + vgIdx(tList + n) * 6u;
    let a = vec2f(vgAt(t), vgAt(t + 1u));
    let b = vec2f(vgAt(t + 2u), vgAt(t + 3u));
    let c = vec2f(vgAt(t + 4u), vgAt(t + 5u));
    let s0 = (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x);
    let s1 = (c.x - b.x) * (q.y - b.y) - (c.y - b.y) * (q.x - b.x);
    let s2 = (a.x - c.x) * (q.y - c.y) - (a.y - c.y) * (q.x - c.x);
    if (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) { return vec2f(1.0, 0.0); }
  }
  let eBins = p + vgIdx(p + 9u);
  let eList = p + vgIdx(p + 10u);
  let edges = p + vgIdx(p + 11u);
  let eLast = vgIdx(eBins + bin + 1u);
  var d = 1e9;
  for (var n = vgIdx(eBins + bin); n < eLast; n++) {
    let e = edges + vgIdx(eList + n) * 4u;
    let a = vec2f(vgAt(e), vgAt(e + 1u));
    let v = vec2f(vgAt(e + 2u), vgAt(e + 3u)) - a;
    let t = clamp(dot(q - a, v) / max(dot(v, v), 1e-30), 0.0, 1.0);
    d = min(d, length(q - a - v * t));
  }
  return vec2f(0.0, d);
}

// A glyph read from its outline at the drawing's four MSAA sample points (pixels from the middle, y down): its ink by
// the samples inside, its halo by those within its width of the outline, under the ink as the field's.
fn vectorGlyph(i: GlyphOut, ex: vec2f, ey: vec2f) -> vec4f {
  var samples = array<vec2f, 4>(vec2f(-0.125, -0.375), vec2f(0.375, -0.125), vec2f(-0.375, 0.125), vec2f(0.125, 0.375));
  var fill = 0.0;
  var near = 0.0;
  for (var s = 0u; s < 4u; s++) {
    let r = vgRead(i.program - 1u, i.em + ex * samples[s].x + ey * samples[s].y);
    if (r.x > 0.5) { fill += 0.25; } else if (r.y <= i.haloEm) { near += 0.25; }
  }
  let ink = fill * i.color.a;
  var out = vec4f(i.color.rgb * ink, ink);
  if (i.haloEm > 0.0) {
    let h = (fill + near) * i.halo.a;
    out += vec4f(i.halo.rgb * h, h) * (1.0 - ink);
  }
  if (out.a < 0.002) { discard; }
  return out;
}

@fragment fn glyphFs(i: GlyphOut) -> @location(0) vec4f {
  // A pixel's step in ems across and down, taken before any branch (a derivative needs its neighbours' lanes).
  let ex = dpdx(i.em);
  let ey = dpdy(i.em);
  if ((i.page & 0x40000u) != 0u) {
    // A flat quad: covered by how far inside its nearest side the pixel is, over a pixel.
    let inside = min(i.uv, vec2f(1.0) - i.uv) * i.k;
    let cover = clamp(inside.x + 0.5, 0.0, 1.0) * clamp(inside.y + 0.5, 0.0, 1.0) * i.color.a;
    if (cover < 0.002) { discard; }
    return vec4f(i.color.rgb * cover, cover);
  }
  if (i.program != 0u) { return vectorGlyph(i, ex, ey); }
  let v = textureSampleLevel(pages, pageSmp, i.uv, i.page & 0xffffu, 0.0).r;
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
