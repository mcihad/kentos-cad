//! The geometry-core crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod crs;
mod crs_custom;
mod crs_ground;
mod crs_measure;
mod crs_ntv2;
mod crs_text;
mod data_search;
mod dimensions;
mod display;
mod field_reduce;
mod field_traverse;
mod geodesy;
mod golden;
mod label_text;
mod leader;
mod line_parts;
mod locks;
mod measure;
mod navigation;
mod numeric;
mod point_editor;
mod point_text;
mod polygonize;
mod reshape;
mod snap;
mod template_members;
mod text;
mod topology;
mod vertex_points;
mod vertex_table;
