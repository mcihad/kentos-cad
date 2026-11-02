//! A road's cross sections (docs/adr/0903, plan/road.md Y5): a sample line across an alignment at a station, the
//! terrain along it, and (in [`template`]) the platform a template builds on it and its slopes tied to the terrain.

pub mod sample;

pub use sample::{GroundPoint, SampleLine, ground, sample_line};
