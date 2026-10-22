//! The plotter's tests: one binary (docs/adr/0170), each file a module.

#[path = "../support/atlas_drawing.rs"]
mod atlas_drawing;
#[path = "../support/files.rs"]
mod files;

mod atlas;
mod book;
mod map_layers;
mod output;
mod units;

mod viewport_annotations;

mod multipage_sets;
