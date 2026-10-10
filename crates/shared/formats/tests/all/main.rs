//! The formats crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod coords;
mod dxf;
mod dxf_units;
mod field_csv;
mod field_gsi;
mod field_gts7;
mod field_jobxml;
mod field_nikon;
mod field_sdr;
mod field_sniff;
mod field_write;
mod gnss;
mod multidim;
mod raster;
mod raster_georef;
mod raster_pyramid;
mod raster_timing;
mod table_file;
