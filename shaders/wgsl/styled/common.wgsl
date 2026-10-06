// Styled drawing (docs/STYLE.md §6): the frame and batch style uniforms, the atlas, and helpers every styled pipeline uses.
struct Frame {
  offset: vec2f,     // camera centre, origin-relative metres: the float32 high part
  scale: vec2f,
  pxPerM: f32,       // device px per metre
  dpr: f32,
  viewport: vec2f,   // device px
  offsetLo: vec2f,   // the camera centre's low part (contract version 3)
};
struct SStyle {
  color: vec4f,
  stroke: vec4f,
  dash0: vec4f,
  dash1: vec4f,
  rect: vec4f,
  a: vec4f,
  b: vec4f,
  c: vec4f,
  flags: vec4u,      // unit, cap | kind, shape, fit
  origin: vec4f,     // xy: the batch's tile from the layers' origin, metres, exact (zw unused)
};
// Two groups (contract version 2): the frame and the atlas together, then the batch's style. A native wgpu
// device may offer only two bind groups (Iced's does), so the atlas sits beside the frame.
@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var atlasTex: texture_2d<f32>;
@group(0) @binding(2) var atlasSmp: sampler;
@group(1) @binding(0) var<uniform> st: SStyle;

// A position is from its batch's tile (docs/adr/0157). The camera's high part less the tile's origin is exact (the
// origin is a whole multiple of 2^16 m near it); a position near the camera less that is exact too, and small; the
// camera's low part comes off last, so no step of the camera is lost however far the tile lies from the anchor.
fn toPx(p: vec2f) -> vec2f { return ((p - (frame.offset - st.origin.xy)) - frame.offsetLo) * frame.pxPerM; }
fn pxToClip(px: vec2f) -> vec4f { return vec4f(px / (0.5 * frame.viewport), 0.0, 1.0); }
fn unitK() -> f32 { if (st.flags.x == 0u) { return frame.pxPerM; } return frame.dpr; }
fn fmod(x: f32, y: f32) -> f32 { return x - y * floor(x / y); }

fn corner(i: u32) -> vec2f {
  var c = array<vec2f, 6>(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 1.0));
  return c[i];
}

// Dash coverage: s in device px, pattern (raw units) scaled by k; total/on/offset as for WebGL2.
// A drawn dash shorter than a pixel followed by a gap is a dot: it inks a pixel wherever it falls (docs/adr/0186 §3).
fn dashCover(s: f32, k: f32, totalRaw: f32, onShare: f32, offsetRaw: f32) -> f32 {
  let total = totalRaw * k;
  if (total <= 0.0) { return 1.0; }
  let d = array<f32, 8>(st.dash0.x, st.dash0.y, st.dash0.z, st.dash0.w, st.dash1.x, st.dash1.y, st.dash1.z, st.dash1.w);
  if (total < 4.0) {
    var dots = 0.0;
    for (var i = 0u; i < 8u; i += 2u) {
      if (d[i + 1u] > 0.0) { dots += max(0.0, 1.0 - d[i] * k); }
    }
    return min(1.0, onShare + dots / total);
  }
  let t = fmod(s + offsetRaw * k, total);
  var acc = 0.0;
  var a = 0.0;
  var found = false;
  for (var i = 0u; i < 8u; i++) {
    let l = d[i] * k;
    if ((i & 1u) == 0u && l < 1.0 && d[i + 1u] > 0.0) {
      let e = abs(t - (acc + 0.5 * l));
      a = max(a, clamp(1.0 - min(e, total - e), 0.0, 1.0));
    }
    if (!found && t < acc + l) {
      found = true;
      if ((i & 1u) == 0u) { a = max(a, clamp(min(t - acc, acc + l - t) + 0.5, 0.0, 1.0)); }
    }
    acc += l;
  }
  return a;
}

