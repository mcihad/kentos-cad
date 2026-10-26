//! Stable room identities with an explicit sequential numbering order (ADR 0653).
use crate::Vec2;
#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SPACES_NUMBER: &str = "cad.spaces.number";
pub const CAD_SPACES_NUMBER_VERSION: u32 = 1;

/// The same first-number rule for the command and its visible text field.
pub fn valid_space_number(value: &str) -> bool {
    value.trim() == value
        && value.chars().count() <= 256
        && value.as_bytes().last().is_some_and(u8::is_ascii_digit)
        && !value.chars().any(char::is_control)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SpaceNumberOrder {
    /// The UUID list is the click order; the command never sorts it.
    #[default]
    Input,
    /// Ascending projection of each room's label anchor onto from→to;
    /// equal projections use the left-normal projection, then UUID.
    Direction {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        from: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        to: Vec2,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpacesNumber {
    /// 1–10000 distinct room UUIDs. No implicit layer/level expansion.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// Complete first number, with an ASCII decimal suffix: Z-001 → Z-002.
    /// Up to 256 characters, no surrounding whitespace. Digits never become f64.
    pub first: String,
    #[serde(default)]
    pub order: SpaceNumberOrder,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
