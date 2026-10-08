//! Point clouds of KentOS (docs/adr/0207): LAS 1.0–1.4, LAZ, COPC and text
//! clouds (XYZ, PTS, TXT, CSV), read in pieces as the view and the
//! operations need them; one implementation for the desktop (natively) and
//! the web (in its point cloud worker).
//!
//! Nothing here does input or output (CLAUDE.md §14): a reader says which
//! bytes of its source it needs ([`Need`]) and the host reads them (the
//! desktop from a file or a URL's ranges, the web from the file's slices,
//! the origin private file system or `fetch` with `Range`); the index
//! builder and the writers hand out the bytes to write and where. A source
//! never panics a reader: every fault is a [`PcError`] that says what is
//! wrong, in the window's words.
//!
//! - [`las`]: the header, VLRs and EVLRs; [`crs`] the system they name;
//! - [`record`]: point record layouts 0–10, their fields, and the move of
//!   any record to formats 6, 7 and 8 (LAS 1.4 R15);
//! - [`chunks`]: LAZ chunks (the `laz` crate's compressors) and chunk tables;
//! - [`copc`]: COPC's info VLR, keys and hierarchy pages;
//! - [`text`]: text clouds;
//! - [`source`]: a cloud opened from its bytes, its nodes and its records;
//! - [`index`]: the COPC index built once for a cloud that has none;
//! - [`look`]: points' colours for a style; [`classes`] ASPRS's classes;
//! - [`place`]: the rule of the systems and a new cloud's sample;
//! - [`nodes`]: a node's points made ready to draw;
//! - [`write`]: LAS and LAZ writers; [`vpc`] virtual point clouds;
//! - [`ops`]: queries, thinning, the ground filter, classing by height, clip,
//!   merge, tiles, rasterizing and the boundary.
#![forbid(unsafe_code)]
// A panic traps the WASM module; user data must never cause one (docs/adr/0008).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
// `!(r > 0.0)` refuses NaN as well as r ≤ 0; that is the point of writing it so.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod bytes;
pub mod chunks;
pub mod classes;
pub mod copc;
pub mod crs;
pub mod index;
pub mod las;
pub mod look;
pub mod nodes;
pub mod ops;
pub mod place;
pub mod record;
pub mod source;
pub mod text;
pub mod vpc;
pub mod write;

pub use kentos_formats::raster::{ByteStore, Need, Step};

/// Why a cloud is not read or written, in the window's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcError(pub String);

impl PcError {
    pub fn new(message: impl Into<String>) -> PcError {
        PcError(message.into())
    }
}

impl std::fmt::Display for PcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<laz::LasZipError> for PcError {
    fn from(e: laz::LasZipError) -> PcError {
        PcError(format!("LAZ çözülemedi: {e}."))
    }
}

impl From<std::io::Error> for PcError {
    fn from(e: std::io::Error) -> PcError {
        PcError(format!("Nokta verisi okunamadı: {e}."))
    }
}

pub type Result<T> = std::result::Result<T, PcError>;
