//! The desktop's half of the style engine (docs/STYLE.md; the web's page:
//! `render/styledLayer.ts`, `render/styledBatches.ts`, `style/library.ts`,
//! `style/fromLayer.ts`). The shared style core (`kentos-style-core`)
//! compiles symbols on the geometry store's shapes and packs GPU batches;
//! this crate gives it what the page gives it on the web and reads its answer:
//!
//! - [`library`]: symbols and assets from three sources (the system library
//!   that ships with KentOS, the user's, the project's), read-only for the
//!   system one; [`system`] holds the shipped library, generated from the
//!   web's TypeScript (`apps/web/scripts/style/system-library.test.ts`).
//! - [`simple`]: a layer's simple look (colour, line type, weight, fill,
//!   point symbol) and a hatch object's own pattern as symbols, so every layer
//!   goes through the same drawing path.
//! - [`program`]: one layer's build: how each object is drawn, the program
//!   the core reads, the objects' values for its expressions ([`table`]).
//! - [`batches`]: the core's batches with their colours from the theme, their
//!   atlas images and how far they reach: what the renderer draws.
//!
//! Both platforms are held to `fixtures/style/v1/batches.json` (a layer's
//! way to the GPU) and `cases.json` (the core's answers).
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod batches;
pub mod color;
pub mod library;
pub mod program;
pub mod simple;
pub mod system;
pub mod table;

pub use batches::{
    AtlasImage, Cap, FillPaintBatch, MarkerLook, StyledBatch, StyledLayer, TileMark, Unit,
};
pub use color::StylePalette;
pub use library::{Item, ItemKind, Source, StyleLibrary};
pub use program::{BuildOptions, LayerCall, symbol_scale_of};
