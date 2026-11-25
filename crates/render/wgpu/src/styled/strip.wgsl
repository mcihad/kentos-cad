// Native only (docs/adr/9247): a stroke's quad as a strip of four corners instead of a list of six. The list's corners
// 0, 1, 5, 2, in a strip, make the same quad of two triangles, so every pixel it covers and every number it interpolates
// is the same; the vertex stage runs four times a segment instead of six.
@vertex fn strokeVsStrip(@builtin(vertex_index) vi: u32, @location(0) seg: vec4f, @location(1) mt: vec2f, @location(2) low: vec4f) -> StrokeOut {
  var list = array<u32, 4>(0u, 1u, 5u, 2u);
  return strokeVertex(list[vi], seg, mt, low);
}
