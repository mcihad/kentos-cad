// Styled drawing: triangulated areas with a paint computed per pixel (solid, hatch, gradient, tile, pattern).
// ── Areas ──
struct AreaOut {
  @builtin(position) pos: vec4f,
  @location(0) world: vec2f,
};
@vertex fn areaVs(@location(0) p: vec2f) -> AreaOut {
  var o: AreaOut;
  o.pos = pxToClip(toPx(p));
  o.world = p;
  return o;
}
@fragment fn solidFs(i: AreaOut) -> @location(0) vec4f {
  return st.color;
}
// Hatch: a = (dir.x, dir.y, spacing, width), b = (offset, dashTotal, dashOn, dashOffset), rect.x = stagger (a line's
// dashes that much further along than the line's before, docs/adr/0186 §3), flags.x = unit.
@fragment fn hatchFs(i: AreaOut) -> @location(0) vec4f {
  let screen = st.flags.x == 1u;
  var p = i.world;
  var k = frame.pxPerM;
  if (screen) { p = i.pos.xy * vec2f(1.0, -1.0) / frame.dpr; k = frame.dpr; }
  let dir = st.a.xy;
  let n = vec2f(-dir.y, dir.x);
  let halfW = max(st.a.w * k, 1.0) * 0.5;
  let gap = st.a.z * k;
  var a: f32;
  if (gap < 3.0) {
    // An even tint that fades from the hairline look to the paper's coverage (see the GLSL twin).
    let hair = min(1.0, 2.0 * halfW / max(gap, 1e-4));
    let paper = min(1.0, st.a.w / max(st.a.z, 1e-9));
    a = mix(paper, hair, clamp((gap - 1.0) / 2.0, 0.0, 1.0));
  } else {
    let u = dot(p, n) - st.b.x;
    let d = abs(fract(u / st.a.z + 0.5) - 0.5) * gap;
    a = clamp(halfW + 0.5 - d, 0.0, 1.0);
    let row = floor(u / st.a.z + 0.5);
    a *= dashCover((dot(p, dir) - row * st.rect.x) * k, k, st.b.y, st.b.z, st.b.w);
  }
  if (a < 0.004) { discard; }
  return vec4f(st.color.rgb, st.color.a * a);
}
// Gradient (docs/adr/0186 §3): color = first, stroke = second, a = (dir.x, dir.y, from, to), b = (centre.x, centre.y,
// radius, 0), flags = (unit, shape: 0 linear, 1 cylinder, 2 spherical, inverted, 0).
@fragment fn gradientFs(i: AreaOut) -> @location(0) vec4f {
  let p = i.world;
  var t: f32;
  if (st.flags.y == 2u) {
    t = 1.0 - clamp(length(p - st.b.xy) / max(st.b.z, 1e-9), 0.0, 1.0);
  } else {
    let u = clamp((dot(p, st.a.xy) - st.a.z) / max(st.a.w - st.a.z, 1e-9), 0.0, 1.0);
    t = select(u, 1.0 - abs(2.0 * u - 1.0), st.flags.y == 1u);
  }
  if (st.flags.z == 1u) { t = 1.0 - t; }
  return mix(st.color, st.stroke, t);
}
// Tile: rect, a = (tile.x, tile.y, cos, sin), b = (shift.x, shift.y, opacity, 0), flags.x = unit. Premultiplied out.
@fragment fn tileFs(i: AreaOut) -> @location(0) vec4f {
  var p = i.world;
  if (st.flags.x == 1u) { p = i.pos.xy * vec2f(1.0, -1.0) / frame.dpr; }
  let r = st.a.zw;
  p = vec2f(r.x * p.x + r.y * p.y, -r.y * p.x + r.x * p.y) - st.b.xy;
  let f = fract(p / st.a.xy);
  let inset = vec2f(0.5 / 2048.0);
  let uv = st.rect.xy + inset + vec2f(f.x, 1.0 - f.y) * (st.rect.zw - 2.0 * inset);
  let c = textureSampleLevel(atlasTex, atlasSmp, uv, 0.0) * st.b.z;
  if (c.a < 0.004) { discard; }
  return c;
}

fn hash3(p: vec2f) -> vec3f {
  var q = fract(vec3f(p.xyx) * vec3f(0.1031, 0.1030, 0.0973));
  q += dot(q, q.yxz + 33.33);
  return fract((q.xxy + q.yzz) * q.zyx);
}

// Pattern: color = fill, stroke, dash0 = (size, cos, sin), dash1 = (shift, jitter), rect = (half, markOffset),
// a = (markRot cos, sin, coverage, seed), b = (strokeW, tint, opacity, 0), flags = (unit, stagger, shape, reach). Premultiplied out.
@fragment fn patternFs(i: AreaOut) -> @location(0) vec4f {
  var p = i.world;
  var k = frame.pxPerM;
  if (st.flags.x == 1u) { p = i.pos.xy * vec2f(1.0, -1.0) / frame.dpr; k = frame.dpr; }
  let rot = st.dash0.zw;
  p = vec2f(rot.x * p.x + rot.y * p.y, -rot.y * p.x + rot.x * p.y) - st.dash1.xy;
  let size = st.dash0.xy;
  let shape = st.flags.z;
  var col = vec4f(0.0);
  if (min(size.x, size.y) * k < 4.0) {
    var ink = st.color;
    if (!(st.color.a > 0.0 && !isOpen(shape)) && st.stroke.a > 0.0) { ink = st.stroke; }
    let a = ink.a * st.b.y;
    col = vec4f(ink.rgb * a, a);
  } else {
    var best = 1e9;
    var bestQ = vec2f(0.0);
    let reach = i32(st.flags.w);
    let row0 = floor(p.y / size.y);
    for (var dy = -1; dy <= 1; dy++) {
      if (abs(dy) > reach) { continue; }
      let row = row0 + f32(dy);
      var sx = 0.0;
      if (st.flags.y == 1u && fmod(row, 2.0) > 0.5) { sx = 0.5 * size.x; }
      let col0 = floor((p.x - sx) / size.x);
      for (var dx = -1; dx <= 1; dx++) {
        if (abs(dx) > reach) { continue; }
        let cell = vec2f(col0 + f32(dx), row);
        let h = hash3(cell + vec2f(st.a.w * 17.31, st.a.w * 7.73));
        if (h.z > st.a.z) { continue; }
        let c = vec2f((cell.x + 0.5) * size.x + sx, (row + 0.5) * size.y) + (h.xy - 0.5) * st.dash1.zw;
        var q = p - c - st.rect.zw;
        let mr = st.a.xy;
        q = vec2f(mr.x * q.x + mr.y * q.y, -mr.y * q.x + mr.x * q.y) * k;
        let d = shapeDist(shape, q, st.rect.xy * k, st.c);
        if (d < best) { best = d; bestQ = q; }
      }
    }
    if (best > 1e8) { discard; }
    var sw = 0.0;
    if (st.b.x > 0.0) { sw = max(st.b.x * k, 1.0); }
    col = shapeColor(shape, best, bestQ, st.rect.xy * k, st.color, st.stroke, sw);
  }
  col *= st.b.z;
  if (col.a < 0.004) { discard; }
  return col;
}

