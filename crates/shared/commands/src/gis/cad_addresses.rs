//! The address data's command of the product command catalog (docs/adr/0229
//! §14): `cad.addresses.define` writes an address locator into the
//! project's settings, or takes one away. Adres verisi writes through it on
//! the desktop (`crates/native/application`); the shared cases are in
//! `fixtures/commands/v1`. A locator is a project setting, as a network is:
//! writing it makes the drawing dirty and is no undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::address::AddressLocator;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Writes or removes an address locator.
pub const CAD_ADDRESSES_DEFINE: &str = "cad.addresses.define";
pub const CAD_ADDRESSES_DEFINE_VERSION: u32 = 1;

/// What `cad.addresses.define` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AddressesDefineOperation {
    /// The locator replaces the project's of its id, or goes last.
    Set,
    /// The locator of `id` goes.
    Remove,
}

/// Input of `cad.addresses.define` v1.
///
/// - `set`: `locator`, which replaces the project's locator of its id or
///   goes last;
/// - `remove`: `id`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_locator` (set
/// without `locator`), `no_id` (remove without `id`), `invalid_locator` (the
/// locator, or the project's locators with it, by
/// `address_locators_problem`); then `invalid_revision`,
/// `revision_conflict` (status `conflict`), `unknown_locator` (remove: the
/// project has no locator of the id). A locator naming a layer the drawing
/// does not have is written, with the warning `unknown_layer`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesDefine {
    pub operation: AddressesDefineOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locator: Option<AddressLocator>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.addresses.define` v1, and its plan: the project's locators
/// as the write leaves them and the revision after it (a plan: before it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesDefined {
    pub locators: Vec<AddressLocator>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Checks a project's address data (docs/adr/0579): read only.
pub const CAD_ADDRESSES_CHECK: &str = "cad.addresses.check";
pub const CAD_ADDRESSES_CHECK_VERSION: u32 = 1;

/// The most of each list `cad.addresses.check` gives unless asked for fewer.
pub const ADDRESS_CHECK_LIMIT: u32 = 100;

/// Input of `cad.addresses.check` v1: the address data of `locator` (else the project's first) checked; each list cut
/// at `limit` (1–100, default 100).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_limit`, `no_address_data` (the project defines
/// none), `unknown_locator`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
}

/// A door number one street has more than once.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressRepeat {
    pub street: String,
    pub neighbourhood: String,
    pub door: String,
    /// The doors' objects, in the drawing's order.
    pub uids: Vec<String>,
}

/// An address code on more than one door.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressCodeRepeat {
    pub code: String,
    pub uids: Vec<String>,
}

/// One way a street's name is written and how many of its doors write it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressWritten {
    pub text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub doors: u32,
}

/// A street whose doors write its name in more than one way: the ways, the most written first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressSpelling {
    pub neighbourhood: String,
    pub district: String,
    pub written: Vec<AddressWritten>,
    pub uids: Vec<String>,
}

/// A neighbourhood or a district whose doors write its name in more than one way (docs/adr/0647 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressNameSpelling {
    /// The district its first door writes (a neighbourhood's); empty for a district.
    pub within: String,
    pub written: Vec<AddressWritten>,
    pub uids: Vec<String>,
}

/// Output of `cad.addresses.check` v1, and its plan: what the address data holds, what it left out, and what in it
/// repeats or is written in more than one way (each list's count, then its first `limit`, by their first door).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesChecked {
    pub locator: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub doors: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub streets: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub neighbourhoods: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub areas: u32,
    /// Objects of the door layer that are no points.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub not_doors: u32,
    /// Doors with no street name.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub empty_streets: u32,
    /// Doors with no door number read.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unread_doors: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub not_streets: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unnamed_lines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub not_areas: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unnamed_areas: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub repeated: u32,
    pub repeats: Vec<AddressRepeat>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub coded: u32,
    pub codes: Vec<AddressCodeRepeat>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub spelled: u32,
    pub spellings: Vec<AddressSpelling>,
    /// Neighbourhoods written in more than one way (docs/adr/0647 §1).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub neighbourhoods_spelled: u32,
    pub neighbourhood_spellings: Vec<AddressNameSpelling>,
    /// Districts written in more than one way.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub districts_spelled: u32,
    pub district_spellings: Vec<AddressNameSpelling>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
