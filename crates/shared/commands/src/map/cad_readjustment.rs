//! `cad.readjustment.distribute` v1 (plan/map-cad.md MAP-11, docs/adr/1460): Düzenleme dağıtım taslağı, read only. A
//! land readjustment's first numbers as a reproducible draft, never as a decision: each parcel's registered area (as
//! written, decimal text), the deduction the rule package gives (its ratio is the package's data, with its
//! jurisdiction, source document, effective date and release; no ratio is a constant in code), the net area, and
//! each holder's entitlement from their exact share (1/3 stays 1/3). Every rounding is the package's numeric policy
//! (places, mode, remainder rule); the totals are kept exactly (registered = deduction + net; Σ entitlements = net)
//! and the answer says where each remainder went. Numbers cross as decimal text, never as floats (CLAUDE.md §23).
//! docs/adr/5080: by value (`basis: value`), a user-chosen alternative the regulation does not have: the distribution
//! mass (registered − deduction) goes to the parcels in proportion to their values before the readjustment, each value
//! the user's with its source; the answer says so in a warning.
//! The shared cases are `fixtures/commands/v1/cad.readjustment.distribute.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_READJUSTMENT_DISTRIBUTE: &str = "cad.readjustment.distribute";
pub const CAD_READJUSTMENT_DISTRIBUTE_VERSION: u32 = 1;

/// A rule package: who issues it, the document it rests on, when it holds and its release, whether it is approved,
/// the deduction ratio (decimal text in [0, 1)), and the numeric policy: places, rounding mode (`half_even`,
/// `half_away_from_zero`, …) and remainder rule (`reject`, `largest_remainder`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentRules {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub release: u32,
    pub jurisdiction: String,
    pub source: String,
    pub effective: String,
    pub approved: bool,
    pub deduction: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: u32,
    pub rounding: String,
    pub remainder: String,
    /// docs/adr/5080: what the distribution mass is shared by: `area` (none: the regulation's, by the parcels' areas) or
    /// `value` (the user's alternative: by their values before the readjustment, `entries[i].value`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub basis: Option<String>,
    /// docs/adr/5080, with `basis: value`: who determined the values and by what decision (KentOS makes none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value_source: Option<String>,
}

/// A record: the parcel, the holder (an id, not a name: personal data stays in its own access-controlled record), the
/// parcel's registered area (decimal text, square metres; the same on every record of the parcel) and the holder's
/// share (`n/d`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentEntry {
    pub parcel: String,
    pub holder: String,
    pub area: String,
    pub share: String,
    /// docs/adr/5080, with `basis: value` only: the parcel's value before the readjustment (plain decimal text, not
    /// below zero, the same on every record of the parcel; the user's, e.g. a valuation's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<String>,
}

/// Input of `cad.readjustment.distribute` v1: the rule package and the records, in order.
///
/// Refusals, in this order: `invalid_input` (`rules.*`: an empty id, source or jurisdiction, a deduction off [0, 1),
/// places over 6, an unknown mode or rule; `entries`: none), then record by record `invalid_input`
/// (`entries[i].area` not plain decimal text or finer than the policy's places, `entries[i].share` not `n/d` with
/// d > 0, `entries[i].holder` or `.parcel` empty), `area_mismatch` (`entries[i].area`: a parcel's records disagree),
/// `duplicate_holder` (`entries[i].holder`: twice in one parcel), `shares_not_whole` (`entries[i].share`: a parcel's
/// shares do not add up to exactly 1; the first record of the parcel), then `needs_rule` (a remainder and the rule is
/// `reject`). Docs/adr/5080: `invalid_input` at `rules.basis` (neither `area` nor `value`), `rules.valueSource` (empty
/// by value), `entries[i].value` (given by area; by value missing, not plain decimal text, below zero, or another on the
/// parcel's records), `entries` (by value, the values add up to zero).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentDistribute {
    pub rules: ReadjustmentRules,
    pub entries: Vec<ReadjustmentEntry>,
}

/// A holder's entitlement in a parcel: the exact share, the exact value (net × share, a fraction `n/d` of square
/// metres) and the value at the policy's places after the remainder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentPart {
    pub holder: String,
    pub share: String,
    pub exact: String,
    pub value: String,
}

/// A parcel: its registered area, its deduction at the policy's places, its net area, the remainder its holders'
/// split had and the holder it went to (none when there was none), and its holders' parts in the records' order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentParcel {
    pub parcel: String,
    pub area: String,
    pub deduction: String,
    pub net: String,
    pub remainder: String,
    pub remainder_to: Vec<String>,
    pub parts: Vec<ReadjustmentPart>,
    /// docs/adr/5080, by value: the parcel's value as given and its exact share of the values (`n/d`); its deduction is
    /// then its area − its net, below zero when it gains.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value_share: Option<String>,
}

/// A holder's total over the parcels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentHolder {
    pub holder: String,
    pub value: String,
}

/// Output of `cad.readjustment.distribute` v1: the package's id and release, whether the result is a draft (always
/// here: no official decision is enabled), the totals (registered, the deduction's exact and rounded value, its
/// remainder and the parcels it went to, net, the holders' sum) and whether they are kept exactly, the parcels in the
/// records' order and the holders' totals in their first appearance's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentDistributed {
    pub rules: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub release: u32,
    pub draft: bool,
    pub registered: String,
    pub deduction_exact: String,
    pub deduction: String,
    pub deduction_remainder: String,
    pub deduction_remainder_to: Vec<String>,
    pub net: String,
    pub held: String,
    pub conserved: bool,
    pub parcels: Vec<ReadjustmentParcel>,
    pub holders: Vec<ReadjustmentHolder>,
    /// docs/adr/5080, by value: `value`, the values' source as given, and the remainder of the mass's split over the
    /// parcels and the parcels it went to (the deduction is then not split: its remainder is 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub basis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mass_remainder: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub mass_remainder_to: Vec<String>,
}
