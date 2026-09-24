/** Shared vertex transform: local (origin-relative) metres → clip space. */
const TRANSFORM = /* glsl */ `
uniform vec2 u_offset; // camera centre, origin-relative
uniform vec2 u_scale;  // clip units per metre (x, y)
vec4 toClip(vec2 p) { return vec4((p - u_offset) * u_scale, 0.0, 1.0); }
`;

export const LINE_VS = /* glsl */ `#version 300 es
in vec2 a_pos;
in float a_dist;
${TRANSFORM}
out float v_dist;
void main() {
  gl_Position = toClip(a_pos);
  v_dist = a_dist;
}`;

export const LINE_FS = /* glsl */ `#version 300 es
precision highp float;
uniform vec4 u_color;
uniform vec4 u_dash;      // on, off, on, off (device px); all zero = solid
uniform float u_pxPerUnit; // device px per metre
in float v_dist;
out vec4 outColor;
void main() {
  float len = u_dash.x + u_dash.y + u_dash.z + u_dash.w;
  if (len > 0.0) {
    float t = mod(v_dist * u_pxPerUnit, len);
    bool on = t < u_dash.x || (t >= u_dash.x + u_dash.y && t < u_dash.x + u_dash.y + u_dash.z);
    if (!on) discard;
  }
  outColor = u_color;
}`;

export const FILL_VS = /* glsl */ `#version 300 es
in vec2 a_pos;
${TRANSFORM}
void main() { gl_Position = toClip(a_pos); }`;

export const FILL_FS = /* glsl */ `#version 300 es
precision highp float;
uniform vec4 u_color;
out vec4 outColor;
void main() { outColor = u_color; }`;

export const POINT_VS = /* glsl */ `#version 300 es
in vec2 a_pos;
uniform float u_size; // device px
${TRANSFORM}
void main() {
  gl_Position = toClip(a_pos);
  gl_PointSize = u_size;
}`;

/** Survey symbols drawn procedurally: ring, cross (kot), triangle (poligon). */
export const POINT_FS = /* glsl */ `#version 300 es
precision highp float;
uniform vec4 u_color;
uniform float u_size;
uniform float u_dpr;
uniform int u_shape; // 0 ring, 1 cross, 2 triangle
out vec4 outColor;

float sdTriangle(vec2 p, float r) {
  const float k = 1.7320508;
  p.x = abs(p.x) - r;
  p.y = p.y + r / k;
  if (p.x + k * p.y > 0.0) p = vec2(p.x - k * p.y, -k * p.x - p.y) / 2.0;
  p.x -= clamp(p.x, -2.0 * r, 0.0);
  return -length(p) * sign(p.y);
}

void main() {
  vec2 p = (gl_PointCoord - 0.5) * u_size;
  p.y = -p.y;
  float R = u_size * 0.5 - 1.2 * u_dpr;
  float lw = 0.65 * u_dpr;
  float d;
  if (u_shape == 1) {
    vec2 q = abs(p);
    d = min(length(vec2(max(q.x - R, 0.0), q.y)), length(vec2(q.x, max(q.y - R, 0.0))));
  } else if (u_shape == 2) {
    d = abs(sdTriangle(p + vec2(0.0, R * 0.18), R * 0.95));
  } else {
    d = abs(length(p) - R);
  }
  float stroke = 1.0 - smoothstep(lw - 0.6, lw + 0.6, d);
  float dotA = 1.0 - smoothstep(-0.6, 0.6, length(p) - 1.0 * u_dpr);
  float a = max(stroke, u_shape == 1 ? 0.0 : dotA);
  if (a < 0.02) discard;
  outColor = vec4(u_color.rgb, u_color.a * a);
}`;
