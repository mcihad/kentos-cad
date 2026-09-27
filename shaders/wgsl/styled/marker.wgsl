// Styled drawing: instanced markers, a shape from its distance field or an atlas image.
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
