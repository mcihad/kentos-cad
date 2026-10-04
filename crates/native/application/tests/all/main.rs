//! The application crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod catalog;
mod create;
mod delete;
mod edit;
mod fixtures;
mod polygon;
mod transform;
