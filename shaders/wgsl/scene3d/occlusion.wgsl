// The ambient light's occlusion of a 3D view (docs/adr/8120; McGuire, Mara, Luebke, “Scalable Ambient Obscurance”,
// HPG 2012): from the depth the opaque faces left (reversed, perspective: near/distance), each pixel's position about
// the eye and its face's normal (from the neighbour nearer in depth on each axis, so an edge does not bend it); twelve
// points on a spiral within a radius in metres around it, turned by a 4 × 4 pattern the composite averages away; each
// point above the face and within the radius occludes, the nearer the more (a point a centimetre away no more than one
// ten centimetres away: the paper's ε of 0.01 m²). The sky is not occluded; a corner keeps at least the light's
// `LEAST` (the occlusion takes the ambient light, not the sun's). One value a pixel, 1 for nothing occluded.

struct Occlusion {
  lens: vec4f,  // x: 1/tan(half the vertical field of view); y: the aspect; z: the near plane (m); w: unused
  shape: vec4f, // x: the radius (m); y: the strength; z, w: the target's width and height (px)
};
@group(0) @binding(0) var depth: texture_depth_multisampled_2d;
@group(0) @binding(1) var<uniform> o: Occlusion;

const SAMPLES: i32 = 12;
const LEAST: f32 = 0.45;
// The largest radius on the screen (px): close up, a wider one only costs time.
const WIDEST: f32 = 72.0;

struct Out {
  @builtin(position) pos: vec4f,
};

@vertex fn occlusionVs(@builtin(vertex_index) i: u32) -> Out {
  let t = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  var out: Out;
  out.pos = vec4f(t.x * 2.0 - 1.0, 1.0 - t.y * 2.0, 0.0, 1.0);
  return out;
}

// The point a pixel shows about the eye (x right, y up, looking along −z); the sky far away.
fn position(p: vec2i) -> vec3f {
  let size = vec2i(i32(o.shape.z), i32(o.shape.w));
  let q = clamp(p, vec2i(0), size - 1);
  let d = textureLoad(depth, q, 0);
  if (d <= 0.0) { return vec3f(0.0, 0.0, -1.0e9); }
  let distance = o.lens.z / d;
  let ndc = vec2f((f32(q.x) + 0.5) / o.shape.z * 2.0 - 1.0, 1.0 - (f32(q.y) + 0.5) / o.shape.w * 2.0);
  return vec3f(ndc.x * distance * o.lens.y / o.lens.x, ndc.y * distance / o.lens.x, -distance);
}

@fragment fn occlusionFs(i: Out) -> @location(0) vec4f {
  let p = vec2i(i.pos.xy);
  if (textureLoad(depth, p, 0) <= 0.0) { return vec4f(1.0); }
  let c = position(p);
  let r = position(p + vec2i(1, 0));
  let l = position(p - vec2i(1, 0));
  let u = position(p - vec2i(0, 1));
  let d = position(p + vec2i(0, 1));
  let dx = select(c - l, r - c, abs(r.z - c.z) < abs(c.z - l.z));
  let dy = select(d - c, c - u, abs(c.z - u.z) < abs(d.z - c.z));
  var n = normalize(cross(dx, dy));
  if (dot(n, c) > 0.0) { n = -n; }
  let radius = o.shape.x;
  let onscreen = min(radius * o.lens.x * o.shape.w * 0.5 / -c.z, WIDEST);
  if (onscreen < 1.5) { return vec4f(1.0); }
  let turn = f32((p.x & 3) * 4 + (p.y & 3)) / 16.0 * 6.2831853;
  var sum = 0.0;
  for (var k = 0; k < SAMPLES; k = k + 1) {
    let t = (f32(k) + 0.5) / f32(SAMPLES);
    let a = f32(k) * 2.3999632 + turn;
    let q = position(p + vec2i(vec2f(cos(a), sin(a)) * t * onscreen));
    let v = q - c;
    let vv = dot(v, v);
    let f = max(radius * radius - vv, 0.0);
    // Above the face (a hair, so the face itself does not occlude), the nearer the more.
    sum = sum + f * f * f * max((dot(v, n) - 0.002 * -c.z) / (vv + 0.01), 0.0);
  }
  let r2 = radius * radius;
  let ao = max(LEAST, 1.0 - sum * o.shape.y * 5.0 / (r2 * r2 * r2 * f32(SAMPLES)));
  return vec4f(ao);
}
