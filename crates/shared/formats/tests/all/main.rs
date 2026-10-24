//! The formats crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod coords;
mod compress;
mod dxf;
mod cad_io;
mod cad_spatial;
mod cad_sheet;
mod dxf_units;
mod exif;
mod field_csv;
mod field_gsi;
mod field_gts7;
mod field_jobxml;
mod field_nikon;
mod field_sdr;
mod field_sniff;
mod field_write;
mod gnss;
mod kml;
mod kml_timing;
mod lin;
mod parquet;
mod pat;
mod pdf;
mod multidim;
mod raster;
mod raster_gcp;
mod raster_georef;
mod raster_mosaic;
mod raster_pyramid;
mod raster_timing;
mod table_file;
mod wgs84_write;
mod workbook;
mod workbook_timing;
