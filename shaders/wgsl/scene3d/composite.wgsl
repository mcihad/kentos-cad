// A 3D view drawn in its own target laid over the window (docs/adr/8120): one triangle over the viewport reads the
// view's target texel by texel (the viewport is as many pixels as the target; a view partly off the window reads the
// part on it, `place.uv`), premultiplied, and the host blends it over what is under it (the sky). When the quality
// asks, the colour is darkened by the ambient light's occlusion averaged over the 4 × 4 texels its pattern repeats in,
// lifted where a see-through face veils the pixel (by its alpha: a window's glass more than a room's pale volume).

struct Place {
  uv: vec4f,    // xy: the target's corner the viewport's top left shows, zw: its bottom right (0–1)
  shade: vec4f, // x: 1 to darken by the occlusion; yzw: unused
};
@group(0) @binding(0) var colour: texture_2d<f32>;
@group(0) @binding(1) var occlusion: texture_2d<f32>;
@group(0) @binding(2) var<uniform> place: Place;
@group(0) @binding(3) var veil: texture_2d<f32>;

struct Out {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
};

@vertex fn compositeVs(@builtin(vertex_index) i: u32) -> Out {
  // (0, 0), (2, 0), (0, 2): a triangle whose middle covers the viewport.
  let t = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  var o: Out;
  o.pos = vec4f(t.x * 2.0 - 1.0, 1.0 - t.y * 2.0, 0.0, 1.0);
  o.uv = place.uv.xy + t * (place.uv.zw - place.uv.xy);
  return o;
}

@fragment fn compositeFs(i: Out) -> @location(0) vec4f {
  let size = vec2i(textureDimensions(colour));
  let at = clamp(vec2i(i.uv * vec2f(size)), vec2i(0), size - 1);
  let c = textureLoad(colour, at, 0);
  if (place.shade.x < 0.5) { return c; }
  var sum = 0.0;
  for (var y = -2; y < 2; y = y + 1) {
    for (var x = -2; x < 2; x = x + 1) {
      sum = sum + textureLoad(occlusion, clamp(at + vec2i(x, y), vec2i(0), size - 1), 0).r;
    }
  }
  let lifted = mix(sum / 16.0, 1.0, textureLoad(veil, at, 0).r);
  return vec4f(c.rgb * lifted, c.a);
}
