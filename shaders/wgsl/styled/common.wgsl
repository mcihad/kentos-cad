// Styled drawing (docs/STYLE.md §6): the frame and batch style uniforms, the atlas, and helpers every styled pipeline uses.
struct Frame {
  offset: vec2f,     // camera centre, origin-relative metres
  scale: vec2f,
  pxPerM: f32,       // device px per metre
  dpr: f32,
  viewport: vec2f,   // device px
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
};
// Two groups (contract version 2): the frame and the atlas together, then the batch's style. A native wgpu
// device may offer only two bind groups (Iced's does), so the atlas sits beside the frame.
@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var atlasTex: texture_2d<f32>;
@group(0) @binding(2) var atlasSmp: sampler;
@group(1) @binding(0) var<uniform> st: SStyle;

fn toPx(p: vec2f) -> vec2f { return (p - frame.offset) * frame.pxPerM; }
fn pxToClip(px: vec2f) -> vec4f { return vec4f(px / (0.5 * frame.viewport), 0.0, 1.0); }
fn unitK() -> f32 { if (st.flags.x == 0u) { return frame.pxPerM; } return frame.dpr; }
fn fmod(x: f32, y: f32) -> f32 { return x - y * floor(x / y); }

fn corner(i: u32) -> vec2f {
  var c = array<vec2f, 6>(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(0.0, 1.0));
  return c[i];
}

// Dash coverage: s in device px, pattern (raw units) scaled by k; total/on/offset as for WebGL2.
fn dashCover(s: f32, k: f32, totalRaw: f32, onShare: f32, offsetRaw: f32) -> f32 {
  let total = totalRaw * k;
  if (total <= 0.0) { return 1.0; }
  if (total < 4.0) { return onShare; }
  let t = fmod(s + offsetRaw * k, total);
  let d = array<f32, 8>(st.dash0.x, st.dash0.y, st.dash0.z, st.dash0.w, st.dash1.x, st.dash1.y, st.dash1.z, st.dash1.w);
  var acc = 0.0;
  for (var i = 0u; i < 8u; i++) {
    let l = d[i] * k;
    if (t < acc + l) {
      if ((i & 1u) == 1u) { return 0.0; }
      return clamp(min(t - acc, acc + l - t) + 0.5, 0.0, 1.0);
    }
    acc += l;
  }
  return 0.0;
}

