//! Netcad NCZ drawings (docs/adr/0138): a file's bytes into the app's
//! objects (`ImportResult`, the contract every import gives), with a report
//! of what was read, converted or left out, like every reader of the formats
//! crate (CLAUDE.md §9.7). The desktop links it; the browser runs it in a
//! module of its own (`crates/wasm/ncz-wasm`), loaded only when an NCZ is
//! imported (§20).
//!
//! The block reader is a port of Erdinç Örsan ÜNAL's NCZ Reader (QGIS
//! plugin, `ncz_pure.py` 1.4.3, GPL-2.0-or-later); README.md says what is
//! his and what is not, and the condition on distributing this crate.
//!
//! Rules, the formats crate's:
//! - coordinates are the float64 the file holds, never rounded, rescaled or
//!   reprojected (§5, §23); what the file says of its system is reported as
//!   `declaredCrs` and asked about, never applied. A map sheet's frame is the
//!   one computed shape: the file keeps only the box of its cell, and the
//!   cell's corners are worked out in the zone the file names (`sheet`);
//! - nothing a file holds may crash the reader: records that do not read are
//!   counted and reported in Turkish;
//! - one pass per table and record walk, no quadratic behaviour.
#![forbid(unsafe_code)]
// A panic traps the WASM module; user data must never cause one (docs/adr/0008).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
// `!(r > 0.0)` refuses NaN as well as r ≤ 0; that is the point of writing it so.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod attributes;
mod crs;
mod emit;
pub mod format;
mod sheet;
mod symbols;

pub use emit::read;

/// What a read's progress counts to (`Watch::step`'s `total`): thousandths.
pub const PROGRESS_TOTAL: u64 = 1000;
