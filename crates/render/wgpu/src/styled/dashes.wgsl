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
