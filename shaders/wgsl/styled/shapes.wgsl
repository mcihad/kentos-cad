// Styled drawing: the marker shapes as distance fields, shared by markers and pattern fills.
fn mBox(p: vec2f, b: vec2f) -> f32 { let d = abs(p) - b; return max(d.x, d.y); }
fn sdSeg(p: vec2f, a: vec2f, b: vec2f) -> f32 { let pa = p - a; let ba = b - a; let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0); return length(pa - ba * h); }
fn mRhombus(q: vec2f, b: vec2f) -> f32 { let p = abs(q); return (p.x * b.y + p.y * b.x - b.x * b.y) / length(b); }
fn mNgon(p: vec2f, a: f32, n: f32, a0: f32) -> f32 {
  if (dot(p, p) < 1e-12) { return -a; }
  let s = 6.2831853 / n;
  var t = atan2(p.y, p.x) - a0;
  t -= s * floor(t / s + 0.5);
  return length(p) * cos(t) - a;
}
fn sdStar(q: vec2f, r: f32, rf: f32) -> f32 {
  let k1 = vec2f(0.809016994375, -0.587785252292);
  let k2 = vec2f(-0.809016994375, -0.587785252292);
  var p = vec2f(abs(q.x), q.y);
  p -= 2.0 * max(dot(k1, p), 0.0) * k1;
  p -= 2.0 * max(dot(k2, p), 0.0) * k2;
  p.x = abs(p.x);
  p.y -= r;
  let ba = rf * vec2f(-k1.y, k1.x) - vec2f(0.0, 1.0);
  let h = clamp(dot(p, ba) / dot(ba, ba), 0.0, r);
  return length(p - ba * h) * sign(p.y * ba.x - p.x * ba.y);
}
// Gear and open arc (see the GLSL twin).
fn sdGear(p: vec2f, r: f32, n: f32, depth: f32) -> f32 {
  let rb = r * (1.0 - depth);
  if (dot(p, p) < 1e-12) { return -rb; }
  let a = 6.2831853 / n;
  let k = floor(atan2(p.y, p.x) / a + 0.5) * a;
  let q = vec2f(cos(k) * p.x + sin(k) * p.y, -sin(k) * p.x + cos(k) * p.y);
  let hw = 0.25 * a * (rb + 0.5 * (r - rb));
  let x0 = rb - 0.5 * (r - rb);
  return min(length(p) - rb, mBox(q - vec2f(0.5 * (x0 + r), 0.0), vec2f(0.5 * (r - x0), hw)));
}
fn sdArc(q: vec2f, r: f32, ap: f32) -> f32 {
  let p = vec2f(abs(q.x), q.y);
  let sc = vec2f(sin(ap), cos(ap));
  if (sc.y * p.x > sc.x * p.y) { return length(p - sc * r); }
  return abs(length(p) - r);
}
// sp: hole (share of the radius), teeth, opening (radians), tooth depth.
fn shapeBase(s: u32, p: vec2f, hs: vec2f, sp: vec4f) -> f32 {
  let r = min(hs.x, hs.y);
  switch s {
    case 0u, 1u: { return length(p) - r; }
    case 2u: { return mBox(p, vec2f(r)); }
    case 3u: { return mBox(p, hs); }
    case 4u: { return mRhombus(p, hs); }
    case 5u: { return mNgon(p, r * 0.57735027, 3.0, -1.5707963); }
    case 6u: { return mNgon(p, r, 5.0, -1.5707963); }
    case 7u: { return mNgon(p, r, 6.0, 0.0); }
    case 8u: { return mNgon(p, r, 8.0, 0.0); }
    case 9u: { return sdStar(p, r, 0.4); }
    case 10u: { return min(sdSeg(p, vec2f(-r, 0.0), vec2f(r, 0.0)), sdSeg(p, vec2f(0.0, -r), vec2f(0.0, r))); }
    case 11u: { let d = r * 0.7071068; return min(sdSeg(p, vec2f(-d), vec2f(d)), sdSeg(p, vec2f(-d, d), vec2f(d, -d))); }
    case 12u: { return sdSeg(p, vec2f(-hs.x, 0.0), vec2f(hs.x, 0.0)); }
    case 13u: { return min(sdSeg(p, vec2f(-hs.x, 0.0), vec2f(hs.x, 0.0)), min(sdSeg(p, vec2f(hs.x * 0.5, hs.y * 0.4), vec2f(hs.x, 0.0)), sdSeg(p, vec2f(hs.x * 0.5, -hs.y * 0.4), vec2f(hs.x, 0.0)))); }
    case 14u: {
      let a = vec2f(hs.x, 0.0);
      let b = vec2f(-hs.x, hs.y * 0.8);
      let c = vec2f(-hs.x, -hs.y * 0.8);
      let d = min(sdSeg(p, a, b), min(sdSeg(p, b, c), sdSeg(p, c, a)));
      let inside = p.x >= -hs.x && abs(p.y) <= (hs.x - p.x) * 0.4 * hs.y / hs.x;
      if (inside) { return -d; }
      return d;
    }
    case 15u: { return min(sdSeg(p, vec2f(-hs.x, hs.y), vec2f(hs.x, 0.0)), sdSeg(p, vec2f(hs.x, 0.0), vec2f(-hs.x, -hs.y))); }
    case 16u: { return max(length(p) - r, -p.y); }
    case 18u: { return sdGear(p, r, max(sp.y, 3.0), sp.w); }
    case 19u: { return sdArc(p, r, min(sp.z * 0.5, 3.1415927)); }
    default: { return max(length(p) - r, max(-p.x, -p.y)); }
  }
}
fn isOpen(s: u32) -> bool { return s == 10u || s == 11u || s == 12u || s == 13u || s == 15u || s == 19u; }
fn shapeDist(s: u32, p: vec2f, hs: vec2f, sp: vec4f) -> f32 {
  var d = shapeBase(s, p, hs, sp);
  if (sp.x > 0.0 && !isOpen(s)) { d = max(d, sp.x * min(hs.x, hs.y) - length(p)); }
  return d;
}
// A shape's colour (premultiplied) from its distance d in device px; q is the point in px (ring dot).
fn shapeColor(shape: u32, d: f32, q: vec2f, hs: vec2f, fill: vec4f, strokeIn: vec4f, swIn: f32) -> vec4f {
  let open = isOpen(shape);
  var stroke = strokeIn;
  if (stroke.a <= 0.0) { if (open) { stroke = fill; } else { stroke = vec4f(0.0); } }
  var sw = swIn;
  if (sw <= 0.0 && open) { sw = max(frame.dpr, 1.0); }
  var col = vec4f(0.0);
  if (!open && fill.a > 0.0) { col = vec4f(fill.rgb * fill.a, fill.a) * clamp(0.5 - d, 0.0, 1.0); }
  if (stroke.a > 0.0 && sw > 0.0) {
    let sa = clamp(sw * 0.5 + 0.5 - abs(d), 0.0, 1.0);
    let sc = vec4f(stroke.rgb * stroke.a, stroke.a) * sa;
    col = sc + col * (1.0 - sc.a);
  }
  if (shape == 1u && stroke.a > 0.0) {
    let dt = clamp(max(1.2 * frame.dpr, hs.x * 0.16) + 0.5 - length(q), 0.0, 1.0);
    let dc = vec4f(stroke.rgb * stroke.a, stroke.a) * dt;
    col = dc + col * (1.0 - dc.a);
  }
  return col;
}
