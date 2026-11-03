// KentOS's 3D scene (docs/adr/0910 §4–§5, plan/3d.md D1): triangle meshes in tiles, each tile's positions float32
// offsets from its own origin. The host computes, in float64, the tile's origin as shown minus the camera's eye and
// hands it here as `tile.offset`: a position relative to the eye is the vertex's offset (its height exaggerated) plus
// that, small numbers only; `view.clip` is the projection times the rotation, no translation. Depth is reversed (near
// 1, far 0; perspective's far plane at infinity), compared greater. A smooth mesh's vertices carry their normals
// (signed 8-bit; the exaggeration turns them by its inverse transpose, diag(1, 1, 1/k)); a flat mesh's are zero and
// its faces' normals come from the position's derivatives (glTF asks a reader to make them so for a mesh without
// normals). Both faces are lit. A part's colour is its style; a style with alpha below 1 is drawn in the see-through
// pass. Version 2: the normal (20-byte vertices).
const LAYOUT_VERSION: u32 = 2u;

struct View {
  clip: mat4x4f,  // the projection times the camera's rotation, column-major
  light: vec4f,   // xyz: towards the light, unit, in the shown space; w: the ambient share
  look: vec4f,    // x: the vertical exaggeration; yzw: unused
};
struct Tile {
  offset: vec4f,  // xyz: the tile's origin as shown minus the eye (m); w: unused
};
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> styles: array<vec4f>;
@group(1) @binding(0) var<uniform> tile: Tile;

struct MeshOut {
  @builtin(position) pos: vec4f,
  @location(0) rel: vec3f,
  @location(1) @interpolate(flat) part: u32,
  @location(2) normal: vec3f,
};

@vertex fn meshVs(@location(0) p: vec3f, @location(1) part: u32, @location(2) n: vec4f) -> MeshOut {
  var o: MeshOut;
  let rel = vec3f(p.x, p.y, p.z * view.look.x) + tile.offset.xyz;
  o.pos = view.clip * vec4f(rel, 1.0);
  o.rel = rel;
  o.part = part;
  o.normal = vec3f(n.x, n.y, n.z / view.look.x);
  return o;
}

@fragment fn meshFs(i: MeshOut) -> @location(0) vec4f {
  var n = normalize(cross(dpdx(i.rel), dpdy(i.rel)));
  if (dot(i.normal, i.normal) > 1e-4) { n = normalize(i.normal); }
  if (dot(n, i.rel) > 0.0) { n = -n; }
  let lit = view.light.w + (1.0 - view.light.w) * max(dot(n, view.light.xyz), 0.0);
  let c = styles[i.part];
  return vec4f(c.rgb * lit * c.a, c.a);
}
