//! The contracts crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod annotation;
mod crs;
mod document;
mod fields;
mod identity;
