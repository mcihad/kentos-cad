//! The numeric policy of cadastral values (CLAUDE.md §23): values cross as
//! decimal text or exact fractions, never as floats, and rounding is a
//! versioned rule, not a library default.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Exact decimal as text ("748.5151"): no float, no exponent.
// A newtype: serde writes it as the bare string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DecimalString(pub String);

/// A share as an exact fraction; both parts are integers as decimal text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShareValue {
    pub numerator: String,
    pub denominator: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RoundingMode {
    HalfEven,
    HalfAwayFromZero,
    HalfTowardZero,
    AwayFromZero,
    TowardZero,
    TowardPositive,
    TowardNegative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RemainderRule {
    Reject,
    LargestRemainder,
}

/// Only an approved policy may produce a final (legal) value; a draft one
/// may be previewed but not committed (§23.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PolicyStatus {
    Draft,
    Approved,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NumericPolicy {
    pub id: String,
    pub version: u32,
    pub status: PolicyStatus,
    /// What is rounded, e.g. "parcel_area".
    pub quantity: String,
    /// Unit of the value, e.g. "m2".
    pub unit: String,
    /// Places after the decimal point in the final value.
    pub scale: u32,
    pub rounding: RoundingMode,
    pub remainder: RemainderRule,
    /// The official or institutional rule this policy implements, once verified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
}
