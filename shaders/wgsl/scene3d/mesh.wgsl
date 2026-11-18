// KentOS's 3D scene (docs/adr/0910 §4–§5, plan/3d.md D1): triangle meshes in tiles, each tile's positions float32
// offsets from its own origin. The host computes, in float64, the tile's origin as shown minus the camera's eye and
// hands it here as `tile.offset`: a position relative to the eye is the vertex's offset (its height exaggerated) plus
// that, small numbers only; `view.clip` is the projection times the rotation, no translation. Depth is reversed (near
// 1, far 0; perspective's far plane at infinity), compared greater. A smooth mesh's vertices carry their normals
// (signed 8-bit; the exaggeration turns them by its inverse transpose, diag(1, 1, 1/k)); a flat mesh's are zero and
// its faces' normals come from the position's derivatives (glTF asks a reader to make them so for a mesh without
// normals). Both faces are lit: the light and its ambient share, and when the host asks (docs/adr/8120) a sky that
// brightens what faces up and darkens what faces down, and a fill from the light's other side so that two faces seen
// side by side never read as one. A part's colour is its style; a style with alpha below 1 is drawn in the
// see-through pass. A section's planes (docs/adr/8073 §3) come relative to the eye: a fragment a plane cuts away is
// discarded, after the derivatives are taken; the cut faces are not capped.
//
// The bodies' edges (docs/adr/8120) are lines over the faces, where two faces of a body meet at an angle: drawn a
// hair nearer than the faces they lie on (perspective: a fraction of the distance; orthographic: of the depth left),
// in their part's colour darkened, none for a see-through part's (a room's, an opening's). At the highest quality the
// faces also write their veil to a second target: a see-through face its alpha (the most of those over a pixel), an
// opaque one none; the composite lifts the ambient light's occlusion by it, so what is seen through a window's glass
// is not shaded as if the glass were not there.
// Version 3: the edges' pipeline and the View's `edge` block (208 bytes).
const LAYOUT_VERSION: u32 = 3u;

struct View {
  clip: mat4x4f,              // the projection times the camera's rotation, column-major
  light: vec4f,               // xyz: towards the light, unit, in the shown space; w: the ambient share
  look: vec4f,                // x: the vertical exaggeration; y: the section's planes in use; z: the sky's share; w: the fill's
  edge: vec4f,                // x: 1 perspective, 0 orthographic; y: how much nearer an edge is drawn; z: how dark; w: unused
  planes: array<vec4f, 6>,    // xyz: a plane's unit normal (the kept side), w: its constant about the eye
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

// Whether a section keeps the point `rel` (relative to the eye).
fn kept(rel: vec3f) -> bool {
  for (var k = 0u; k < u32(view.look.y); k = k + 1u) {
    let q = view.planes[k];
    if (dot(q.xyz, rel) + q.w < 0.0) { return false; }
  }
  return true;
}

fn shade(i: MeshOut) -> vec4f {
  var n = normalize(cross(dpdx(i.rel), dpdy(i.rel)));
  if (dot(i.normal, i.normal) > 1e-4) { n = normalize(i.normal); }
  if (dot(n, i.rel) > 0.0) { n = -n; }
  let a = view.light.w;
  var lit = a + (1.0 - a) * max(dot(n, view.light.xyz), 0.0);
  // The fill: from the light's other side, a little from above.
  let fill = normalize(vec3f(-view.light.x, -view.light.y, 0.6));
  lit = lit + view.look.w * (1.0 - a) * max(dot(n, fill), 0.0);
  // The sky: up-facing brighter, down-facing darker, as far as `look.z` asks.
  lit = lit * (1.0 + view.look.z * n.z);
  let c = styles[i.part];
  if (!kept(i.rel)) { discard; }
  return vec4f(c.rgb * lit * c.a, c.a);
}

@fragment fn meshFs(i: MeshOut) -> @location(0) vec4f {
  return shade(i);
}

struct Veiled {
  @location(0) colour: vec4f,
  @location(1) veil: vec4f,
};

@fragment fn meshVeiledFs(i: MeshOut) -> Veiled {
  var o: Veiled;
  o.colour = shade(i);
  let a = styles[i.part].a;
  o.veil = vec4f(select(a, 0.0, a >= 1.0));
  return o;
}

struct EdgeOut {
  @builtin(position) pos: vec4f,
  @location(0) rel: vec3f,
  @location(1) @interpolate(flat) part: u32,
};

@vertex fn edgeVs(@location(0) p: vec3f, @location(1) part: u32, @location(2) n: vec4f) -> EdgeOut {
  var o: EdgeOut;
  let rel = vec3f(p.x, p.y, p.z * view.look.x) + tile.offset.xyz;
  var pos = view.clip * vec4f(rel, 1.0);
  // Reversed depth: perspective's z/w is near/distance, so a little more z is a fraction of the distance nearer;
  // orthographic's is 1 − depth/range, so the depth left (1 − z) is shortened by that fraction.
  let k = view.edge.y;
  pos.z = select(pos.z + k * (pos.w - pos.z), pos.z * (1.0 + k), view.edge.x > 0.5);
  o.pos = pos;
  o.rel = rel;
  o.part = part;
  return o;
}

@fragment fn edgeFs(i: EdgeOut) -> @location(0) vec4f {
  let c = styles[i.part];
  // A see-through part's volume (a room, an opening) has no edges drawn; a window's glass has.
  if (c.a < 0.5 || !kept(i.rel)) { discard; }
  return vec4f(c.rgb * (1.0 - view.edge.z), 1.0);
}
