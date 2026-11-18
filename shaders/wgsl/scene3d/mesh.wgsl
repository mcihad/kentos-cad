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
// see-through pass. A section's planes (docs/adr/8073 §3) come relative to the eye: a face's samples a plane cuts away
// are left out (each plane's distance carried from the pixel's middle to the sample by the derivatives, so a cut body
// ends where its cap's samples begin, docs/adr/8131), its fragment discarded when none is kept; a part whose style's
// alpha comes raised by 2 is never cut (a section's caps: cut where they are made, lying a hair in front of what the
// planes cut, drawn as much nearer as an edge is).
//
// The bodies' edges (docs/adr/8120) are lines over the faces, where two faces of a body meet at an angle: drawn a
// hair nearer than the faces they lie on (perspective: a fraction of the distance; orthographic: of the depth left),
// in their part's colour darkened, none for a see-through part's (a room's, an opening's). At the highest quality the
// faces also write their veil to a second target: a see-through face its alpha (the most of those over a pixel), an
// opaque one none; the composite lifts the ambient light's occlusion by it, so what is seen through a window's glass
// is not shaded as if the glass were not there.
// Version 3: the edges' pipeline and the View's `edge` block (208 bytes). Version 4 (docs/adr/8131): a style's alpha
// raised by 2 for a part no plane cuts, the styles read by the vertex stage too, and `edge.w` the samples a pixel.
const LAYOUT_VERSION: u32 = 4u;

struct View {
  clip: mat4x4f,              // the projection times the camera's rotation, column-major
  light: vec4f,               // xyz: towards the light, unit, in the shown space; w: the ambient share
  look: vec4f,                // x: the vertical exaggeration; y: the section's planes in use; z: the sky's share; w: the fill's
  edge: vec4f,                // x: 1 perspective, 0 orthographic; y: how much nearer an edge is drawn; z: how dark; w: samples a pixel
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
  // A section's cap (a part no plane cuts, docs/adr/8131) drawn as much nearer as an edge is: an edge just behind it
  // stays behind it.
  if (!cuts(part)) { o.pos = nearer(o.pos); }
  o.rel = rel;
  o.part = part;
  o.normal = vec3f(n.x, n.y, n.z / view.look.x);
  return o;
}

// A part's colour and alpha as its style gives them, the raise of a part no plane cuts taken off.
fn colour(part: u32) -> vec4f {
  let c = styles[part];
  return vec4f(c.rgb, select(c.a, c.a - 2.0, c.a >= 2.0));
}

// Whether a section's planes cut the part (a section's caps are not cut, docs/adr/8131).
fn cuts(part: u32) -> bool {
  return styles[part].a < 2.0;
}

// `pos` drawn a hair nearer, as an edge is. Reversed depth: perspective's z/w is near/distance, so a little more z is a
// fraction of the distance nearer; orthographic's is 1 − depth/range, so the depth left (1 − z) is shortened by that
// fraction.
fn nearer(pos: vec4f) -> vec4f {
  let k = view.edge.y;
  var out = pos;
  out.z = select(pos.z + k * (pos.w - pos.z), pos.z * (1.0 + k), view.edge.x > 0.5);
  return out;
}

// Whether a section keeps the point `rel` (relative to the eye).
fn kept(rel: vec3f) -> bool {
  for (var k = 0u; k < u32(view.look.y); k = k + 1u) {
    let q = view.planes[k];
    if (dot(q.xyz, rel) + q.w < 0.0) { return false; }
  }
  return true;
}

// The samples of the pixel a section keeps of a face at `rel` (its derivatives `rx`, `ry`): one sample, its middle;
// four, the standard pattern's places (from the pixel's top left corner, y down: (3/8, 1/8), (7/8, 3/8), (1/8, 5/8),
// (5/8, 7/8)). A pixel's middle kept or cut alone would leave a cut face's samples up to half a pixel past its plane,
// in front of its cap's, or short of it.
fn keptSamples(rel: vec3f, rx: vec3f, ry: vec3f) -> u32 {
  if (view.edge.w < 2.0) { return select(0u, 1u, kept(rel)); }
  var mask = 0u;
  if (kept(rel - 0.125 * rx - 0.375 * ry)) { mask = mask | 1u; }
  if (kept(rel + 0.375 * rx - 0.125 * ry)) { mask = mask | 2u; }
  if (kept(rel - 0.375 * rx + 0.125 * ry)) { mask = mask | 4u; }
  if (kept(rel + 0.125 * rx + 0.375 * ry)) { mask = mask | 8u; }
  return mask;
}

struct Shaded {
  @location(0) colour: vec4f,
  @builtin(sample_mask) mask: u32,
};

fn shade(i: MeshOut) -> Shaded {
  let rx = dpdx(i.rel);
  let ry = dpdy(i.rel);
  var n = normalize(cross(rx, ry));
  if (dot(i.normal, i.normal) > 1e-4) { n = normalize(i.normal); }
  if (dot(n, i.rel) > 0.0) { n = -n; }
  let a = view.light.w;
  var lit = a + (1.0 - a) * max(dot(n, view.light.xyz), 0.0);
  // The fill: from the light's other side, a little from above.
  let fill = normalize(vec3f(-view.light.x, -view.light.y, 0.6));
  lit = lit + view.look.w * (1.0 - a) * max(dot(n, fill), 0.0);
  // The sky: up-facing brighter, down-facing darker, as far as `look.z` asks.
  lit = lit * (1.0 + view.look.z * n.z);
  let c = colour(i.part);
  var o: Shaded;
  o.mask = 0xffffffffu;
  if (cuts(i.part) && view.look.y >= 1.0) {
    o.mask = keptSamples(i.rel, rx, ry);
    if (o.mask == 0u) { discard; }
  }
  o.colour = vec4f(c.rgb * lit * c.a, c.a);
  return o;
}

@fragment fn meshFs(i: MeshOut) -> Shaded {
  return shade(i);
}

struct Veiled {
  @location(0) colour: vec4f,
  @location(1) veil: vec4f,
  @builtin(sample_mask) mask: u32,
};

@fragment fn meshVeiledFs(i: MeshOut) -> Veiled {
  let s = shade(i);
  var o: Veiled;
  o.colour = s.colour;
  o.mask = s.mask;
  let a = colour(i.part).a;
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
  o.pos = nearer(view.clip * vec4f(rel, 1.0));
  o.rel = rel;
  o.part = part;
  return o;
}

@fragment fn edgeFs(i: EdgeOut) -> @location(0) vec4f {
  let c = colour(i.part);
  // A see-through part's volume (a room, an opening) has no edges drawn; a window's glass has.
  if (c.a < 0.5 || (cuts(i.part) && !kept(i.rel))) { discard; }
  return vec4f(c.rgb * (1.0 - view.edge.z), 1.0);
}
