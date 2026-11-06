// Native extension only: shared v10/browser bindings remain unchanged.
// flags.zw are offset/count for strokes and hatches (unused there in v10).
@group(1) @binding(1) var<storage, read> dashPage: array<f32>;

fn dashCover(s: f32, k: f32, totalRaw: f32, onShare: f32, offsetRaw: f32) -> f32 {
  let count = st.flags.w;
  if (count == 0u) { return inlineDashCover(s, k, totalRaw, onShare, offsetRaw); }
  let total = totalRaw * k;
  if (total <= 0.0) { return 1.0; }
  let start = st.flags.z;
  if (total < 4.0) {
    var dots = 0.0;
    for (var i = 0u; i + 1u < count; i += 2u) {
      if (dashPage[start + i + 1u] > 0.0) {
        dots += max(0.0, 1.0 - dashPage[start + i] * k);
      }
    }
    return min(1.0, onShare + dots / total);
  }
  let t = fmod(s + offsetRaw * k, total);
  var acc = 0.0;
  var a = 0.0;
  var found = false;
  for (var i = 0u; i < count; i++) {
    let l = dashPage[start + i] * k;
    if ((i & 1u) == 0u && i + 1u < count && l < 1.0 && dashPage[start + i + 1u] > 0.0) {
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
// One element of the pattern: the uniform's eight for a short recipe, the page for a long one.
fn dashLen(i: u32) -> f32 {
  if (st.flags.w == 0u) {
    let d = array<f32, 8>(st.dash0.x, st.dash0.y, st.dash0.z, st.dash0.w, st.dash1.x, st.dash1.y, st.dash1.z, st.dash1.w);
    return d[i];
  }
  return dashPage[st.flags.z + i];
}
// A stroke's dashes without its dots, which are `dotCover`'s (ADR 1124): the segment that does not own a dot adds
// no pixel of it. Hatches keep `dashCover` and its one-pixel dot. Under four pixels the mean, as `dashCover`.
fn strokeDash(s: f32) -> f32 {
  let k = unitK();
  let total = st.a.y * k;
  if (total < 4.0) { return dashCover(s, k, st.a.y, st.a.z, st.a.w); }
  var count = st.flags.w;
  if (count == 0u) { count = 8u; }
  let t = fmod(s + st.a.w * k, total);
  var acc = 0.0;
  var a = 0.0;
  var found = false;
  for (var i = 0u; i < count; i++) {
    let raw = dashLen(i);
    let l = raw * k;
    var gap = 0.0;
    if (i + 1u < count) { gap = dashLen(i + 1u); }
    if ((i & 1u) == 0u && l < 1.0 && gap > 0.0 && raw != 0.0) {
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
// ADR 1124: a zero-length dash before a gap is a round dot of the line's weight (halfW its radius), centred on its
// place along the line, drawn by the segment its centre lies on (x along it, y across; a path's last segment its
// end too). The CPU twin is `dot_cover` (crates/native/style/src/cpu.rs). Under four pixels a pattern keeps its mean.
fn dotCover(s: f32, x: f32, y: f32, len: f32, last: bool, halfW: f32) -> f32 {
  let k = unitK();
  let total = st.a.y * k;
  if (total < 4.0) { return 0.0; }
  var count = st.flags.w;
  if (count == 0u) { count = 8u; }
  let t = fmod(s + st.a.w * k, total);
  var acc = 0.0;
  var a = 0.0;
  for (var i = 0u; i + 1u < count; i++) {
    let l = dashLen(i);
    if ((i & 1u) == 0u && l == 0.0 && dashLen(i + 1u) > 0.0) {
      var e = t - acc;
      if (e > 0.5 * total) { e -= total; } else if (e < -0.5 * total) { e += total; }
      let centre = x - e;
      // A sixty-fourth of a pixel decides a centre the rounding puts on a joint: the next segment draws it.
      if (centre >= -0.015625 && (centre < len - 0.015625 || (last && centre <= len + 0.015625))) {
        a = max(a, cover(halfW - length(vec2f(e, y))));
      }
    }
    acc += l * k;
  }
  return a;
}
