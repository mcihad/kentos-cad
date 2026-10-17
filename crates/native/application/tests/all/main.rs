//! The application crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod array_assoc;
mod audit_cmd;
mod break_join_ties;
mod capabilities;
mod catalog;
mod change_effects;
mod cleanup_cmd;
mod clipboard_cmd;
mod create;
mod curves_edit;
mod delete;
mod divide;
mod edit;
mod entries;
mod fixtures;
mod forms_timing;
mod info_templates;
mod layers_find_cmd;
mod lengthen;
mod library_files;
mod match_props;
mod object_policy;
mod order_cmd;
mod polygon;
mod processing_sink;
mod protocol;
mod protocol_cases;
mod provider_timing;
mod python_calls;
mod reassociate;
mod sources;
mod standards_cmd;
mod stretch_ties;
mod tables_cmd;
mod transform;
mod units;
