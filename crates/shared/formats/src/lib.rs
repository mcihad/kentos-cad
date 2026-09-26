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
//!   decimal that reads back to the same float64;
//! - nothing a file holds may crash a reader: bad lines and unknown objects
//!   are counted and reported in Turkish with their line numbers;
//! - one pass over the input, no quadratic behaviour on large files.
#![forbid(unsafe_code)]
// A panic traps the WASM module; user data must never cause one (docs/adr/0008).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
// `!(r > 0.0)` refuses NaN as well as r ≤ 0; that is the point of writing it so.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod coords;
pub mod dxf;
pub mod geojson;
pub mod geom;
pub mod gis;
pub mod json;
pub mod math;
pub mod num;
pub mod nurbs;
pub mod report;
pub mod shp;
pub mod text;

pub use kentos_contracts as contracts;
