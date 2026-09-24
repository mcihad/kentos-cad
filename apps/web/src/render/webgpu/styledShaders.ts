/**
 * WGSL twins of the styled WebGL2 shaders (../webgl2/styledShaders.ts):
 * strokes, solid/hatch/tile fills and markers, with the same distance
 * fields, dash test and unit handling. Group 0 is the frame (shared with
 * the plain pipelines), group 1 the batch style, group 2 the atlas.
 * The atlas has one level (images are drawn at their shown size), so it
 * is sampled with textureSampleLevel, which needs no uniform control flow.
 */
export const STYLED_WGSL = /* wgsl */ `
struct Frame {
  offset: vec2f,     // camera centre, origin-relative metres
  scale: vec2f,
  pxPerM: f32,       // device px per metre
  dpr: f32,
  viewport: vec2f,   // device px
};
struct SStyle {
  color: vec4f,
  stroke: vec4f,
  dash0: vec4f,
  dash1: vec4f,
  rect: vec4f,
  a: vec4f,
  b: vec4f,
  c: vec4f,
  flags: vec4u,      // unit, cap | kind, shape, fit
};
@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> st: SStyle;
@group(2) @binding(0) var atlasTex: texture_2d<f32>;
@group(2) @binding(1) var atlasSmp: sampler;

fn toPx(p: vec2f) -> vec2f { return (p - frame.offset) * frame.pxPerM; }
fn pxToClip(px: vec2f) -> vec4f { return vec4f(px / (0.5 * frame.viewport), 0.0, 1.0); }
fn unitK() -> f32 { if (st.flags.x == 0u) { return frame.pxPerM; } return frame.dpr; }
fn fmod(x: f32, y: f32) -> f32 { return x - y * floor(x / y); }

fn corner(i: u32) -> vec2f {
  var c = array<vec2f, 6>(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 1.0));
  return c[i];
}

// Dash coverage: s in device px, pattern (raw units) scaled by k; total/on/offset as for WebGL2.
fn dashCover(s: f32, k: f32, totalRaw: f32, onShare: f32, offsetRaw: f32) -> f32 {
  let total = totalRaw * k;
  if (total <= 0.0) { return 1.0; }
  if (total < 4.0) { return onShare; }
  let t = fmod(s + offsetRaw * k, total);
  let d = array<f32, 8>(st.dash0.x, st.dash0.y, st.dash0.z, st.dash0.w, st.dash1.x, st.dash1.y, st.dash1.z, st.dash1.w);
  var acc = 0.0;
  for (var i = 0u; i < 8u; i++) {
    let l = d[i] * k;
    if (t < acc + l) {
      if ((i & 1u) == 1u) { return 0.0; }
      return clamp(min(t - acc, acc + l - t) + 0.5, 0.0, 1.0);
    }
    acc += l;
  }
  return 0.0;
}

// ── Strokes: a = (width, dashTotal, dashOn, dashOffset), flags = (unit, cap) ──
struct StrokeOut {
  @builtin(position) pos: vec4f,
  @location(0) local: vec2f,
  @location(1) len: f32,
  @location(2) s0: f32,
  @location(3) @interpolate(flat) ends: u32,
  @location(4) @interpolate(flat) halfW: f32,
};
@vertex fn strokeVs(@builtin(vertex_index) vi: u32, @location(0) seg: vec4f, @location(1) mt: vec2f) -> StrokeOut {
  let halfW = max(st.a.x * unitK(), 1.0) * 0.5;
  let blur = max(st.b.x * unitK(), 1.0);
  let A = toPx(seg.xy);
  let B = toPx(seg.zw);
  let d = B - A;
  let len = length(d);
  var dir = vec2f(1.0, 0.0);
  if (len > 1e-4) { dir = d / len; }
  let n = vec2f(-dir.y, dir.x);
  let c = corner(vi);
  let ext = halfW + 0.5 * blur + 1.0;
  let along = mix(-ext, len + ext, c.x);
  let across = mix(-ext, ext, c.y);
  var o: StrokeOut;
  o.pos = pxToClip(A + dir * along + n * across);
  o.local = vec2f(along, across);
  o.len = len;
  o.s0 = mt.x * frame.pxPerM;
  o.ends = u32(mt.y + 0.5);
  o.halfW = halfW;
  return o;
}
// Coverage t px inside an edge: a one-pixel ramp, or a smooth fade over the blur width (see the GLSL twin).
fn cover(t: f32) -> f32 {
  let blur = max(st.b.x * unitK(), 1.0);
  let c = clamp(t / blur + 0.5, 0.0, 1.0);
  if (blur > 1.0) { return smoothstep(0.0, 1.0, c); }
  return c;
}
fn capCover(x: f32, y: f32, cap: u32, halfW: f32) -> f32 {
  if (cap == 1u) { return cover(halfW - length(vec2f(x, y))); }
  let body = cover(halfW - y);
  if (cap == 2u) { return body * cover(halfW - x); }
  return body * cover(-x);
}
@fragment fn strokeFs(i: StrokeOut) -> @location(0) vec4f {
  let x = i.local.x;
  let y = abs(i.local.y);
  var a: f32;
  if (x < 0.0) {
    var cap = 1u;
    if ((i.ends & 1u) != 0u) { cap = st.flags.y; }
    a = capCover(-x, y, cap, i.halfW);
  } else if (x > i.len) {
    var cap = 1u;
    if ((i.ends & 2u) != 0u) { cap = st.flags.y; }
    a = capCover(x - i.len, y, cap, i.halfW);
  } else {
    a = cover(i.halfW - y);
  }
  a *= dashCover(i.s0 + x, unitK(), st.a.y, st.a.z, st.a.w);
  if (a < 0.004) { discard; }
  return vec4f(st.color.rgb, st.color.a * a);
}

// ── Areas ──
struct AreaOut {
  @builtin(position) pos: vec4f,
  @location(0) world: vec2f,
};
@vertex fn areaVs(@location(0) p: vec2f) -> AreaOut {
  var o: AreaOut;
  o.pos = pxToClip(toPx(p));
  o.world = p;
  return o;
}
@fragment fn solidFs(i: AreaOut) -> @location(0) vec4f {
  return st.color;
}
// Hatch: a = (dir.x, dir.y, spacing, width), b = (offset, dashTotal, dashOn, dashOffset), flags.x = unit.
@fragment fn hatchFs(i: AreaOut) -> @location(0) vec4f {
  let screen = st.flags.x == 1u;
  var p = i.world;
  var k = frame.pxPerM;
  if (screen) { p = i.pos.xy * vec2f(1.0, -1.0) / frame.dpr; k = frame.dpr; }
  let dir = st.a.xy;
  let n = vec2f(-dir.y, dir.x);
  let halfW = max(st.a.w * k, 1.0) * 0.5;
  let gap = st.a.z * k;
  var a: f32;
  if (gap < 3.0) {
    // An even tint that fades from the hairline look to the paper's coverage (see the GLSL twin).
    let hair = min(1.0, 2.0 * halfW / max(gap, 1e-4));
    let paper = min(1.0, st.a.w / max(st.a.z, 1e-9));
    a = mix(paper, hair, clamp((gap - 1.0) / 2.0, 0.0, 1.0));
  } else {
    let u = dot(p, n) - st.b.x;
    let d = abs(fract(u / st.a.z + 0.5) - 0.5) * gap;
    a = clamp(halfW + 0.5 - d, 0.0, 1.0);
    a *= dashCover(dot(p, dir) * k, k, st.b.y, st.b.z, st.b.w);
  }
  if (a < 0.004) { discard; }
  return vec4f(st.color.rgb, st.color.a * a);
}
// Tile: rect, a = (tile.x, tile.y, cos, sin), b = (shift.x, shift.y, opacity, 0), flags.x = unit. Premultiplied out.
@fragment fn tileFs(i: AreaOut) -> @location(0) vec4f {
  var p = i.world;
  if (st.flags.x == 1u) { p = i.pos.xy * vec2f(1.0, -1.0) / frame.dpr; }
  let r = st.a.zw;
  p = vec2f(r.x * p.x + r.y * p.y, -r.y * p.x + r.x * p.y) - st.b.xy;
  let f = fract(p / st.a.xy);
  let inset = vec2f(0.5 / 2048.0);
  let uv = st.rect.xy + inset + vec2f(f.x, 1.0 - f.y) * (st.rect.zw - 2.0 * inset);
  let c = textureSampleLevel(atlasTex, atlasSmp, uv, 0.0) * st.b.z;
  if (c.a < 0.004) { discard; }
  return c;
}

// ── Markers: color = fill, stroke, rect, a = (offset.xy, anchor.xy), b = (aspect, strokeW, opacity, 0), flags = (unit, kind, shape, fit) ──
struct MarkerOut {
  @builtin(position) pos: vec4f,
  @location(0) local: vec2f,
  @location(1) @interpolate(flat) hs: vec2f,
  @location(2) @interpolate(flat) sw: f32,
};
@vertex fn markerVs(@builtin(vertex_index) vi: u32, @location(0) i0: vec4f, @location(1) i1: f32) -> MarkerOut {
  let k = unitK();
  var w = i0.w * k;
  var h = i1 * k;
  let fit = st.flags.w;
  if (fit == 1u) { h = w * st.b.x; } else if (fit == 2u) { w = h / max(st.b.x, 1e-6); } else if (h <= 0.0) { h = w; }
  var sw = 0.0;
  if (st.b.y > 0.0) { sw = max(st.b.y * k, 1.0); }
  let pad = sw * 0.5 + 1.5;
  let c = corner(vi) - vec2f(0.5);
  // A triangle sits on its centroid: its apex rises above a square box (see the GLSL twin).
  var box = vec2f(w, h);
  if (st.flags.y == 0u && st.flags.z == 5u) { box.y = max(h, min(w, h) * 1.1547005); }
  let local = c * (box + 2.0 * pad);
  let q = local - st.a.zw * vec2f(w, h) + st.a.xy * k;
  let ca = cos(i0.z);
  let sa = sin(i0.z);
  var o: MarkerOut;
  o.pos = pxToClip(toPx(i0.xy) + vec2f(ca * q.x - sa * q.y, sa * q.x + ca * q.y));
  o.local = local;
  o.hs = vec2f(w, h) * 0.5;
  o.sw = sw;
  return o;
}

// Convex shapes use mitered fields (see the GLSL twin).
fn mBox(p: vec2f, b: vec2f) -> f32 { let d = abs(p) - b; return max(d.x, d.y); }
fn sdSeg(p: vec2f, a: vec2f, b: vec2f) -> f32 { let pa = p - a; let ba = b - a; let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0); return length(pa - ba * h); }
fn mRhombus(q: vec2f, b: vec2f) -> f32 { let p = abs(q); return (p.x * b.y + p.y * b.x - b.x * b.y) / length(b); }
fn mNgon(p: vec2f, a: f32, n: f32, a0: f32) -> f32 {
  if (dot(p, p) < 1e-12) { return -a; }
  let s = 6.2831853 / n;
  var t = atan2(p.y, p.x) - a0;
  t -= s * floor(t / s + 0.5);
  return length(p) * cos(t) - a;
}
fn sdStar(q: vec2f, r: f32, rf: f32) -> f32 {
  let k1 = vec2f(0.809016994375, -0.587785252292);
  let k2 = vec2f(-0.809016994375, -0.587785252292);
  var p = vec2f(abs(q.x), q.y);
  p -= 2.0 * max(dot(k1, p), 0.0) * k1;
  p -= 2.0 * max(dot(k2, p), 0.0) * k2;
  p.x = abs(p.x);
  p.y -= r;
  let ba = rf * vec2f(-k1.y, k1.x) - vec2f(0.0, 1.0);
  let h = clamp(dot(p, ba) / dot(ba, ba), 0.0, r);
  return length(p - ba * h) * sign(p.y * ba.x - p.x * ba.y);
}
// Gear and open arc (see the GLSL twin).
fn sdGear(p: vec2f, r: f32, n: f32, depth: f32) -> f32 {
  let rb = r * (1.0 - depth);
  if (dot(p, p) < 1e-12) { return -rb; }
  let a = 6.2831853 / n;
  let k = floor(atan2(p.y, p.x) / a + 0.5) * a;
  let q = vec2f(cos(k) * p.x + sin(k) * p.y, -sin(k) * p.x + cos(k) * p.y);
  let hw = 0.25 * a * (rb + 0.5 * (r - rb));
  let x0 = rb - 0.5 * (r - rb);
  return min(length(p) - rb, mBox(q - vec2f(0.5 * (x0 + r), 0.0), vec2f(0.5 * (r - x0), hw)));
}
fn sdArc(q: vec2f, r: f32, ap: f32) -> f32 {
  let p = vec2f(abs(q.x), q.y);
  let sc = vec2f(sin(ap), cos(ap));
  if (sc.y * p.x > sc.x * p.y) { return length(p - sc * r); }
  return abs(length(p) - r);
}
// sp: hole (share of the radius), teeth, opening (radians), tooth depth.
fn shapeBase(s: u32, p: vec2f, hs: vec2f, sp: vec4f) -> f32 {
  let r = min(hs.x, hs.y);
  switch s {
    case 0u, 1u: { return length(p) - r; }
    case 2u: { return mBox(p, vec2f(r)); }
    case 3u: { return mBox(p, hs); }
    case 4u: { return mRhombus(p, hs); }
    case 5u: { return mNgon(p, r * 0.57735027, 3.0, -1.5707963); }
    case 6u: { return mNgon(p, r, 5.0, -1.5707963); }
    case 7u: { return mNgon(p, r, 6.0, 0.0); }
    case 8u: { return mNgon(p, r, 8.0, 0.0); }
    case 9u: { return sdStar(p, r, 0.4); }
    case 10u: { return min(sdSeg(p, vec2f(-r, 0.0), vec2f(r, 0.0)), sdSeg(p, vec2f(0.0, -r), vec2f(0.0, r))); }
    case 11u: { let d = r * 0.7071068; return min(sdSeg(p, vec2f(-d), vec2f(d)), sdSeg(p, vec2f(-d, d), vec2f(d, -d))); }
    case 12u: { return sdSeg(p, vec2f(-hs.x, 0.0), vec2f(hs.x, 0.0)); }
    case 13u: { return min(sdSeg(p, vec2f(-hs.x, 0.0), vec2f(hs.x, 0.0)), min(sdSeg(p, vec2f(hs.x * 0.5, hs.y * 0.4), vec2f(hs.x, 0.0)), sdSeg(p, vec2f(hs.x * 0.5, -hs.y * 0.4), vec2f(hs.x, 0.0)))); }
    case 14u: {
      let a = vec2f(hs.x, 0.0);
      let b = vec2f(-hs.x, hs.y * 0.8);
      let c = vec2f(-hs.x, -hs.y * 0.8);
      let d = min(sdSeg(p, a, b), min(sdSeg(p, b, c), sdSeg(p, c, a)));
      let inside = p.x >= -hs.x && abs(p.y) <= (hs.x - p.x) * 0.4 * hs.y / hs.x;
      if (inside) { return -d; }
      return d;
    }
    case 15u: { return min(sdSeg(p, vec2f(-hs.x, hs.y), vec2f(hs.x, 0.0)), sdSeg(p, vec2f(hs.x, 0.0), vec2f(-hs.x, -hs.y))); }
    case 16u: { return max(length(p) - r, -p.y); }
    case 18u: { return sdGear(p, r, max(sp.y, 3.0), sp.w); }
    case 19u: { return sdArc(p, r, min(sp.z * 0.5, 3.1415927)); }
    default: { return max(length(p) - r, max(-p.x, -p.y)); }
  }
}
fn isOpen(s: u32) -> bool { return s == 10u || s == 11u || s == 12u || s == 13u || s == 15u || s == 19u; }
fn shapeDist(s: u32, p: vec2f, hs: vec2f, sp: vec4f) -> f32 {
  var d = shapeBase(s, p, hs, sp);
  if (sp.x > 0.0 && !isOpen(s)) { d = max(d, sp.x * min(hs.x, hs.y) - length(p)); }
  return d;
}
// A shape's colour (premultiplied) from its distance d in device px; q is the point in px (ring dot).
fn shapeColor(shape: u32, d: f32, q: vec2f, hs: vec2f, fill: vec4f, strokeIn: vec4f, swIn: f32) -> vec4f {
  let open = isOpen(shape);
  var stroke = strokeIn;
  if (stroke.a <= 0.0) { if (open) { stroke = fill; } else { stroke = vec4f(0.0); } }
  var sw = swIn;
  if (sw <= 0.0 && open) { sw = max(frame.dpr, 1.0); }
  var col = vec4f(0.0);
  if (!open && fill.a > 0.0) { col = vec4f(fill.rgb * fill.a, fill.a) * clamp(0.5 - d, 0.0, 1.0); }
  if (stroke.a > 0.0 && sw > 0.0) {
    let sa = clamp(sw * 0.5 + 0.5 - abs(d), 0.0, 1.0);
    let sc = vec4f(stroke.rgb * stroke.a, stroke.a) * sa;
    col = sc + col * (1.0 - sc.a);
  }
  if (shape == 1u && stroke.a > 0.0) {
    let dt = clamp(max(1.2 * frame.dpr, hs.x * 0.16) + 0.5 - length(q), 0.0, 1.0);
    let dc = vec4f(stroke.rgb * stroke.a, stroke.a) * dt;
    col = dc + col * (1.0 - dc.a);
  }
  return col;
}
fn hash3(p: vec2f) -> vec3f {
  var q = fract(vec3f(p.xyx) * vec3f(0.1031, 0.1030, 0.0973));
  q += dot(q, q.yxz + 33.33);
  return fract((q.xxy + q.yzz) * q.zyx);
}

// Pattern: color = fill, stroke, dash0 = (size, cos, sin), dash1 = (shift, jitter), rect = (half, markOffset),
// a = (markRot cos, sin, coverage, seed), b = (strokeW, tint, opacity, 0), flags = (unit, stagger, shape, reach). Premultiplied out.
@fragment fn patternFs(i: AreaOut) -> @location(0) vec4f {
  var p = i.world;
  var k = frame.pxPerM;
  if (st.flags.x == 1u) { p = i.pos.xy * vec2f(1.0, -1.0) / frame.dpr; k = frame.dpr; }
  let rot = st.dash0.zw;
  p = vec2f(rot.x * p.x + rot.y * p.y, -rot.y * p.x + rot.x * p.y) - st.dash1.xy;
  let size = st.dash0.xy;
  let shape = st.flags.z;
  var col = vec4f(0.0);
  if (min(size.x, size.y) * k < 4.0) {
    var ink = st.color;
    if (!(st.color.a > 0.0 && !isOpen(shape)) && st.stroke.a > 0.0) { ink = st.stroke; }
    let a = ink.a * st.b.y;
    col = vec4f(ink.rgb * a, a);
  } else {
    var best = 1e9;
    var bestQ = vec2f(0.0);
    let reach = i32(st.flags.w);
    let row0 = floor(p.y / size.y);
    for (var dy = -1; dy <= 1; dy++) {
      if (abs(dy) > reach) { continue; }
      let row = row0 + f32(dy);
      var sx = 0.0;
      if (st.flags.y == 1u && fmod(row, 2.0) > 0.5) { sx = 0.5 * size.x; }
      let col0 = floor((p.x - sx) / size.x);
      for (var dx = -1; dx <= 1; dx++) {
        if (abs(dx) > reach) { continue; }
        let cell = vec2f(col0 + f32(dx), row);
        let h = hash3(cell + vec2f(st.a.w * 17.31, st.a.w * 7.73));
        if (h.z > st.a.z) { continue; }
        let c = vec2f((cell.x + 0.5) * size.x + sx, (row + 0.5) * size.y) + (h.xy - 0.5) * st.dash1.zw;
        var q = p - c - st.rect.zw;
        let mr = st.a.xy;
        q = vec2f(mr.x * q.x + mr.y * q.y, -mr.y * q.x + mr.x * q.y) * k;
        let d = shapeDist(shape, q, st.rect.xy * k, st.c);
        if (d < best) { best = d; bestQ = q; }
      }
    }
    if (best > 1e8) { discard; }
    var sw = 0.0;
    if (st.b.x > 0.0) { sw = max(st.b.x * k, 1.0); }
    col = shapeColor(shape, best, bestQ, st.rect.xy * k, st.color, st.stroke, sw);
  }
  col *= st.b.z;
  if (col.a < 0.004) { discard; }
  return col;
}

@fragment fn markerFs(i: MarkerOut) -> @location(0) vec4f {
  var col = vec4f(0.0);
  if (st.flags.y == 1u) {
    let t = i.local / (2.0 * i.hs) + 0.5;
    if (any(t < vec2f(0.0)) || any(t > vec2f(1.0))) { discard; }
    col = textureSampleLevel(atlasTex, atlasSmp, st.rect.xy + vec2f(t.x, 1.0 - t.y) * st.rect.zw, 0.0);
  } else {
    let shape = st.flags.z;
    col = shapeColor(shape, shapeDist(shape, i.local, i.hs, st.c), i.local, i.hs, st.color, st.stroke, i.sw);
  }
  col *= st.b.z;
  if (col.a < 0.004) { discard; }
  return col;
}
`;
