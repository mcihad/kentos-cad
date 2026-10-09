//! The styled pipelines' uniforms as `styled.layout.json` lays them out: the
//! frame (group 0) and one style block per batch (group 1), packed as the
//! web's WebGPU backend packs them (`webgpu/styledRenderer.ts` `styleData`).
//! Atlas rectangles are written into a block when the batch's image moves.

use bytemuck::{Pod, Zeroable};
use kentos_native_style::batches::{
    BatchKind, Cap, FillPaintBatch, MarkerLook, StyledBatch, Unit, shape_index,
};

/// The frame uniform (WGSL `Frame`, 40 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct StyledFrameUniform {
    /// Camera centre relative to the layers' origin, metres: the float32
    /// high part; `offset_lo` holds the rest (docs/adr/0157 §2).
    pub offset: [f32; 2],
    /// Clip units per metre (unused by the styled shaders; kept for the contract).
    pub scale: [f32; 2],
    /// Device pixels per metre.
    pub px_per_m: f32,
    /// Device pixels per logical pixel.
    pub dpr: f32,
    /// The drawing area in device pixels.
    pub viewport: [f32; 2],
    /// The camera centre's low part: `offset + offset_lo` is it to about
    /// 2⁻⁴⁸ of its size.
    pub offset_lo: [f32; 2],
}

/// Bytes of one style block (WGSL `SStyle`: eight `vec4f`, a `vec4u` and the
/// tile's `origin`).
pub const STYLE_BYTES: usize = 160;

/// A batch's style block: 36 four-byte words, floats and the four flags,
/// then its tile's origin (docs/adr/0157: x, y in metres from the layers'
/// origin, exact in float32; two words unused).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleBlock {
    pub f: [f32; 32],
    pub u: [u32; 4],
    pub origin: [f32; 4],
}

impl Default for StyleBlock {
    fn default() -> Self {
        Self {
            f: [0.0; 32],
            u: [0; 4],
            origin: [0.0; 4],
        }
    }
}

impl StyleBlock {
    pub fn bytes(&self) -> [u8; STYLE_BYTES] {
        let mut out = [0u8; STYLE_BYTES];
        out[..128].copy_from_slice(bytemuck::cast_slice(&self.f));
        out[128..144].copy_from_slice(bytemuck::cast_slice(&self.u));
        out[144..].copy_from_slice(bytemuck::cast_slice(&self.origin));
        out
    }

    fn set(&mut self, at: usize, values: &[f64]) {
        for (i, v) in values.iter().enumerate() {
            if let Some(slot) = self.f.get_mut(at + i) {
                // NaN (a colour the page could not read) draws as nothing, as a typed array holds it.
                *slot = *v as f32;
            }
        }
    }

    /// The atlas rectangle (x, y, w, h in 0..1, y down) at `rect`.
    pub fn set_rect(&mut self, uv: [f32; 4]) {
        self.f[16..20].copy_from_slice(&uv);
    }

    /// A marker image's proportions (height / width) at `b.x`.
    pub fn set_aspect(&mut self, aspect: f32) {
        self.f[24] = aspect;
    }
}

/// Dash values for the shader: an odd pattern is repeated to become even (as
/// in SVG), cut to eight, with the total and the share that is ink (`dashValues`).
pub fn dash_values(dash: Option<&[f64]>) -> ([f64; 8], f64, f64) {
    let mut d = [0.0; 8];
    let Some(dash) = dash.filter(|d| !d.is_empty()) else {
        return (d, 0.0, 1.0);
    };
    let even: Vec<f64> = if dash.len() % 2 == 1 {
        dash.iter().chain(dash).copied().collect()
    } else {
        dash.to_vec()
    };
    for (slot, v) in d.iter_mut().zip(even.iter()) {
        *slot = *v;
    }
    let total: f64 = d.iter().sum();
    let on: f64 = d.iter().step_by(2).sum();
    (d, total, if total > 0.0 { on / total } else { 1.0 })
}

/// A pattern's per-cell random shift and whether neighbour cells must be
/// looked at: the shape, its offset or its shift reach past half a cell (`patternReach`).
pub fn pattern_reach(
    half: [f64; 2],
    stroke_width: f64,
    size: [f64; 2],
    mark_offset: [f64; 2],
    jitter: f64,
) -> ([f64; 2], u32) {
    let ext = half[0].hypot(half[1]) + stroke_width / 2.0;
    let j = [
        (size[0] - 2.0 * ext).max(0.0) * jitter,
        (size[1] - 2.0 * ext).max(0.0) * jitter,
    ];
    let out = ext + mark_offset[0].hypot(mark_offset[1]) + j[0].max(j[1]) / 2.0;
    (j, u32::from(out > size[0].min(size[1]) / 2.0))
}

fn unit(u: Unit) -> u32 {
    match u {
        Unit::World => 0,
        Unit::Px => 1,
    }
}

const NONE: [f64; 4] = [0.0; 4];

/// A batch's style block; atlas rectangles are filled in when the image is placed.
pub fn style_block(b: &StyledBatch) -> StyleBlock {
    let mut s = StyleBlock {
        // Whole multiples of 2¹⁶ m: exact in float32.
        origin: [b.origin[0] as f32, b.origin[1] as f32, 0.0, 0.0],
        ..StyleBlock::default()
    };
    match &b.kind {
        BatchKind::Stroke {
            color,
            width,
            unit: u,
            dash,
            dash_offset,
            cap,
            blur,
        } => {
            s.set(0, color);
            let (d, total, on) = dash_values(dash.as_deref());
            s.set(8, &d);
            s.set(20, &[*width, total, on, *dash_offset]);
            s.set(24, &[*blur, 0.0, 0.0, 0.0]);
            let cap = match cap {
                Cap::Butt => 0,
                Cap::Round => 1,
                Cap::Square => 2,
            };
            s.u = [unit(*u), cap, 0, 0];
        }
        BatchKind::Fill { paint } => match paint {
            FillPaintBatch::Solid { color } => s.set(0, color),
            FillPaintBatch::Hatch {
                color,
                angle,
                spacing,
                width,
                offset,
                dash,
                dash_offset,
                stagger,
                unit: u,
            } => {
                s.set(0, color);
                let (d, total, on) = dash_values(dash.as_deref());
                s.set(8, &d);
                // A family's stagger (docs/adr/0186 §3).
                s.set(16, &[*stagger, 0.0, 0.0, 0.0]);
                s.set(20, &[angle.cos(), angle.sin(), *spacing, *width]);
                s.set(24, &[*offset, total, on, *dash_offset]);
                s.u[0] = unit(*u);
            }
            // A raster (docs/adr/0204 §5): b.z = opacity; its quads carry the rest.
            FillPaintBatch::Raster { opacity, .. } => s.set(24, &[0.0, 0.0, *opacity, 0.0]),
            // A point cloud (docs/adr/0207 §6): b.z = opacity; its picture carries the rest.
            FillPaintBatch::PointCloud { opacity, .. } => s.set(24, &[0.0, 0.0, *opacity, 0.0]),
            // A map service (docs/adr/0208 §3): b.z = opacity; its tiles' quads carry the rest.
            FillPaintBatch::Service { opacity, .. } => s.set(24, &[0.0, 0.0, *opacity, 0.0]),
            // A picture (docs/adr/0192 §3): a = (corner, width, height), b = (cos, sin, opacity, mirror).
            FillPaintBatch::Image {
                corner,
                size,
                angle,
                mirror,
                opacity,
                ..
            } => {
                s.set(20, &[corner[0], corner[1], size[0], size[1]]);
                s.set(
                    24,
                    &[
                        angle.cos(),
                        angle.sin(),
                        *opacity,
                        if *mirror { 1.0 } else { 0.0 },
                    ],
                );
            }
            FillPaintBatch::Gradient {
                color,
                color2,
                shape,
                inverted,
                dir,
                from,
                to,
                centre,
                radius,
            } => {
                s.set(0, color);
                s.set(4, color2);
                s.set(20, &[dir.cos(), dir.sin(), *from, *to]);
                s.set(24, &[centre[0], centre[1], *radius, 0.0]);
                s.u = [0, *shape, u32::from(*inverted), 0];
            }
            FillPaintBatch::Pattern {
                shape,
                fill,
                stroke,
                stroke_width,
                half,
                mark_offset,
                mark_rotation,
                params,
                size,
                stagger,
                angle,
                offset,
                jitter,
                coverage,
                seed,
                tint,
                opacity,
                unit: u,
            } => {
                let (j, reach) = pattern_reach(*half, *stroke_width, *size, *mark_offset, *jitter);
                s.set(0, fill.as_ref().unwrap_or(&NONE));
                s.set(4, stroke.as_ref().unwrap_or(&NONE));
                s.set(8, &[size[0], size[1], angle.cos(), angle.sin()]);
                s.set(12, &[offset[0], offset[1], j[0], j[1]]);
                s.set(16, &[half[0], half[1], mark_offset[0], mark_offset[1]]);
                s.set(
                    20,
                    &[mark_rotation.cos(), mark_rotation.sin(), *coverage, *seed],
                );
                s.set(24, &[*stroke_width, *tint, *opacity, 0.0]);
                s.set(28, params);
                s.u = [unit(*u), u32::from(*stagger), shape_index(shape), reach];
            }
            FillPaintBatch::Tile {
                size,
                angle,
                offset,
                opacity,
                unit: u,
                ..
            } => {
                s.set(20, &[size[0], size[1], angle.cos(), angle.sin()]);
                s.set(24, &[offset[0], offset[1], *opacity, 0.0]);
                s.u[0] = unit(*u);
            }
        },
        BatchKind::Marker {
            unit: u,
            look,
            offset,
            anchor,
            opacity,
            ..
        } => {
            s.set(20, &[offset[0], offset[1], anchor[0], anchor[1]]);
            match look {
                MarkerLook::Shape {
                    shape,
                    fill,
                    stroke,
                    stroke_width,
                    params,
                } => {
                    s.set(0, fill.as_ref().unwrap_or(&NONE));
                    s.set(4, stroke.as_ref().unwrap_or(&NONE));
                    s.set(24, &[1.0, *stroke_width, *opacity, 0.0]);
                    s.set(28, params);
                    s.u = [unit(*u), 0, shape_index(shape), 0];
                }
                MarkerLook::Image { fit_height, .. } => {
                    s.set(24, &[1.0, 0.0, *opacity, 0.0]);
                    s.u = [unit(*u), 1, 0, if *fit_height { 2 } else { 1 }];
                }
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashes_are_made_even_and_summed() {
        let (d, total, on) = dash_values(Some(&[3.0, 1.5]));
        assert_eq!(&d[..4], &[3.0, 1.5, 0.0, 0.0]);
        assert_eq!(total, 4.5);
        assert!((on - 3.0 / 4.5).abs() < 1e-12);
        let (d, total, _) = dash_values(Some(&[2.0]));
        assert_eq!(&d[..3], &[2.0, 2.0, 0.0]);
        assert_eq!(total, 4.0);
        assert_eq!(dash_values(None).1, 0.0);
    }

    #[test]
    fn a_block_is_the_contract_s_size() {
        assert_eq!(StyleBlock::default().bytes().len(), STYLE_BYTES);
        assert_eq!(std::mem::size_of::<StyledFrameUniform>(), 40);
    }
}
