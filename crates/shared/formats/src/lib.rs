//! File formats of KentOS (CLAUDE.md §9.7): coordinate lists (Netcad NCN,
//! TXT, CSV), ASCII DXF, GeoJSON and Shapefile (docs/adr/0046). Readers turn a file's bytes into the
//! app's objects (the versioned contracts, `kentos-contracts`) with a report
//! of what was read, converted or left out; writers do the reverse. The
//! browser runs this crate in a Web Worker (`kentos-formats-wasm`), the
//! server's import job will run it natively: one implementation (§14).
//!
//! Rules:
//! - coordinates are the float64 nearest to what the file wrote, never
//!   rounded, rescaled or reprojected (§5, §23); writers write the shortest
//!   decimal that reads back to the same float64. The one rescaling is a
//!   local project's unit: a DXF's declared one in, the project's out
//!   (`units`, docs/adr/0165 §2);
//! - nothing a file holds may crash a reader: bad lines and unknown objects
//!   are counted and reported in Turkish with their line numbers;
//! - one pass over the input, no quadratic behaviour on large files.
//!
//! Each codec is a Cargo feature, all of them on by default (docs/adr/8054 §11): without them the crate is the
//! objects' own rules the viewer core draws with (blocks, walls, spaces, openings, grids) and the helpers every
//! format shares, so a phone or a viewer compiles none of the heavy readers and writers.
#![forbid(unsafe_code)]
// A panic traps the WASM module; user data must never cause one (docs/adr/0008).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
// `!(r > 0.0)` refuses NaN as well as r ≤ 0; that is the point of writing it so.
#![allow(clippy::neg_cmp_op_on_partial_ord)]
// Without every codec some shared helpers go unused; the full build still finds dead code (docs/adr/8054 §11).
#![cfg_attr(
    not(all(
        feature = "dxf",
        feature = "geojson",
        feature = "shapefile",
        feature = "kml",
        feature = "gnss",
        feature = "field-book",
        feature = "landxml",
        feature = "pdf",
        feature = "raster",
        feature = "multidim",
        feature = "parquet",
        feature = "sld",
        feature = "spreadsheet"
    )),
    allow(dead_code)
)]

pub mod arch_elements;
pub mod blocks;
#[cfg(any(feature = "parquet", feature = "raster"))]
pub mod compress;
pub mod coords;
pub mod dimension_spans;
#[cfg(feature = "dxf")]
pub mod dxf;
pub(crate) mod exact;
pub mod exif;
#[cfg(any(
    feature = "geojson",
    feature = "kml",
    feature = "gnss",
    feature = "shapefile"
))]
pub(crate) mod export_geom;
pub mod extract;
#[cfg(feature = "field-book")]
pub mod field;
pub mod fields;
#[cfg(feature = "geojson")]
pub mod geojson;
pub mod geom;
pub mod gis;
pub mod gltf;
#[cfg(feature = "gnss")]
pub mod gnss;
pub mod import;
mod import_bounds;
pub mod json;
#[cfg(feature = "kml")]
pub mod kml;
#[cfg(feature = "landxml")]
pub mod landxml;
#[cfg(feature = "landxml")]
pub mod landxml_write;
pub mod lin;
pub mod math;
#[cfg(feature = "pdf")]
pub mod pdf;
// The raster codec draws meshes and NetCDF cubes with part of it; the rest is the `multidim` feature.
#[cfg(feature = "raster")]
pub mod multidim;
pub mod num;
pub mod nurbs;
pub mod openings;
#[cfg(feature = "parquet")]
pub mod parquet;
pub mod pat;
#[cfg(feature = "raster")]
pub mod raster;
pub mod report;
pub mod reproject;
#[cfg(feature = "shapefile")]
pub mod shp;
#[cfg(feature = "sld")]
pub mod sld;
pub mod spaces;
#[cfg(feature = "sld")]
pub mod style_exchange;
#[cfg(feature = "sld")]
pub mod style_expr;
#[cfg(feature = "sld")]
pub mod style_extras;
#[cfg(feature = "sld")]
pub mod style_labels;
#[cfg(feature = "spreadsheet")]
pub mod table_file;
pub mod text;
pub mod units;
pub mod vertical;
pub mod walls;
pub mod watch;
#[cfg(any(feature = "geojson", feature = "kml", feature = "gnss"))]
pub(crate) mod wgs84;
#[cfg(any(
    feature = "dxf",
    feature = "geojson",
    feature = "kml",
    feature = "gnss"
))]
pub(crate) mod wire;
#[cfg(feature = "spreadsheet")]
pub mod workbook;
pub mod xml;
#[cfg(feature = "spreadsheet")]
pub mod xml_scan;
pub mod zip;

pub use kentos_contracts as contracts;
