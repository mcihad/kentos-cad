// Native extension only (docs/adr/1063 §2): a block definition's strokes and areas, drawn once per insert placed by
// the instance's similarity. The definition's numbers come per vertex (a stroke's segment repeated for its six
// corners), the instance's per instance: its linear part by columns (m), its translation from the batch's tile in a
// high and a low part (t.xy, t.zw) and its uniform scale (k.x). The shared v10 source and its bindings are unchanged.
fn placedBy(m: vec4f, p: vec2f) -> vec2f {
  return vec2f(m.x * p.x + m.z * p.y, m.y * p.x + m.w * p.y);
}
@vertex fn strokeVsInstanced(@builtin(vertex_index) vi: u32, @location(0) seg: vec4f, @location(1) mt: vec2f,
    @location(2) m: vec4f, @location(3) t: vec4f, @location(4) k: vec2f) -> StrokeOut {
  // The translation's high part is the position, the small placed offset rides with its low part (toPxLow takes the
  // camera's parts off each before they meet), so no digit of a far insert is lost.
  let a = placedBy(m, seg.xy);
  let b = placedBy(m, seg.zw);
  return strokeVertex(vi % 6u, vec4f(t.xy, t.xy), vec2f(mt.x * k.x, mt.y), vec4f(t.zw + a, t.zw + b));
}
@vertex fn areaVsInstanced(@location(0) p: vec2f, @location(2) m: vec4f, @location(3) t: vec4f, @location(4) k: vec2f) -> AreaOut {
  let d = placedBy(m, p);
  var o: AreaOut;
  o.pos = pxToClip(toPxLow(t.xy, t.zw + d));
  o.world = t.xy + (t.zw + d);
  return o;
}
