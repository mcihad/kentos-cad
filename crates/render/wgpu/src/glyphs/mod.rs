//! The drawing's text on the GPU (plan/render.md RM-13, docs/adr/9212): the glyphs of the drawing's faces as signed
//! distance fields in an atlas, each glyph of a text one instanced quad. Placing and layout stay with the host.

pub mod sdf;
