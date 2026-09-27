// Styled drawing: thick, capped, dashed lines as instanced segments.
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

