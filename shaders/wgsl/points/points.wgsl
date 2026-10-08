// KentOS point clouds (docs/adr/0207 §6): a cloud's points drawn into its own picture with their depth, the highest
// nearest, before the styled pass shows the picture over the cloud's plan (styled/fill.wgsl's cloudFs). One draw per
// octree node, its points instances of a quad: `d` is the node's centre from the camera (float64 on the host, float32
// here, small for a node in view), each instance its point's offset from that centre and its colour.
struct Draw {
  d: vec4f,      // xy: the node's centre from the camera's centre (m); z: its height above the cloud's least (m)
  view: vec4f,   // xy: clip units per metre; zw: a point's half size in clip units
  depth: vec4f,  // x: 1 / the cloud's height range (1 when it has none); y: 1 a round point, 0 a square one
};
@group(0) @binding(0) var<uniform> draw: Draw;

struct PointOut {
  @builtin(position) pos: vec4f,
  @location(0) color: vec4f,
  @location(1) corner: vec2f,
};

@vertex fn pointsVs(@builtin(vertex_index) v: u32, @location(0) p: vec3f, @location(1) c: vec4f) -> PointOut {
  var corners = array<vec2f, 6>(
    vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
    vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0),
  );
  let k = corners[v];
  var o: PointOut;
  let xy = (draw.d.xy + p.xy) * draw.view.xy + k * draw.view.zw;
  let h = draw.d.z + p.z;
  o.pos = vec4f(xy, clamp(1.0 - h * draw.depth.x, 0.0, 1.0), 1.0);
  o.color = c;
  o.corner = k;
  return o;
}

// A hidden class's point has no alpha: it is not drawn.
@fragment fn pointsFs(i: PointOut) -> @location(0) vec4f {
  if (i.color.a < 0.5) { discard; }
  if (draw.depth.y > 0.5 && dot(i.corner, i.corner) > 1.0) { discard; }
  return vec4f(i.color.rgb, 1.0);
}
