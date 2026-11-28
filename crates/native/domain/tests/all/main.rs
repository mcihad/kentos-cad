//! The domain crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod arrays;
mod blocks;
mod constraints;
mod data_edits;
mod dimension_sets;
mod dimension_ties;
mod exchange;
mod external;
mod field_ids;
mod fixtures;
mod hatch_boundaries;
mod hatch_texts;
mod identity;
mod layer_fields;
mod layer_rules;
mod patches;
mod related;
mod related_sums;
mod relation_keys;
mod snapshot_v2;
mod steps;
mod style_tables;
