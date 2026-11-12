//! `cad.addresses.compare` (docs/adr/0736): an address data's doors compared with an address register, the
//! institution's or the national address database's table of address codes (UAVT), read only. The register is a table
//! file (CSV or text, XLSX, ODS) or a layer of the drawing; its columns are named. What the doors lack, write otherwise
//! or stand away from, and what the register has that no door has, each list cut at `limit`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ADDRESSES_COMPARE: &str = "cad.addresses.compare";
pub const CAD_ADDRESSES_COMPARE_VERSION: u32 = 1;

/// The most of each list `cad.addresses.compare` gives unless asked for another.
pub const ADDRESS_COMPARE_LIMIT: u32 = 100;
/// The most of each list it gives when asked: a window writes every difference it lists.
pub const ADDRESS_COMPARE_MAX_LIMIT: u32 = 10_000;
/// The distance a door may stand from its row's position, metres, unless another is given.
pub const ADDRESS_COMPARE_TOLERANCE: f64 = 10.0;
/// The largest tolerance, metres.
pub const ADDRESS_COMPARE_MAX_TOLERANCE: f64 = 10_000.0;

/// Where the register is and which of its columns say what: a table file (`path`, a workbook's `sheet`) or a layer of
/// the drawing (`layer`, its attributes the columns), exactly one of them. The code's column is needed; the others are
/// compared where they are named (a neighbourhood or district only when the address data's doors have that field too).
/// `x` and `y` are the position in the project's coordinates; a layer's point is its position when they are not named.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressRegister {
    /// A CSV or text table, an XLSX or an ODS file: absolute, or from the drawing's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// A workbook's sheet by its name; none: its first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheet: Option<String>,
    /// A layer of the drawing by its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The column of the address code.
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub street: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub door: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub neighbourhood: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub district: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub x: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub y: Option<String>,
}

/// Input of `cad.addresses.compare` v1: the address data of `locator` (else the project's first) compared with
/// `register`; a door `tolerance` metres or more from its row's position is said (default 10, at most 10 000); each
/// list cut at `limit` (1–10 000, default 100). `drawing` is the drawing's own file, for a register path given from its
/// folder.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_limit`, `invalid_tolerance`, `register_missing`
/// (no path and no layer, or both), `no_address_data`, `unknown_locator`, `no_address_codes` (the address data's doors
/// have no code field), `invalid_file_path`, `file_not_found`, `register_unreadable` (the file does not read as a table,
/// has no rows, or no sheet of that name), `layer_not_found`, `register_column_missing`. Warning: `register_cut` (the
/// register had more rows than are read: 250 000).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesCompare {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locator: Option<String>,
    pub register: AddressRegister,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub tolerance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A part of an address the register writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AddressPart {
    Street,
    Door,
    Neighbourhood,
    District,
}

/// A door whose code no row of the register has.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressUnregistered {
    pub uid: String,
    pub code: String,
    pub street: String,
    pub door: String,
}

/// A part a door writes otherwise than its row: both texts as written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressPartDifference {
    pub part: AddressPart,
    pub drawing: String,
    pub register: String,
}

/// A door that writes its address otherwise than its row (`row`: from 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressDifference {
    pub uid: String,
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub row: u32,
    pub parts: Vec<AddressPartDifference>,
}

/// A door farther from its row's position than the tolerance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressDistance {
    pub uid: String,
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub row: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub distance: f64,
}

/// An address of the register no door has: its first row's texts, its codes and rows (and, for a layer, the rows'
/// objects).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressUndrawn {
    pub street: String,
    pub door: String,
    pub neighbourhood: String,
    pub codes: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub rows: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
}

/// A code on more than one row of the register (and, for a layer, the rows' objects).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressRegisterRepeat {
    pub code: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub rows: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
}

/// Output (and plan) of `cad.addresses.compare` v1: the counts, then each list's count and its first `limit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesCompared {
    pub locator: String,
    /// The address data's doors, those with a code, and those whose code the register has.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub doors: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub coded: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub matched: u32,
    /// The register's rows read with a code, and those without one (left out).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub uncoded_rows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unregistered: u32,
    pub unregistered_doors: Vec<AddressUnregistered>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub differing: u32,
    pub differences: Vec<AddressDifference>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub distant: u32,
    pub distances: Vec<AddressDistance>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub undrawn: u32,
    pub undrawn_addresses: Vec<AddressUndrawn>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub repeated: u32,
    pub register_repeats: Vec<AddressRegisterRepeat>,
    pub revision: String,
}
