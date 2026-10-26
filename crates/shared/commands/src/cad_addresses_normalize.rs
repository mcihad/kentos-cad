//! `cad.addresses.normalize` (docs/adr/0647 §2): the doors of an address data's groups that write a street's, a
//! neighbourhood's or a district's name in more than one way given one spelling: the first of `prefer` the group
//! writes, else its most written. One undo step (“Adres yazılışlarını düzelt”).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_ADDRESSES_NORMALIZE: &str = "cad.addresses.normalize";
pub const CAD_ADDRESSES_NORMALIZE_VERSION: u32 = 1;

/// Which name of the doors is set right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AddressName {
    /// The street's name, in each street (its doors grouped by street, neighbourhood and district).
    #[default]
    Street,
    /// The neighbourhood's name, in each neighbourhood of a district.
    Neighbourhood,
    /// The district's name.
    District,
}

/// Input of `cad.addresses.normalize` v1. Refusals, in order: `invalid_revision`, `revision_conflict`,
/// `no_address_data`, `unknown_locator`, `invalid_input` (the doors have no field of that name, an empty preferred
/// spelling), `layer_locked`, a field's rule the spelling does not fit (`invalid_attribute`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesNormalize {
    /// The address data's id; none: the project's first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locator: Option<String>,
    #[serde(default)]
    pub what: AddressName,
    /// Spellings preferred: a group that writes one of them takes the first it writes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub prefer: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// One group set right: the spelling chosen, the ones it took the place of, the doors written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressNormalized {
    pub to: String,
    pub from: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub doors: u32,
}

/// Output of `cad.addresses.normalize` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AddressesNormalized {
    pub locator: String,
    pub what: AddressName,
    /// The groups, by their first doors.
    pub groups: Vec<AddressNormalized>,
    /// The doors written (or to be written).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub written: u32,
    /// Their persistent ids, in the drawing's order.
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
