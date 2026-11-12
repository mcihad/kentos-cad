//! The plotter's tests: one binary (docs/adr/0170), each file a module.

#[path = "../support/atlas_drawing.rs"]
mod atlas_drawing;
#[path = "../support/files.rs"]
mod files;

mod atlas;
mod atlas_timing;
mod bands;
mod book;
mod map_layers;
mod map_levels;
mod map_view_cut;
mod ramp_levels;
mod stair_levels;
mod output;
mod units;

mod viewport_annotations;

mod multipage_sets;

mod page_setups;

mod plot_ink;

mod document_quality;

mod preview;

mod calibration;
mod coordinate_facts;
mod survey_facts;
mod outdated_drawings;

mod drawing_set;

mod source_snapshot;

mod line_look_order;
mod line_types;
mod curve_dashes;
mod dots;
mod shx_texts;
mod table_styles;
mod block_text_colours;
