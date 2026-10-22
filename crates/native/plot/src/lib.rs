//! A drawing on paper (docs/adr/0252): what a sheet PDF takes from the
//! drawing, shared by the desktop's sheet mode and the headless command host
//! (Python, MCP), so a script, an agent and the window print the same.
//!
//! - [`layers`]: a layer through the style engine whole, at a map's scale on
//!   the paper's palette.
//! - [`vectors`]: the engine's batches as the PDF's paths.
//! - [`maps`]: a map frame's content: the shown layers' paths, the texts the
//!   host laid out.
//! - [`inputs`]: what a sheet reads of the drawing (its system, the layer
//!   tables' rows, the coordinate lists, the legends).
//! - [`fonts`]: the drawing's typefaces as the PDF embeds them.
//! - [`book`]: the project's book as a PDF without a window (the headless
//!   host's).
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod atlas;
pub mod book;
pub mod fonts;
mod images;
pub mod inputs;
pub mod labels;
pub mod layers;
pub mod maps;
mod raster;
mod plot_ink;
pub use raster::{MapImage, render_image as render_map_image,render_image_profile as render_map_image_profile};
pub mod vectors;
