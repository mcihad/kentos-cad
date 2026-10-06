//! Styled layers (docs/STYLE.md §6, docs/adr/0090): the style engine's
//! batches drawn with the shared styled WGSL (`shaders/wgsl/styled`,
//! `styled.layout.json`), as the web's WebGPU backend draws them. The batches
//! come from `kentos-native-style` (the style core's answer with its colours
//! and images); this module owns the GPU side:
//!
//! - [`gpu`]: pipelines per sample count, a layer's buffers (one vertex
//!   buffer, one uniform buffer of style blocks read at dynamic offsets), the
//!   frame's visibility checks and draw calls;
//! - [`atlas`]: the image page (SVG and raster markers, text, pattern
//!   tiles), drawn on the CPU in power-of-two steps ([`raster`], tiny-skia);
//! - [`picture`]: what the host hands the atlas (vector pictures, pixels,
//!   glyph outlines) so this crate reads no files and knows no fonts;
//! - [`uniform`]: the frame and style blocks as the contract lays them out;
//! - [`cpu`]: the same shaders on the CPU, for a sheet's map drawn as a
//!   picture (a PDF's, and the screen's where the map has something no
//!   vector writes).

pub mod atlas;
pub mod cpu;
pub mod gpu;
pub mod picture;
pub mod pictures;
pub mod raster;
pub mod shader;
pub mod uniform;

pub use gpu::{StyledFrame, StyledGpu, StyledLayerPart, StyledScene, ViewStyled};
pub use picture::{Bitmap, ImageSource, NoImages, Picture, TextOutline};
