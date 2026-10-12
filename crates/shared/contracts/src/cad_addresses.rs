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
