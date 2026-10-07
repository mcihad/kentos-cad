//! The domain crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod exchange;
mod external;
mod fixtures;
mod identity;
mod layer_fields;
mod layer_rules;
mod snapshot_v2;
