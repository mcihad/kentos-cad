//! The contracts crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod address_rules;
mod annotation;
mod annotation_scale;
mod annotative;
mod crs;
mod document;
mod fields;
mod identity;
mod levels;
mod network_rules;
mod service_rules;
mod table_styles;
mod temporal_rules;
