// Native extension only (plan/render.md RM-11, docs/adr/9239): a pattern tile's SVG marks drawn from their triangles
// where the atlas's bitmap would be magnified. The program (kentos_native_style::vector_tiles, whose `covers` is this
// shader's reading on the CPU) sits in the layer's long-pattern page at flags.z, flags.w numbers long. The tile as
// `tileFs` takes it: a = (tile.x, tile.y, cos, sin), b = (shift.x, shift.y, opacity, 0). The area's own triangles
// bound what is drawn, as they bound the bitmap. Premultiplied out.
fn vtAt(i: u32) -> f32 { return dashPage[st.flags.z + i]; }
fn vtIdx(i: u32) -> u32 { return u32(dashPage[st.flags.z + i] + 0.5); }
fn vtSide(a: vec2f, b: vec2f, q: vec2f) -> f32 { return (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x); }

// The colours (a bit each, at most eight) of the mark at `mark` whose triangles hold the SVG point q.
fn vtInside(mark: u32, q: vec2f) -> u32 {
  let lo = vec2f(vtAt(mark + 2u), vtAt(mark + 3u));
  let hi = vec2f(vtAt(mark + 4u), vtAt(mark + 5u));
  if (q.x < lo.x || q.y < lo.y || q.x > hi.x || q.y > hi.y) { return 0u; }
  let nx = vtIdx(mark + 6u);
  let ny = vtIdx(mark + 7u);
  let span = max(hi - lo, vec2f(1e-9));
  let bx = min(u32(max(floor((q.x - lo.x) / span.x * f32(nx)), 0.0)), nx - 1u);
  let by = min(u32(max(floor((q.y - lo.y) / span.y * f32(ny)), 0.0)), ny - 1u);
  let bin = by * nx + bx;
  var hits = 0u;
  let colours = min(vtIdx(mark + 8u), 8u);
  for (var c = 0u; c < colours; c++) {
    let head = mark + 9u + c * 7u;
    let bins = vtIdx(head + 4u);
    let list = vtIdx(head + 5u);
    let tris = vtIdx(head + 6u);
    let last = vtIdx(bins + bin + 1u);
    for (var n = vtIdx(bins + bin); n < last; n++) {
      let t = tris + vtIdx(list + n) * 6u;
      let a = vec2f(vtAt(t), vtAt(t + 1u));
      let b = vec2f(vtAt(t + 2u), vtAt(t + 3u));
      let d = vec2f(vtAt(t + 4u), vtAt(t + 5u));
      if (vtSide(a, b, q) >= 0.0 && vtSide(b, d, q) >= 0.0 && vtSide(d, a, q) >= 0.0) {
        hits |= 1u << c;
        break;
      }
    }
  }
  return hits;
}

@fragment fn tileVectorFs(i: AreaOut) -> @location(0) vec4f {
  // A pixel's step on the ground across and down: the four samples where the drawing's 4× MSAA takes them (the
  // standard pattern, pixels from the middle, y down), so a mark's edge is smoothed as an area's is.
  let ex = dpdx(i.world);
  let ey = dpdy(i.world);
  var samples = array<vec2f, 4>(vec2f(-0.125, -0.375), vec2f(0.375, -0.125), vec2f(-0.375, 0.125), vec2f(0.125, 0.375));
  let size = st.a.xy;
  let r = st.a.zw;
  let places = vtIdx(0u);
  var out = vec4f(0.0);
  var p = 0u;
  loop {
    if (p >= places) { break; }
    let mark = vtIdx(1u + p * 8u + 6u);
    let svg = vec2f(vtAt(mark), vtAt(mark + 1u));
    // The samples each colour of this mark covers, over all its places (a staggered tile's two rows) and cells.
    var masks = array<u32, 8>(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
    var q = p;
    loop {
      if (q >= places || vtIdx(1u + q * 8u + 6u) != mark) { break; }
      let base = 1u + q * 8u;
      let middle = vec2f(vtAt(base) * size.x, vtAt(base + 1u) * size.y + vtAt(base + 2u) * size.x);
      let k = vtAt(base + 3u) * size.x;
      let turn = vec2f(vtAt(base + 4u), vtAt(base + 5u));
      let reach = vtAt(base + 7u) * size.x;
      for (var s = 0u; s < 4u; s++) {
        let w = i.world + ex * samples[s].x + ey * samples[s].y;
        let t = vec2f(r.x * w.x + r.y * w.y, -r.y * w.x + r.x * w.y) - st.b.xy;
        // The cells whose mark reaches the point (three across at most: a mark wider than two tiles is cut there).
        let i0 = ceil((t.x - middle.x - reach) / size.x);
        let i1 = min(floor((t.x - middle.x + reach) / size.x), i0 + 2.0);
        let j0 = ceil((t.y - middle.y - reach) / size.y);
        let j1 = min(floor((t.y - middle.y + reach) / size.y), j0 + 2.0);
        for (var cy = j0; cy <= j1; cy += 1.0) {
          for (var cx = i0; cx <= i1; cx += 1.0) {
            let d = t - (vec2f(cx, cy) * size + middle);
            // Into the mark: turned back, then SVG pixels from its top left (y down).
            let local = vec2f(turn.x * d.x + turn.y * d.y, -turn.y * d.x + turn.x * d.y) / k;
            let hits = vtInside(mark, vec2f(local.x + svg.x * 0.5, svg.y * 0.5 - local.y));
            for (var c = 0u; c < 8u; c++) {
              if ((hits & (1u << c)) != 0u) { masks[c] |= 1u << s; }
            }
          }
        }
      }
      q++;
    }
    // Its colours over what is drawn so far, each by the share of samples it covers.
    let colours = min(vtIdx(mark + 8u), 8u);
    for (var c = 0u; c < colours; c++) {
      let a = f32(countOneBits(masks[c])) * 0.25;
      if (a > 0.0) {
        let head = mark + 9u + c * 7u;
        let ink = vec4f(vtAt(head), vtAt(head + 1u), vtAt(head + 2u), vtAt(head + 3u));
        let src = vec4f(ink.rgb * ink.a, ink.a) * a;
        out = src + out * (1.0 - src.a);
      }
    }
    p = q;
  }
  out *= st.b.z;
  if (out.a < 0.004) { discard; }
  return out;
}
