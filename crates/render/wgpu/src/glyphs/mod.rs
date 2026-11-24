//! The drawing's text on the GPU (plan/render.md RM-13, docs/adr/9212): the glyphs of the drawing's faces as signed
//! distance fields in an atlas, each glyph of a text one instanced quad. Placing and layout stay with the host.
//!
//! - [`sdf`]: a glyph's field from its outline;
//! - [`atlas`]: the fields on pages of bytes, each made once;
//! - [`GlyphGpu`]: the pipeline, the pages as a texture array, and a set of texts as a [`GlyphLayer`] of instances.
//!
//! A text is anchored in the world ([`GlyphText::at`], float64, drawn from two float32 parts as the styled layers'
//! points are), its glyphs' em squares placed from the anchor in world metres (the drawing's own text: it grows with the
//! zoom and turns with the view) or in logical pixels (a label: it keeps its size). The edge and the halo come from one
//! sample of the field, smoothed over a device pixel, so the text needs no multisampling.

pub mod atlas;
mod gpu;
pub mod sdf;

pub use atlas::{GlyphAtlas, GlyphKey, Lookup, Slot};
pub use gpu::{GlyphGpu, GlyphLayer};

/// A flat quadrilateral drawn with a text (docs/adr/9212 §7): a mask under its letters, an underline over them, a
/// table cell's fill. The unit square's place from the text's anchor, in the text's unit, as [`GlyphPlace::transform`]
/// (a parallelogram), and its colour; its edges smoothed over a device pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphQuad {
    pub transform: [f64; 6],
    pub color: Rgba8,
}

use std::sync::Arc;

use kentos_geometry_core::text::shaping::placed::PlacedGlyph;

use crate::Rgba8;

/// A view's texts as its host hands them over (docs/adr/9212 §5): the renderer builds their layer again when the
/// version changes, and draws it over the view's picture every frame.
#[derive(Clone, Debug, Default)]
pub struct GlyphSet {
    pub version: u64,
    pub texts: Arc<Vec<GlyphText>>,
    /// The logical pixels a metre the pixel texts were placed at: while the view zooms they grow with it, as a picture
    /// of them would, until they are placed again (0: they keep their size).
    pub placed_px: f64,
}

/// What a text's sizes and offsets are in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlyphUnit {
    /// World metres: the text grows with the zoom and turns with the view.
    World,
    /// Logical pixels from the world anchor: the text keeps its size on screen; it `turns` with the view, or stays
    /// as it was placed on the screen.
    Px { turns: bool },
}

/// A glyph of a text: which one, and where its em square goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphPlace {
    pub key: GlyphKey,
    /// The glyph's em square (x right, y up, its origin at 0) into the text's unit from its anchor, as the geometry
    /// core's affines: x' = a x + c y + e, y' = b x + d y + f for [a, b, c, d, e, f].
    pub transform: [f64; 6],
}

impl GlyphPlace {
    /// A glyph of the geometry core's plan (`text::shaping::placed`): its size, synthetic slant and origin, then the
    /// plan's transform into model metres from the text's point.
    pub fn of(placed: &PlacedGlyph) -> GlyphPlace {
        let g = &placed.glyph;
        let own = [g.size, 0.0, g.size * g.slant, g.size, g.origin[0], g.origin[1]];
        GlyphPlace {
            key: GlyphKey { font: g.font, glyph: g.id },
            transform: then(&placed.transform, &own),
        }
    }
}

/// `outer` after `inner`.
fn then(outer: &[f64; 6], inner: &[f64; 6]) -> [f64; 6] {
    let [a, b, c, d, e, f] = *outer;
    let [p, q, r, s, t, u] = *inner;
    [a * p + c * q, b * p + d * q, a * r + c * s, b * r + d * s, a * t + c * u + e, b * t + d * u + f]
}

/// A text drawn with the glyph pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphText {
    /// Its anchor in the world, metres.
    pub at: [f64; 2],
    pub unit: GlyphUnit,
    pub color: Rgba8,
    /// The halo round its letters, and its width each side in logical pixels (none at 0). The field reaches an
    /// eighth of an em past the edge: a wider halo is cut to it (a 1.5 px halo is whole on letters of 12 px an em).
    pub halo: Rgba8,
    pub halo_px: f32,
    pub glyphs: Vec<GlyphPlace>,
    /// Quads drawn before its glyphs (a mask, a cell's fill), and after them (an underline and its halo).
    pub under: Vec<GlyphQuad>,
    pub over: Vec<GlyphQuad>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::Vec2;
    use kentos_geometry_core::entity::TextPlace;
    use kentos_geometry_core::text::Font;
    use kentos_geometry_core::text::shaping::{OutlineOp, placed};

    /// A glyph's outline through its place's transform lands where the geometry core's own outline is.
    #[test]
    fn a_placed_glyph_maps_its_em_square_as_the_plan_draws_its_outline() {
        let place = TextPlace {
            p: Vec2::new(0.0, 0.0),
            text: "Ağ",
            height: 2.5,
            rotation: 30.0,
            align: None,
            width_factor: Some(0.8),
            box_width: None,
            line_spacing: None,
            runs: &[],
            flow: None,
            font: None,
            bold: false,
            italic: false,
            lean: 0.2,
            path: None,
        };
        let layout = placed::layout(&place, Font::DEFAULT, false).expect("a layout");
        assert_eq!(layout.glyphs.len(), 2);
        for g in &layout.glyphs {
            let at = GlyphPlace::of(g);
            let bytes = kentos_geometry_core::text::shaping::font_bytes(g.glyph.font).expect("bytes");
            let face = ttf_parser::Face::parse(bytes, 0).expect("a face");
            let units = f64::from(face.units_per_em());
            // The outline's first point in the face's units, as an em, through the place.
            struct First(Option<[f32; 2]>);
            impl ttf_parser::OutlineBuilder for First {
                fn move_to(&mut self, x: f32, y: f32) {
                    self.0.get_or_insert([x, y]);
                }
                fn line_to(&mut self, _: f32, _: f32) {}
                fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
                fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
                fn close(&mut self) {}
            }
            let mut first = First(None);
            face.outline_glyph(ttf_parser::GlyphId(g.glyph.id), &mut first);
            let [x, y] = first.0.expect("an outline").map(|v| f64::from(v) / units);
            let m = at.transform;
            let ours = [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];
            let Some(OutlineOp::Move(theirs)) = g.outline().expect("an outline").first().cloned() else {
                panic!("an outline starts with a move")
            };
            assert!((ours[0] - theirs[0]).abs() < 1e-9 && (ours[1] - theirs[1]).abs() < 1e-9, "{ours:?} {theirs:?}");
        }
    }
}
