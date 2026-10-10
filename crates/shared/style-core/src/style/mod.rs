//! The style engine (docs/STYLE.md): symbols compiled on objects' geometry
//! into drawing primitives and packed into GPU batches, a whole layer in
//! one call next to the geometry store (docs/adr/0008 “Stil derleyicisi”).
//! The page keeps what needs it: colours from the theme, images in the
//! atlas, the upload to the GPU.

pub mod batch;
pub mod build;
pub mod charts;
pub mod compile;
pub mod dots;
pub mod groups;
pub mod heat;
pub mod inverted;
pub mod model;
pub mod place;
pub mod prim;
pub mod resolve;
pub mod rules;
pub mod thematic;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod thematic_tests;
