// Katman çizimi and draw effects (docs/adr/0226 §2–§4): the passes that compose a layer's picture into the drawing
// (opacity, blending mode, mask) and an effect group's picture with its effects (shadow, glow, blur, inner shadow,
// inner glow), and the maps the effects are made from. Colours are premultiplied; values are sRGB-encoded as written
// (the browser's and QGIS's way). Outside a picture is transparent: every read beyond its edge is 0.
//
// One bind group serves every pass: the pass's numbers at a dynamic offset, up to six pictures and a sampler.
struct Params {
  a: vec4f,
  b: vec4f,
  c: vec4f,
  d: vec4f,
  e: vec4f,
  f: vec4f,
  g: vec4f,
  h: vec4f,
};
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var srcA: texture_2d<f32>;
@group(0) @binding(2) var srcB: texture_2d<f32>;
@group(0) @binding(3) var srcC: texture_2d<f32>;
@group(0) @binding(4) var srcD: texture_2d<f32>;
@group(0) @binding(5) var srcE: texture_2d<f32>;
@group(0) @binding(6) var srcF: texture_2d<f32>;
@group(0) @binding(7) var smp: sampler;

struct Full {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
};

// A triangle over the whole target; uv from the top left.
@vertex fn fullVs(@builtin(vertex_index) i: u32) -> Full {
  let q = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  var o: Full;
  o.pos = vec4f(q * 2.0 - 1.0, 0.0, 1.0);
  o.uv = vec2f(q.x, 1.0 - q.y);
  return o;
}

// A texel of `t` of `size` texels, 0 beyond its edge. The size is the pass's own number: asking the texture for
// it costs a read on some GPUs.
fn texel(t: texture_2d<f32>, xy: vec2i, size: vec2i) -> vec4f {
  if (any(xy < vec2i(0)) || any(xy >= size)) { return vec4f(0.0); }
  return textureLoad(t, xy, 0);
}

// ── A layer into the drawing (docs/adr/0226 §4) ──
// a = (opacity, has mask, mode, 0); srcA the layer, srcB the drawing so far (opaque), srcC the mask (red).

// The layer's pictures are the target's size: the fragment's own texel is always inside them.
fn layerFactor(xy: vec2i) -> f32 {
  var k = p.a.x;
  if (p.a.y > 0.5) { k = k * (1.0 - textureLoad(srcC, xy, 0).r); }
  return k;
}

// Normal: the layer's premultiplied colour, faded and masked, laid over the drawing by the pipeline's blending.
@fragment fn overFs(i: Full) -> @location(0) vec4f {
  let xy = vec2i(i.pos.xy);
  return textureLoad(srcA, xy, 0) * layerFactor(xy);
}

fn hardLight(cb: f32, cs: f32) -> f32 {
  if (cs <= 0.5) { return cb * 2.0 * cs; }
  let s = 2.0 * cs - 1.0;
  return cb + s - cb * s;
}

fn softD(cb: f32) -> f32 {
  if (cb <= 0.25) { return ((16.0 * cb - 12.0) * cb + 4.0) * cb; }
  return sqrt(cb);
}

// W3C Compositing and Blending Level 1's separable modes, B(cb, cs); the mode's number is its place in the list.
fn blendOne(mode: u32, cb: f32, cs: f32) -> f32 {
  switch mode {
    case 1u: { return cb * cs; }
    case 2u: { return cb + cs - cb * cs; }
    case 3u: { return hardLight(cs, cb); }
    case 4u: { return min(cb, cs); }
    case 5u: { return max(cb, cs); }
    case 6u: {
      if (cb <= 0.0) { return 0.0; }
      if (cs >= 1.0) { return 1.0; }
      return min(1.0, cb / (1.0 - cs));
    }
    case 7u: {
      if (cb >= 1.0) { return 1.0; }
      if (cs <= 0.0) { return 0.0; }
      return 1.0 - min(1.0, (1.0 - cb) / cs);
    }
    case 8u: { return hardLight(cb, cs); }
    case 9u: {
      if (cs <= 0.5) { return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb); }
      return cb + (2.0 * cs - 1.0) * (softD(cb) - cb);
    }
    case 10u: { return abs(cb - cs); }
    case 11u: { return cb + cs - 2.0 * cb * cs; }
    default: { return cs; }
  }
}

// Another mode: co = (1 − αs)·cb + αs·B(cb, cs), addition co = min(1, cb + Cs); the result replaces the drawing.
@fragment fn blendFs(i: Full) -> @location(0) vec4f {
  let xy = vec2i(i.pos.xy);
  let s = textureLoad(srcA, xy, 0);
  let cb = textureLoad(srcB, xy, 0).rgb;
  let k = layerFactor(xy);
  let alpha = s.a * k;
  if (alpha <= 0.0) { return vec4f(cb, 1.0); }
  let mode = u32(p.a.z + 0.5);
  if (mode == 12u) { return vec4f(min(vec3f(1.0), cb + s.rgb * k), 1.0); }
  let cs = clamp(s.rgb / s.a, vec3f(0.0), vec3f(1.0));
  let b = vec3f(blendOne(mode, cb.r, cs.r), blendOne(mode, cb.g, cs.g), blendOne(mode, cb.b, cs.b));
  return vec4f((1.0 - alpha) * cb + alpha * b, 1.0);
}

// ── Masks (docs/adr/0226 §3) ──
// A box per instance: its middle and half size in the target's pixels (y down), its direction (the cosine and sine
// of its turn as the screen turns, made in double precision on the CPU: a GPU's own sine may be coarse) and how far
// it grows with round corners. Coverage with a one-pixel soft edge; boxes meet by their largest. The fragment finds
// its place in the box from its own position: a rasteriser snaps a turned quad's corners to its sub-pixel grid, which
// would move an interpolated position.
// a = (target width, target height, 0, 0).

struct BoxOut {
  @builtin(position) pos: vec4f,
  @location(0) @interpolate(flat) centre: vec2f,
  @location(1) @interpolate(flat) dir: vec2f,
  @location(2) @interpolate(flat) hr: vec3f,
};

fn corner6(i: u32) -> vec2f {
  var c = array<vec2f, 6>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0));
  return c[i];
}

@vertex fn boxVs(@builtin(vertex_index) vi: u32, @location(0) centre: vec2f, @location(1) half: vec2f, @location(2) dir: vec2f, @location(3) grow: f32) -> BoxOut {
  let reach = half + vec2f(grow + 1.0);
  let local = corner6(vi) * reach;
  let px = centre + vec2f(dir.x * local.x - dir.y * local.y, dir.y * local.x + dir.x * local.y);
  var o: BoxOut;
  o.pos = vec4f(px.x / p.a.x * 2.0 - 1.0, 1.0 - px.y / p.a.y * 2.0, 0.0, 1.0);
  o.centre = centre;
  o.dir = dir;
  o.hr = vec3f(half, grow);
  return o;
}

@fragment fn boxFs(i: BoxOut) -> @location(0) vec4f {
  // The box's own axes: the pixel's centre turned back.
  let v = i.pos.xy - i.centre;
  let local = vec2f(i.dir.x * v.x + i.dir.y * v.y, -i.dir.y * v.x + i.dir.x * v.y);
  let q = abs(local) - i.hr.xy;
  let d = length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - i.hr.z;
  return vec4f(clamp(0.5 - d, 0.0, 1.0));
}

// ── Maps (docs/adr/0226 §2) ──
// An effect's map of the group's alpha is packed, four pixels a texel: row-packed, texel (j, y) holds the pixels
// 4j … 4j + 3 of row y; column-packed, texel (x, j) the pixels of rows 4j … 4j + 3 of column x. A pass along y reads
// a row-packed map, a pass along x a column-packed one: each texel's four pixels are filtered at once by the same
// taps. b.xy the map's size in pixels at its level (a pixel beyond it is 0), b.zw the source's size in texels. Every
// pass stores 8 bits. a = (the work: 2 a Gaussian blur, 3 the largest, 4 the smallest; σ or n in the level's pixels;
// a blur's weights' sum, made once on the CPU; 0).

// 1 for an output texel's pixels inside the map: row-packed (inWidth), column-packed (inHeight).
fn inWidth(j: i32) -> vec4f {
  let x = vec4f(f32(4 * j), f32(4 * j + 1), f32(4 * j + 2), f32(4 * j + 3));
  return step(x + vec4f(0.5), vec4f(p.b.x));
}

fn inHeight(j: i32) -> vec4f {
  let y = vec4f(f32(4 * j), f32(4 * j + 1), f32(4 * j + 2), f32(4 * j + 3));
  return step(y + vec4f(0.5), vec4f(p.b.y));
}

// The group's alpha, column-packed.
@fragment fn packFs(i: Full) -> @location(0) vec4f {
  let x = i32(i.pos.x);
  let j = i32(i.pos.y);
  let size = vec2i(p.b.zw);
  let v = vec4f(texel(srcA, vec2i(x, 4 * j), size).a, texel(srcA, vec2i(x, 4 * j + 1), size).a, texel(srcA, vec2i(x, 4 * j + 2), size).a, texel(srcA, vec2i(x, 4 * j + 3), size).a);
  return v * inHeight(j);
}

// A column-packed map row-packed: the output texel's four pixels from four texels, each its row's channel.
@fragment fn rowsFs(i: Full) -> @location(0) vec4f {
  let j = i32(i.pos.x);
  let y = i32(i.pos.y);
  let size = vec2i(p.b.zw);
  let c = y & 3;
  let v = vec4f(texel(srcA, vec2i(4 * j, y >> 2), size)[c], texel(srcA, vec2i(4 * j + 1, y >> 2), size)[c], texel(srcA, vec2i(4 * j + 2, y >> 2), size)[c], texel(srcA, vec2i(4 * j + 3, y >> 2), size)[c]);
  return v * inWidth(j);
}

// A row-packed map column-packed.
@fragment fn columnsFs(i: Full) -> @location(0) vec4f {
  let x = i32(i.pos.x);
  let j = i32(i.pos.y);
  let size = vec2i(p.b.zw);
  let c = x & 3;
  let v = vec4f(texel(srcA, vec2i(x >> 2, 4 * j), size)[c], texel(srcA, vec2i(x >> 2, 4 * j + 1), size)[c], texel(srcA, vec2i(x >> 2, 4 * j + 2), size)[c], texel(srcA, vec2i(x >> 2, 4 * j + 3), size)[c]);
  return v * inHeight(j);
}

// The pass's work along d, at the fragment's texel xy of a source of `size` texels. A blur's weights by recurrence,
// without an exp a tap: w(0) = 1, w(k + 1) = w(k)·q^(2k + 1), q = exp(−1/2σ²). Away from the edge the taps go in
// pairs (k, k + 1), read between their texels by the sampler's linear filter at k + w(k + 1)/(w(k) + w(k + 1)): one
// read for two taps; near it, tap by tap, 0 beyond.
fn filtered(xy: vec2i, d: vec2i, size: vec2i) -> vec4f {
  let op = i32(p.a.x + 0.5);
  let centre = textureLoad(srcA, xy, 0);
  if (op == 2) {
    let r = i32(ceil(3.0 * p.a.y));
    let q = exp(-1.0 / (2.0 * p.a.y * p.a.y));
    let q2 = q * q;
    var acc = centre;
    var w = 1.0;
    var ratio = q;
    let lo = xy - d * (r + 1);
    let hi = xy + d * (r + 1);
    if (all(lo >= vec2i(0)) && all(hi < size)) {
      let texels = vec2f(size);
      let at = vec2f(xy) + vec2f(0.5);
      for (var k = 1; k <= r; k += 2) {
        let w1 = w * ratio;
        ratio = ratio * q2;
        var w2 = w1 * ratio;
        ratio = ratio * q2;
        w = w2;
        if (k + 1 > r) { w2 = 0.0; }
        let pair = w1 + w2;
        let off = vec2f(d) * (f32(k) + w2 / pair);
        acc += pair * (textureSampleLevel(srcA, smp, (at + off) / texels, 0.0) + textureSampleLevel(srcA, smp, (at - off) / texels, 0.0));
      }
    } else {
      for (var k = 1; k <= r; k++) {
        w = w * ratio;
        ratio = ratio * q2;
        acc += w * (texel(srcA, xy + d * k, size) + texel(srcA, xy - d * k, size));
      }
    }
    return acc / p.a.z;
  }
  let n = i32(p.a.y + 0.5);
  var most = centre;
  var least = centre;
  for (var k = 1; k <= n; k++) {
    let a = texel(srcA, xy + d * k, size);
    let b = texel(srcA, xy - d * k, size);
    most = max(most, max(a, b));
    least = min(least, min(a, b));
  }
  if (op == 3) { return most; }
  return least;
}

// Along x, a column-packed map.
@fragment fn hFs(i: Full) -> @location(0) vec4f {
  let xy = vec2i(i.pos.xy);
  return filtered(xy, vec2i(1, 0), vec2i(p.b.zw)) * inHeight(xy.y);
}

// Along y, a row-packed map.
@fragment fn vFs(i: Full) -> @location(0) vec4f {
  let xy = vec2i(i.pos.xy);
  return filtered(xy, vec2i(0, 1), vec2i(p.b.zw)) * inWidth(xy.x);
}

// A 2 × 2 box: a row-packed map's half; the output's four pixels from two texels of two rows.
@fragment fn downFs(i: Full) -> @location(0) vec4f {
  let j = i32(i.pos.x);
  let y = i32(i.pos.y);
  let size = vec2i(p.b.zw);
  let a0 = texel(srcA, vec2i(2 * j, 2 * y), size);
  let a1 = texel(srcA, vec2i(2 * j + 1, 2 * y), size);
  let b0 = texel(srcA, vec2i(2 * j, 2 * y + 1), size);
  let b1 = texel(srcA, vec2i(2 * j + 1, 2 * y + 1), size);
  let v = vec4f(a0.x + a0.y + b0.x + b0.y, a0.z + a0.w + b0.z + b0.w, a1.x + a1.y + b1.x + b1.y, a1.z + a1.w + b1.z + b1.w);
  return v * 0.25 * inWidth(j);
}

// ── A blurred picture (Bulanıklaştır): its four channels, a pixel a texel ──
// downWideFs halves it by 2 × 2 boxes; blurWideFs blurs it along the direction b.xy (σ a.y, its weights' sum a.z).
// b.zw the source's size.

@fragment fn downWideFs(i: Full) -> @location(0) vec4f {
  let xy = vec2i(i.pos.xy) * 2;
  let size = vec2i(p.b.zw);
  return (texel(srcA, xy, size) + texel(srcA, xy + vec2i(1, 0), size) + texel(srcA, xy + vec2i(0, 1), size) + texel(srcA, xy + vec2i(1, 1), size)) * 0.25;
}

@fragment fn blurWideFs(i: Full) -> @location(0) vec4f {
  let xy = vec2i(i.pos.xy);
  let d = vec2i(p.b.xy);
  let r = i32(ceil(3.0 * p.a.y));
  let s2 = 2.0 * max(p.a.y, 1e-3) * max(p.a.y, 1e-3);
  var w = exp(-f32(r * r) / s2);
  var ratio = exp(f32(2 * r - 1) / s2);
  let q2 = exp(-2.0 / s2);
  var acc = vec4f(0.0);
  let size = vec2i(p.b.zw);
  for (var k = -r; k <= r; k++) {
    acc += w * texel(srcA, xy + d * k, size);
    w = w * ratio;
    ratio = ratio * q2;
  }
  return acc / p.a.z;
}

// ── An effect group onto its target (docs/adr/0226 §2) ──
// Every effect at once, in the order they are laid: the shadow S = c·o·M(x − offset), the outer glow G = c·o·M(x)
// over it, the group's picture K (srcA; blurred: srcF) over those, the inner shadow I = c·o·(1 − M(x − offset))·A and
// the inner glow J = c·o·(1 − M(x))·A over that; the whole laid over the target by the pipeline's premultiplied
// blending (source-over is associative: the same as laying each in turn). M is the effect's packed map (srcB the
// shadow's, srcC the glow's, srcD the inner shadow's, srcE the inner glow's) at its level, read bilinearly between
// texel centres (the last texel repeated), 0 beyond the picture.
// a = (the shadow's offset x, y, the inner shadow's x, y; device pixels, y down); b = the four maps' levels (0 none,
// 1, 2, 4); c, d, e, f = the shadow's, the glow's, the inner shadow's and the inner glow's colours, their alpha the
// opacity; g = (the blurred picture's level (0: the picture as it is), 0, its scale x, y: the target's uv to its);
// h.xy = the target's size in pixels.

// A packed map's pixel (x, y), x and y inside it.
fn packedAt(t: texture_2d<f32>, x: i32, y: i32) -> f32 {
  return textureLoad(t, vec2i(x >> 2, y), 0)[x & 3];
}

// A map of `level` at the target's point `pos` (pixels from the top left); `size` the target's.
fn mapAt(t: texture_2d<f32>, level: f32, pos: vec2f, size: vec2f) -> f32 {
  if (any(pos < vec2f(0.0)) || any(pos > size)) { return 0.0; }
  let last = ceil(size / level) - vec2f(1.0);
  let at = pos / level - vec2f(0.5);
  let base = floor(at);
  let f = at - base;
  let lo = clamp(base, vec2f(0.0), last);
  let hi = clamp(base + vec2f(1.0), vec2f(0.0), last);
  let v00 = packedAt(t, i32(lo.x), i32(lo.y));
  if (all(f == vec2f(0.0))) { return v00; }
  let v10 = packedAt(t, i32(hi.x), i32(lo.y));
  let v01 = packedAt(t, i32(lo.x), i32(hi.y));
  let v11 = packedAt(t, i32(hi.x), i32(hi.y));
  return mix(mix(v00, v10, f.x), mix(v01, v11, f.x), f.y);
}

// An effect's colour times its opacity over a map's value: premultiplied.
fn tint(color: vec4f, m: f32) -> vec4f {
  let alpha = clamp(color.a * m, 0.0, 1.0);
  return vec4f(color.rgb * alpha, alpha);
}

fn laid(top: vec4f, under: vec4f) -> vec4f {
  return top + under * (1.0 - top.a);
}

@fragment fn effectFs(i: Full) -> @location(0) vec4f {
  let xy = vec2i(i.pos.xy);
  // The group's picture is the target's size: the fragment's own texel is inside it.
  let group = textureLoad(srcA, xy, 0);
  let size = p.h.xy;
  var out = vec4f(0.0);
  if (p.b.x > 0.5) { out = tint(p.c, mapAt(srcB, p.b.x, i.pos.xy - p.a.xy, size)); }
  if (p.b.y > 0.5) { out = laid(tint(p.d, mapAt(srcC, p.b.y, i.pos.xy, size)), out); }
  var k = group;
  if (p.g.x > 0.5) { k = textureSampleLevel(srcF, smp, i.uv * p.g.zw, 0.0); }
  out = laid(k, out);
  if (p.b.z > 0.5) { out = laid(tint(p.e, (1.0 - mapAt(srcD, p.b.z, i.pos.xy - p.a.zw, size)) * group.a), out); }
  if (p.b.w > 0.5) { out = laid(tint(p.f, (1.0 - mapAt(srcE, p.b.w, i.pos.xy, size)) * group.a), out); }
  return out;
}
