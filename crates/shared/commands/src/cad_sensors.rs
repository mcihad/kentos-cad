//! Sensor observations on a temporal layer (docs/adr/0646 §3–§5; plan/gis.md `CBS-33`).
//!
//! A sensor layer is a temporal layer (ADR 0210, 0537) whose objects are records without a geometry (ADR 0410):
//! - the layer time's `key` is the asset (a sensor, a meter);
//! - its `start` the observation's moment (`phenomenonTime`);
//! - its `recordedStart` the moment it was received, its `recordedEnd` the moment a later one took its place.
//!
//! `cad.sensors.ingest` writes observations given here, in a SensorThings Observations file, or read from a
//! SensorThings address page by page; duplicates (an asset's observation at a moment it has) and late ones (earlier
//! than the asset's latest) by the policies asked. `cad.sensors.latest` says each asset's latest observation at a
//! moment and its age, so that a stopped stream's last value is not shown as current.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_SENSORS_INGEST: &str = "cad.sensors.ingest";
pub const CAD_SENSORS_INGEST_VERSION: u32 = 1;
pub const CAD_SENSORS_LATEST: &str = "cad.sensors.latest";
pub const CAD_SENSORS_LATEST_VERSION: u32 = 1;

/// The most observations one call takes.
pub const MAX_OBSERVATIONS: usize = 100_000;
/// The most SensorThings pages one call reads, and the pages read when none is asked.
pub const MAX_SENSOR_PAGES: u32 = 100;
pub const SENSOR_PAGES: u32 = 10;

/// One observation: its asset, its moment, its value and quality (a number, a text, a boolean, an object; written
/// as text, ADR 0646 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Observation {
    pub asset: String,
    /// The observation's moment: a date, or a date and a time with an optional zone.
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub quality: Option<serde_json::Value>,
}

/// What a duplicate observation (an asset's at a moment it has) does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DuplicatePolicy {
    /// The one there stays; the new one is skipped.
    #[default]
    First,
    /// The new one takes its place: a layer that records versions ends the old one at the receipt, one that does
    /// not changes the old one's value and quality in place.
    Last,
    /// Both are kept.
    All,
}

/// What a late observation (earlier than the asset's latest) does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LatePolicy {
    /// Written and counted (marked in `lateField` when given).
    #[default]
    Accept,
    /// Not written: said among the rows refused.
    Reject,
}

/// Input of `cad.sensors.ingest` v1. Exactly one source: `observations`, `file` or `url`. Refusals, in order:
/// `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `layer_locked`, `not_temporal`,
/// `no_asset_field`, `invalid_input` (the value field, the asset, the pages), `invalid_source`, `invalid_moment`
/// (the receipt), `invalid_file_path`, `file_not_found`, `import_unreadable`, `source_unreachable`, `invalid_count`.
/// A row it cannot write is said in `rejected`, the others written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SensorsIngest {
    /// The sensor layer's id: a temporal layer whose time names the asset field (`key`).
    pub layer: String,
    /// The field each observation's value is written to.
    pub value_field: String,
    /// The field each observation's quality is written to; none: the quality is not written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quality_field: Option<String>,
    /// The observations given here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub observations: Option<Vec<Observation>>,
    /// A SensorThings Observations answer (`{"value": [...]}` or the list): its absolute path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// A SensorThings Observations address; `@iot.nextLink` followed up to `pages` pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub url: Option<String>,
    /// The asset a file's or an address's observations are of (a Datastream's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// The most pages an address is read for, 1 to 100; none: 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pages: Option<u32>,
    /// The moment the observations were received, written to the layer's recorded start; none: now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub received_at: Option<String>,
    #[serde(default)]
    pub duplicates: DuplicatePolicy,
    #[serde(default)]
    pub late: LatePolicy,
    /// The field a late observation written gets `evet` in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub late_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A row not written: its place among the observations read, its asset and moment as given, and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SensorRejected {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    pub asset: String,
    pub at: String,
    pub code: String,
    pub message: String,
}

/// Output of `cad.sensors.ingest` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SensorsIngested {
    /// The records written, by persistent id, in the observations' order.
    pub added: Vec<String>,
    /// The records a duplicate took the place of (ended, or changed in place).
    pub replaced: Vec<String>,
    /// Duplicates not written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    /// Late observations written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub late: u32,
    pub rejected: Vec<SensorRejected>,
    /// The pages read (a file one, given observations none).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pages: u32,
    /// Whether the source names a page not read.
    pub more: bool,
    /// The receipt written, in UTC.
    pub received_at: String,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.sensors.ingest` would write: as its output, the records to add counted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SensorsIngestPlan {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub adds: u32,
    pub replaced: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub late: u32,
    pub rejected: Vec<SensorRejected>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pages: u32,
    pub more: bool,
    pub received_at: String,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.sensors.latest` v1. Refusals: `layer_not_found`, `not_a_layer`, `not_temporal`, `no_asset_field`,
/// `invalid_input` (the value field, a negative `staleAfter`), `invalid_moment`, `not_recorded`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SensorsLatest {
    pub layer: String,
    pub value_field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quality_field: Option<String>,
    /// The moment asked; none: now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    /// The day known: only the versions recorded by then and not yet superseded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub known_at: Option<String>,
    /// An observation older than this many seconds is stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub stale_after: Option<f64>,
    /// The assets expected (`expectedAssets`): those without an observation are said in `missing`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub expected_assets: Vec<String>,
}

/// An asset's latest observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SensorLatest {
    pub asset: String,
    pub uid: String,
    /// Its moment, in UTC.
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quality: Option<String>,
    /// The moment asked less its moment.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "s")))]
    pub age_seconds: f64,
    /// Older than `staleAfter`.
    pub stale: bool,
}

/// Output of `cad.sensors.latest` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SensorsLatestAnswer {
    pub layer: String,
    /// The moment asked, in UTC.
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub known_at: Option<String>,
    /// Each asset's latest, in the assets' natural order.
    pub assets: Vec<SensorLatest>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stale: u32,
    /// The expected assets without an observation.
    pub missing: Vec<String>,
    /// Objects without an asset or a readable moment.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unreadable: u32,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
