//! The sheet crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod common;
mod fixtures;
mod ops;
mod pdf;
mod templates;
mod wmm;
