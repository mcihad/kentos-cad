//! KentOS Yol's pay item quantities (plan/road.md Y11.4, docs/adr/0909 §4; ROAD-DRAIN-11): `road.pay.edit` writes or
//! removes the project's pay item book (its source and version, required, its currency, the agency's items and the
//! rules that give them KentOS's quantities); `road.pay.query` reads every item's quantity now: each rule's measure of
//! every guardrail, culvert, wall or earthwork volume it reads, times its factor, the sources it came from and whether
//! they are stale, and with a unit price its amount. KentOS gives no item and no price. The desktop's handlers are
//! `crates/native/application/src/road_pay*.rs`; they and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.pay.*.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{PayBookDef, PayItemDef, PayMeasure, PayRuleDef, RoadEditOperation};

pub const ROAD_PAY_EDIT: &str = "road.pay.edit";
pub const ROAD_PAY_EDIT_VERSION: u32 = 1;
pub const ROAD_PAY_QUERY: &str = "road.pay.query";
pub const ROAD_PAY_QUERY_VERSION: u32 = 1;

/// Input of `road.pay.edit` v1: `set` writes the book (a new one needs `source`, `version` and `items`; the given
/// fields replace the book's), `remove` takes it off. `currency` null takes it off.
///
/// Refusals, in this order: `not_a_road_project`, `no_pay_book` (remove with none), `invalid_pay_book` (rebind, a new
/// one without its fields, set with nothing to set, the book by `road_problem`: no source or version, a code twice, a
/// rule's item not in the book, a factor out of bounds, a material on a measure without one), `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayEdit {
    pub operation: RoadEditOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<String>,
    /// The book's currency; null takes it off.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub currency: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub items: Option<Vec<PayItemDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<Vec<PayRuleDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `road.pay.edit` v1: the book written (absent when removed).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayWritten {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub book: Option<PayBookDef>,
    pub revision: String,
}

/// Input of `road.pay.query` v1: every item, or one (`item`: its code).
///
/// Refusals: `not_a_road_project`, `no_pay_book`, `invalid_pay_book` (an item not in the book). Warnings:
/// `pay_source_stale`, `pay_source_unread`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<String>,
}

/// A quantity's source: the structure or volume (`kind`: `guardrail`, `culvert`, `wall` or `volume`), the measure read,
/// its value, the rule's factor, the quantity (value × factor) and whether its corridor changed since.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayLine {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub measure: PayMeasure,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub value: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub quantity: f64,
    pub stale: bool,
}

/// An item's quantity now: its code, name and unit, the sum of its lines, its unit price and amount when priced.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayRow {
    pub code: String,
    pub name: String,
    pub unit: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub quantity: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub amount: Option<f64>,
    pub lines: Vec<PayLine>,
}

/// Output of `road.pay.query` v1: the book's source, version, revision and currency, its items' rows (book order),
/// the priced items' total and how many items with a quantity have no price.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PayQueried {
    pub source: String,
    pub version: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub currency: Option<String>,
    pub rows: Vec<PayRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "money")))]
    pub total: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unpriced: u32,
}
