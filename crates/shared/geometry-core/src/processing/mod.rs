//! What the processing tools compute (`apps/web/src/processing/builtin`, docs/adr/0008
//! S4): corner numbering and edge-length labels. The page and the processing
//! worker ask the geometry store for them by id (`store::processing`); the
//! named operations serve single calls and the fixtures.

pub mod edge_lengths;
pub mod numbering;
