//! The formats crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.
//! A codec's files build with its feature (docs/adr/8054 §11): `--no-default-features` runs the rest.

#[cfg(feature = "dxf")]
mod cad_io;
#[cfg(feature = "dxf")]
mod cad_sheet;
#[cfg(feature = "dxf")]
mod cad_spatial;
#[cfg(feature = "multidim")]
mod chunked;
#[cfg(any(feature = "parquet", feature = "raster"))]
mod compress;
mod coords;
#[cfg(feature = "dxf")]
mod dimension_text_style;
#[cfg(feature = "dxf")]
mod dxf;
#[cfg(feature = "dxf")]
mod dxf_units;
#[cfg(feature = "dxf")]
mod dxf_units_once;
mod exif;
#[cfg(feature = "field-book")]
mod field_csv;
#[cfg(feature = "field-book")]
mod field_gsi;
#[cfg(feature = "field-book")]
mod field_gts7;
#[cfg(feature = "field-book")]
mod field_jobxml;
#[cfg(feature = "field-book")]
mod field_nikon;
#[cfg(feature = "field-book")]
mod field_sdr;
#[cfg(feature = "field-book")]
mod field_sniff;
#[cfg(feature = "field-book")]
mod field_write;
#[cfg(feature = "gnss")]
mod gnss;
#[cfg(feature = "dxf")]
mod import_decisions;
#[cfg(feature = "kml")]
mod kml;
#[cfg(all(feature = "geojson", feature = "gnss", feature = "kml"))]
mod kml_timing;
mod lin;
#[cfg(feature = "multidim")]
mod multidim;
#[cfg(feature = "parquet")]
mod parquet;
mod pat;
#[cfg(feature = "pdf")]
mod pdf;
#[cfg(feature = "raster")]
mod raster;
#[cfg(feature = "raster")]
mod raster_codecs;
#[cfg(feature = "raster")]
mod raster_gcp;
#[cfg(feature = "raster")]
mod raster_georef;
#[cfg(feature = "raster")]
mod raster_mosaic;
#[cfg(feature = "raster")]
mod raster_pyramid;
#[cfg(feature = "raster")]
mod raster_timing;
#[cfg(feature = "spreadsheet")]
mod table_file;
#[cfg(feature = "dxf")]
mod table_style_dxf;
#[cfg(all(feature = "geojson", feature = "gnss"))]
mod wgs84_write;
#[cfg(feature = "spreadsheet")]
mod workbook;
#[cfg(feature = "spreadsheet")]
mod workbook_timing;
