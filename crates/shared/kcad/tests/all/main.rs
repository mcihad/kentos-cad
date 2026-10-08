//! The kcad crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod blocks;
mod columns;
mod dimensions;
mod elevations;
mod fixtures;
mod hatches;
mod images;
mod layer_states;
mod leaders;
mod line_parts;
mod linked_texts;
mod parts;
mod rasters;
mod robustness;
mod survey_sigmas;
mod texts;
mod topology;
