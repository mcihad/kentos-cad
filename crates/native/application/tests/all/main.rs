//! The application crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod array_assoc;
mod break_join_ties;
mod capabilities;
mod catalog;
mod cleanup_cmd;
mod create;
mod curves_edit;
mod delete;
mod divide;
mod edit;
mod fixtures;
mod info_templates;
mod lengthen;
mod match_props;
mod polygon;
mod protocol;
mod protocol_cases;
mod python_calls;
mod reassociate;
mod stretch_ties;
mod transform;
mod units;
