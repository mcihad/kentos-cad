/**
 * GLSL twin of shaders/wgsl/compose/compose.wgsl (docs/adr/0226 §2–§4) for the WebGL2 backend: the same passes and
 * numbers (`u_a` … `u_h` are the WGSL `Params`' `a` … `h`, `u_srcA` … `u_srcF` its pictures), in GL's own orientation:
 * rows from the bottom, so a packed map's rows and a half map's pairs count from the bottom, and `v_uv` rises upwards
 * (the composer turns a shadow's vertical offset round). Colours are premultiplied; outside a picture is transparent.
 */

/** A triangle over the whole target; uv from the bottom left, as GL's textures have it. */
export const COMPOSE_FULL_VS = `#version 300 es
out vec2 v_uv;
void main() {
  vec2 q = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  v_uv = q;
  gl_Position = vec4(q * 2.0 - 1.0, 0.0, 1.0);
}`;

const HEAD = `#version 300 es
precision highp float;
precision highp int;
uniform vec4 u_a;
uniform vec4 u_b;
uniform vec4 u_c;
uniform vec4 u_d;
uniform vec4 u_e;
uniform vec4 u_f;
uniform vec4 u_g;
uniform vec4 u_h;
uniform sampler2D u_srcA;
uniform sampler2D u_srcB;
uniform sampler2D u_srcC;
uniform sampler2D u_srcD;
uniform sampler2D u_srcE;
uniform sampler2D u_srcF;
in vec2 v_uv;
out vec4 o;
// A texel of t of size texels, 0 beyond its edge (the size the pass's own number).
vec4 texel(sampler2D t, ivec2 xy, ivec2 size) {
  if (any(lessThan(xy, ivec2(0))) || any(greaterThanEqual(xy, size))) return vec4(0.0);
  return texelFetch(t, xy, 0);
}
ivec2 here() { return ivec2(gl_FragCoord.xy); }
`;

/** The layer's factor: its opacity, cut by the mask (red). */
const LAYER = `
float layerFactor(ivec2 xy) {
  float k = u_a.x;
  if (u_a.y > 0.5) k *= 1.0 - texelFetch(u_srcC, xy, 0).r;
  return k;
}
`;

export const COMPOSE_OVER_FS = `${HEAD}${LAYER}
void main() { ivec2 xy = here(); o = texelFetch(u_srcA, xy, 0) * layerFactor(xy); }`;

export const COMPOSE_BLEND_FS = `${HEAD}${LAYER}
float hardLight(float cb, float cs) {
  if (cs <= 0.5) return cb * 2.0 * cs;
  float s = 2.0 * cs - 1.0;
  return cb + s - cb * s;
}
float softD(float cb) { return cb <= 0.25 ? ((16.0 * cb - 12.0) * cb + 4.0) * cb : sqrt(cb); }
float blendOne(int mode, float cb, float cs) {
  if (mode == 1) return cb * cs;
  if (mode == 2) return cb + cs - cb * cs;
  if (mode == 3) return hardLight(cs, cb);
  if (mode == 4) return min(cb, cs);
  if (mode == 5) return max(cb, cs);
  if (mode == 6) {
    if (cb <= 0.0) return 0.0;
    if (cs >= 1.0) return 1.0;
    return min(1.0, cb / (1.0 - cs));
  }
  if (mode == 7) {
    if (cb >= 1.0) return 1.0;
    if (cs <= 0.0) return 0.0;
    return 1.0 - min(1.0, (1.0 - cb) / cs);
  }
  if (mode == 8) return hardLight(cb, cs);
  if (mode == 9) {
    if (cs <= 0.5) return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb);
    return cb + (2.0 * cs - 1.0) * (softD(cb) - cb);
  }
  if (mode == 10) return abs(cb - cs);
  if (mode == 11) return cb + cs - 2.0 * cb * cs;
  return cs;
}
void main() {
  ivec2 xy = here();
  vec4 s = texelFetch(u_srcA, xy, 0);
  vec3 cb = texelFetch(u_srcB, xy, 0).rgb;
  float k = layerFactor(xy);
  float alpha = s.a * k;
  if (alpha <= 0.0) { o = vec4(cb, 1.0); return; }
  int mode = int(u_a.z + 0.5);
  if (mode == 12) { o = vec4(min(vec3(1.0), cb + s.rgb * k), 1.0); return; }
  vec3 cs = clamp(s.rgb / s.a, vec3(0.0), vec3(1.0));
  vec3 b = vec3(blendOne(mode, cb.r, cs.r), blendOne(mode, cb.g, cs.g), blendOne(mode, cb.b, cs.b));
  o = vec4((1.0 - alpha) * cb + alpha * b, 1.0);
}`;

/**
 * A mask box per instance (see the WGSL): middle and half size in the target's pixels from its top left, the cosine
 * and sine of its turn as the screen turns, growth.
 */
export const COMPOSE_BOX_VS = `#version 300 es
uniform vec4 u_a;
in vec2 a_centre;
in vec2 a_half;
in vec2 a_dir;
in float a_grow;
flat out vec2 v_centre;
flat out vec2 v_dir;
flat out vec3 v_hr;
const vec2 CORNERS[6] = vec2[6](vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0));
void main() {
  vec2 reach = a_half + vec2(a_grow + 1.0);
  vec2 local = CORNERS[gl_VertexID] * reach;
  vec2 px = a_centre + vec2(a_dir.x * local.x - a_dir.y * local.y, a_dir.y * local.x + a_dir.x * local.y);
  gl_Position = vec4(px.x / u_a.x * 2.0 - 1.0, 1.0 - px.y / u_a.y * 2.0, 0.0, 1.0);
  v_centre = a_centre;
  v_dir = a_dir;
  v_hr = vec3(a_half, a_grow);
}`;

/** The box's coverage from the pixel's own position (GL's rows rise: the target's height turns them down). */
export const COMPOSE_BOX_FS = `#version 300 es
precision highp float;
uniform vec4 u_a;
flat in vec2 v_centre;
flat in vec2 v_dir;
flat in vec3 v_hr;
out vec4 o;
void main() {
  vec2 v = vec2(gl_FragCoord.x, u_a.y - gl_FragCoord.y) - v_centre;
  vec2 local = vec2(v_dir.x * v.x + v_dir.y * v.y, -v_dir.y * v.x + v_dir.x * v.y);
  vec2 q = abs(local) - v_hr.xy;
  float d = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - v_hr.z;
  o = vec4(clamp(0.5 - d, 0.0, 1.0));
}`;

/** The packed maps' common part (see the WGSL): b.xy the map's size in pixels, b.zw the source's in texels. */
const PACKED = `
vec4 inWidth(int j) {
  vec4 x = vec4(float(4 * j), float(4 * j + 1), float(4 * j + 2), float(4 * j + 3));
  return step(x + vec4(0.5), vec4(u_b.x));
}
vec4 inHeight(int j) {
  vec4 y = vec4(float(4 * j), float(4 * j + 1), float(4 * j + 2), float(4 * j + 3));
  return step(y + vec4(0.5), vec4(u_b.y));
}
// The pass's work along d at the fragment's texel xy of a source of size texels (see the WGSL's filtered).
vec4 filtered(ivec2 xy, ivec2 d, ivec2 size) {
  int op = int(u_a.x + 0.5);
  vec4 centre = texelFetch(u_srcA, xy, 0);
  if (op == 2) {
    int r = int(ceil(3.0 * u_a.y));
    float q = exp(-1.0 / (2.0 * u_a.y * u_a.y));
    float q2 = q * q;
    vec4 acc = centre;
    float w = 1.0;
    float ratio = q;
    ivec2 lo = xy - d * (r + 1);
    ivec2 hi = xy + d * (r + 1);
    if (all(greaterThanEqual(lo, ivec2(0))) && all(lessThan(hi, size))) {
      vec2 texels = vec2(size);
      vec2 at = vec2(xy) + vec2(0.5);
      for (int k = 1; k <= r; k += 2) {
        float w1 = w * ratio;
        ratio *= q2;
        float w2 = w1 * ratio;
        ratio *= q2;
        w = w2;
        if (k + 1 > r) w2 = 0.0;
        float pair = w1 + w2;
        vec2 off = vec2(d) * (float(k) + w2 / pair);
        acc += pair * (texture(u_srcA, (at + off) / texels) + texture(u_srcA, (at - off) / texels));
      }
    } else {
      for (int k = 1; k <= r; k++) {
        w *= ratio;
        ratio *= q2;
        acc += w * (texel(u_srcA, xy + d * k, size) + texel(u_srcA, xy - d * k, size));
      }
    }
    return acc / u_a.z;
  }
  int n = int(u_a.y + 0.5);
  vec4 most = centre;
  vec4 least = centre;
  for (int k = 1; k <= n; k++) {
    vec4 a = texel(u_srcA, xy + d * k, size);
    vec4 b = texel(u_srcA, xy - d * k, size);
    most = max(most, max(a, b));
    least = min(least, min(a, b));
  }
  return op == 3 ? most : least;
}
`;

/** The group's alpha, column-packed. */
export const COMPOSE_PACK_FS = `${HEAD}${PACKED}
void main() {
  ivec2 xy = here();
  ivec2 size = ivec2(u_b.zw);
  vec4 v = vec4(texel(u_srcA, ivec2(xy.x, 4 * xy.y), size).a, texel(u_srcA, ivec2(xy.x, 4 * xy.y + 1), size).a, texel(u_srcA, ivec2(xy.x, 4 * xy.y + 2), size).a, texel(u_srcA, ivec2(xy.x, 4 * xy.y + 3), size).a);
  o = v * inHeight(xy.y);
}`;

/** A column-packed map row-packed. */
export const COMPOSE_ROWS_FS = `${HEAD}${PACKED}
void main() {
  ivec2 xy = here();
  ivec2 size = ivec2(u_b.zw);
  int c = xy.y & 3;
  int jy = xy.y >> 2;
  o = vec4(texel(u_srcA, ivec2(4 * xy.x, jy), size)[c], texel(u_srcA, ivec2(4 * xy.x + 1, jy), size)[c], texel(u_srcA, ivec2(4 * xy.x + 2, jy), size)[c], texel(u_srcA, ivec2(4 * xy.x + 3, jy), size)[c]) * inWidth(xy.x);
}`;

/** A row-packed map column-packed. */
export const COMPOSE_COLUMNS_FS = `${HEAD}${PACKED}
void main() {
  ivec2 xy = here();
  ivec2 size = ivec2(u_b.zw);
  int c = xy.x & 3;
  int jx = xy.x >> 2;
  o = vec4(texel(u_srcA, ivec2(jx, 4 * xy.y), size)[c], texel(u_srcA, ivec2(jx, 4 * xy.y + 1), size)[c], texel(u_srcA, ivec2(jx, 4 * xy.y + 2), size)[c], texel(u_srcA, ivec2(jx, 4 * xy.y + 3), size)[c]) * inHeight(xy.y);
}`;

/** Along x, a column-packed map. */
export const COMPOSE_H_FS = `${HEAD}${PACKED}
void main() { ivec2 xy = here(); o = filtered(xy, ivec2(1, 0), ivec2(u_b.zw)) * inHeight(xy.y); }`;

/** Along y, a row-packed map. */
export const COMPOSE_V_FS = `${HEAD}${PACKED}
void main() { ivec2 xy = here(); o = filtered(xy, ivec2(0, 1), ivec2(u_b.zw)) * inWidth(xy.x); }`;

/** A row-packed map's half. */
export const COMPOSE_DOWN_FS = `${HEAD}${PACKED}
void main() {
  ivec2 xy = here();
  ivec2 size = ivec2(u_b.zw);
  vec4 a0 = texel(u_srcA, ivec2(2 * xy.x, 2 * xy.y), size);
  vec4 a1 = texel(u_srcA, ivec2(2 * xy.x + 1, 2 * xy.y), size);
  vec4 b0 = texel(u_srcA, ivec2(2 * xy.x, 2 * xy.y + 1), size);
  vec4 b1 = texel(u_srcA, ivec2(2 * xy.x + 1, 2 * xy.y + 1), size);
  vec4 v = vec4(a0.x + a0.y + b0.x + b0.y, a0.z + a0.w + b0.z + b0.w, a1.x + a1.y + b1.x + b1.y, a1.z + a1.w + b1.z + b1.w);
  o = v * 0.25 * inWidth(xy.x);
}`;

/** A blurred picture's half, its four channels. */
export const COMPOSE_DOWN_WIDE_FS = `${HEAD}
void main() {
  ivec2 xy = here() * 2;
  ivec2 size = ivec2(u_b.zw);
  o = (texel(u_srcA, xy, size) + texel(u_srcA, xy + ivec2(1, 0), size) + texel(u_srcA, xy + ivec2(0, 1), size) + texel(u_srcA, xy + ivec2(1, 1), size)) * 0.25;
}`;

/** A blurred picture blurred along b.xy (σ a.y, its weights' sum a.z). */
export const COMPOSE_BLUR_WIDE_FS = `${HEAD}
void main() {
  ivec2 xy = here();
  ivec2 d = ivec2(u_b.xy);
  ivec2 size = ivec2(u_b.zw);
  int r = int(ceil(3.0 * u_a.y));
  float s2 = 2.0 * max(u_a.y, 1e-3) * max(u_a.y, 1e-3);
  float w = exp(-float(r * r) / s2);
  float ratio = exp(float(2 * r - 1) / s2);
  float q2 = exp(-2.0 / s2);
  vec4 acc = vec4(0.0);
  for (int k = -r; k <= r; k++) {
    acc += w * texel(u_srcA, xy + d * k, size);
    w *= ratio;
    ratio *= q2;
  }
  o = acc / u_a.z;
}`;

/** An effect group onto its target, every effect at once (see the WGSL's effectFs). */
export const COMPOSE_EFFECT_FS = `${HEAD}
float packedAt(sampler2D t, int x, int y) { return texelFetch(t, ivec2(x >> 2, y), 0)[x & 3]; }
float mapAt(sampler2D t, float level, vec2 pos, vec2 size) {
  if (any(lessThan(pos, vec2(0.0))) || any(greaterThan(pos, size))) return 0.0;
  vec2 last = ceil(size / level) - vec2(1.0);
  vec2 at = pos / level - vec2(0.5);
  vec2 base = floor(at);
  vec2 f = at - base;
  vec2 lo = clamp(base, vec2(0.0), last);
  vec2 hi = clamp(base + vec2(1.0), vec2(0.0), last);
  float v00 = packedAt(t, int(lo.x), int(lo.y));
  if (all(equal(f, vec2(0.0)))) return v00;
  float v10 = packedAt(t, int(hi.x), int(lo.y));
  float v01 = packedAt(t, int(lo.x), int(hi.y));
  float v11 = packedAt(t, int(hi.x), int(hi.y));
  return mix(mix(v00, v10, f.x), mix(v01, v11, f.x), f.y);
}
vec4 tint(vec4 color, float m) {
  float alpha = clamp(color.a * m, 0.0, 1.0);
  return vec4(color.rgb * alpha, alpha);
}
vec4 laid(vec4 top, vec4 under) { return top + under * (1.0 - top.a); }
void main() {
  ivec2 xy = here();
  vec4 group = texelFetch(u_srcA, xy, 0);
  vec2 size = u_h.xy;
  vec2 pos = gl_FragCoord.xy;
  vec4 outv = vec4(0.0);
  if (u_b.x > 0.5) outv = tint(u_c, mapAt(u_srcB, u_b.x, pos - u_a.xy, size));
  if (u_b.y > 0.5) outv = laid(tint(u_d, mapAt(u_srcC, u_b.y, pos, size)), outv);
  vec4 k = group;
  if (u_g.x > 0.5) k = texture(u_srcF, v_uv * u_g.zw);
  outv = laid(k, outv);
  if (u_b.z > 0.5) outv = laid(tint(u_e, (1.0 - mapAt(u_srcD, u_b.z, pos - u_a.zw, size)) * group.a), outv);
  if (u_b.w > 0.5) outv = laid(tint(u_f, (1.0 - mapAt(u_srcE, u_b.w, pos, size)) * group.a), outv);
  o = outv;
}`;
