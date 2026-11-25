// Styled drawing: thick, capped, dashed lines as instanced segments.
// ── Strokes: a = (width, dashTotal, dashOn, dashOffset), flags = (unit, cap) ──
struct StrokeOut {
  @invariant @builtin(position) pos: vec4f,
  @location(0) local: vec2f,
  @location(1) len: f32,
  @location(2) s0: f32,
  @location(3) @interpolate(flat) ends: u32,
  @location(4) @interpolate(flat) halfW: f32,
  @location(5) @interpolate(flat) joins: vec4f,
};
// A join's turn from its code (docs/adr/9226): 1024 steps a half turn, 1024 straight on; 0 is no code.
fn turnOf(code: u32) -> f32 { return (f32(code) - 1024.0) * (3.14159265 / 1024.0); }
// mt.y: the ends (bits 0, 1: the path's start and end take the caps), then the start's and the end's join codes.
fn strokeVertex(vi: u32, seg: vec4f, mt: vec2f, low: vec4f) -> StrokeOut {
  let halfW = max(st.a.x * weightK(), 1.0) * 0.5;
  let blur = max(st.b.x * unitK(), 1.0);
  let A = toPxLow(seg.xy, low.xy);
  let B = toPxLow(seg.zw, low.zw);
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
  let packed = u32(mt.y + 0.5);
  o.ends = packed & 3u;
  o.halfW = halfW;
  // A join is split along its bisector (docs/adr/9226): each segment draws its own side of the joint, the union of
  // the two, so a seen-through line is not drawn twice at its corners. The sides' normals in the segment's own axes
  // (along, across); zero where the segment draws its whole end (no code, or a segment too short for its two
  // bisectors to keep apart within its band).
  let cs = (packed >> 2u) & 2047u;
  let ce = (packed >> 13u) & 2047u;
  let ts = turnOf(cs);
  let te = turnOf(ce);
  let reach = select(0.0, abs(tan(0.5 * ts)), cs != 0u) + select(0.0, abs(tan(0.5 * te)), ce != 0u);
  var joins = vec4f(0.0);
  if (len > ext * reach + 1.0) {
    if (cs != 0u) { joins = vec4f(normalize(vec2f(1.0 + cos(ts), -sin(ts))), joins.zw); }
    if (ce != 0u) { joins = vec4f(joins.xy, normalize(vec2f(1.0 + cos(te), sin(te)))); }
  }
  o.joins = joins;
  return o;
}
@vertex fn strokeVs(@builtin(vertex_index) vi: u32, @location(0) seg: vec4f, @location(1) mt: vec2f) -> StrokeOut {
  return strokeVertex(vi, seg, mt, vec4f(0.0));
}
@vertex fn strokeVsPrecise(@builtin(vertex_index) vi: u32, @location(0) seg: vec4f, @location(1) mt: vec2f, @location(2) low: vec4f) -> StrokeOut {
  return strokeVertex(vi, seg, mt, low);
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
  // The joins' sides (docs/adr/9226): the segment before draws behind the start's bisector, the one after beyond the
  // end's; a hundredth of a pixel on the line is drawn by both, so no pixel is left out.
  if (dot(vec2f(x, i.local.y), i.joins.xy) < -0.01) { discard; }
  if (dot(vec2f(x - i.len, i.local.y), i.joins.zw) > 0.01) { discard; }
  let y = abs(i.local.y);
  var startCap = 1u;
  if ((i.ends & 1u) != 0u) { startCap = st.flags.y; }
  var endCap = 1u;
  if ((i.ends & 2u) != 0u) { endCap = st.flags.y; }
  // A path's butt end is smoothed across it as the sides are (docs/adr/9249): the pixel's span against the line's, half
  // the ramp outside the end and half inside. A round or a square cap, or a joint, keeps the line whole to its end.
  var along = 1.0;
  if (startCap == 0u) { along = cover(x); }
  if (endCap == 0u) { along = along + cover(i.len - x) - 1.0; }
  var a: f32;
  if (x < 0.0 && startCap != 0u) {
    a = capCover(-x, y, startCap, i.halfW);
  } else if (x > i.len && endCap != 0u) {
    a = capCover(x - i.len, y, endCap, i.halfW);
  } else {
    a = cover(i.halfW - y) * max(along, 0.0);
  }
  a *= dashCover(i.s0 + x, unitK(), st.a.y, st.a.z, st.a.w);
  if (a < 0.004) { discard; }
  return vec4f(st.color.rgb, st.color.a * a);
}

