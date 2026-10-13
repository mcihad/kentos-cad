//! The kcad crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod addresses;
mod annotation;
mod attachments;
mod blocks;
mod columns;
mod compose;
mod dimension_refs;
mod dimensions;
mod elevations;
mod filters;
mod fixtures;
mod hatches;
mod images;
mod imperial;
mod layer_states;
mod leaders;
mod line_parts;
mod linked_texts;
mod look;
mod measures;
mod networks;
mod nurbs;
mod parts;
mod pointclouds;
mod polyline_extras;
mod rasters;
mod requests;
mod robustness;
mod services;
mod sheets;
mod survey_sigmas;
mod texts;
mod topology;
mod ucs;
