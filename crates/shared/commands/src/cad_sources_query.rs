//! `cad.sources.query` (docs/adr/0581): a question asked of a data set where it is: a GeoPackage table, a PostGIS table, a
//! bound layer's whole source or a layer of the drawing. Its box, columns and the part of its filter a source can check go
//! to the source (§3); the rows come a page at a time in key order; a count and the fields' statistics are of every row
//! the filter keeps, not of the page; the plan says what the source did and what was done here. Read only. The shared
//! cases are `fixtures/commands/v1/cad.sources.query.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SOURCES_QUERY: &str = "cad.sources.query";
pub const CAD_SOURCES_QUERY_VERSION: u32 = 1;

/// The most rows a page holds, and how many when none is said.
pub const MOST_QUERIED: u32 = 10_000;
pub const QUERIED: u32 = 100;

/// What a layer's question is asked of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum QueryScope {
    /// The drawing's objects of the layer: a bound layer's working copy (of a part, a part only).
    #[default]
    Loaded,
    /// A bound layer's source, the whole table.
    Source,
}

/// Records near a place (what a click identifies).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QueryNear {
    /// The place, in the project's system: east, north.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    /// How far from it a record's geometry may be.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
}

/// Input of `cad.sources.query` v1: `layer`, or `path` and `table`.
///
/// Refusals, checked in this order: `invalid_input` (neither or both of `layer` and `path`, `table` without `path`,
/// `path` without `table`, `scope` without `layer`), `invalid_count` (`limit` past 10 000), `invalid_extent` (a box that
/// is reversed or not finite), `invalid_tolerance`, `invalid_cursor` (not a cursor of this question), `invalid_expression`
/// (the filter does not compile, reads `$sıra` or `$ölçek`, or looks at another layer), `layer_not_found`, `invalid_input`
/// (`scope: source` on a layer that is not bound), `file_not_found`, `import_unreadable`, `table_not_found`,
/// `field_not_found` (in `fields` or `statistics`), `source_unreachable`. A job stopped answers `cancelled`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesQuery {
    /// A layer of the drawing (its id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// With `layer`: its working copy (the default) or its source's whole table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<QueryScope>", optional))]
    pub scope: Option<QueryScope>,
    /// A GeoPackage file's path, or a database's address (`postgresql://…`, no password).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// With `path`: the table (`schema.table` in a database).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    /// Records whose box meets this one, in the project's system: west, south, east, north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bbox: Option<[f64; 4]>,
    /// Records near a place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<QueryNear>", optional))]
    pub near: Option<QueryNear>,
    /// A condition in İfadeyle seç's language: the records it holds for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<String>,
    /// The fields returned, by name; none: all of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<Vec<String>>,
    /// Return each record's geometry in the project's system (ISO WKT).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<bool>,
    /// How many records a page holds, 0–10 000; none: 100. 0 returns none (for a count or statistics).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    /// The page after the one whose `next` this is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cursor: Option<String>,
    /// Count every record the question keeps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub count: Option<bool>,
    /// Fields whose statistics are of every record the question keeps (`kentos.statistics/1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub statistics: Option<Vec<String>>,
}

/// What a question was asked of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum QueriedKind {
    /// The drawing's objects.
    Drawing,
    /// A GeoPackage file.
    Geopackage,
    /// A PostGIS database.
    Postgis,
}

/// The source a question read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QueriedSource {
    pub kind: QueriedKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The file's path or the database's address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    /// The source's system (EPSG), when it is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
}

/// A record a question returned.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QueriedRow {
    /// Its lasting id: the same in every window and in a bound layer's working copy.
    pub uid: String,
    /// Its key in the source (fid, primary key); none for a drawing's object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub key: Option<i64>,
    /// The fields asked for, as the drawing's attributes read them (an empty value left out).
    pub attrs: BTreeMap<String, String>,
    /// Its geometry in the project's system, ISO WKT (with `geometry`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wkt: Option<String>,
}

/// A field's statistics over every record a question keeps: exact decimals as text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QueriedStatistic {
    pub field: String,
    /// Values read as numbers.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    /// Values given but not numbers.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mean: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<String>,
    /// The sample standard deviation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub std: Option<String>,
    /// Why the sums are not given (too large to sum exactly).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
}

/// How the box was applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PlanBox {
    /// By the source's spatial index.
    Index,
    /// By each geometry's box, in the source (no index).
    Geometry,
    /// Here, over the drawing's objects.
    Local,
}

/// What the source did and what was done here (docs/adr/0581 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QueryPlan {
    #[serde(rename = "box", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bbox: Option<PlanBox>,
    /// The columns the source read.
    pub columns: Vec<String>,
    /// Whether the source read the geometry.
    pub geometry: bool,
    /// The condition the source checked, its values written in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pushed: Option<String>,
    /// Whether that condition is the whole filter and keeps exactly what it keeps.
    pub exact: bool,
    /// What was done here: `filter` (every record the source handed over checked with the filter), `box`, `near`,
    /// `statistics`.
    pub local: Vec<String>,
    /// The order of the records: `key` (the source's) or `uid` (the drawing's).
    pub order: String,
}

/// Output of `cad.sources.query` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesQueried {
    pub source: QueriedSource,
    pub rows: Vec<QueriedRow>,
    /// The cursor of the page after this one; none on the last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
    /// How many records the question keeps (with `count` or `statistics`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<QueriedStatistic>>", optional))]
    pub statistics: Vec<QueriedStatistic>,
    pub plan: QueryPlan,
    /// The answer is of a bound layer's part, not of the whole data set.
    pub partial: bool,
    /// How many records the source handed over.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    /// A cheap mark of a file's state (its size and modification time); none for a database or the drawing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stamp: Option<String>,
    pub revision: String,
}
