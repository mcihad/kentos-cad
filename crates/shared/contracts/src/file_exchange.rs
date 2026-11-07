//! Dışa aktarma kaybı denetimi (plan/map-cad.md MAP-17, docs/adr/1660): the drawing written to an exchange format and read back by
//! KentOS's own reader of that format, into an empty copy of the drawing (its settings and layers, no objects), then
//! compared object by object. What the trip changed is said in two kinds apart (the plan's acceptance: “sayısal hata ve
//! semantik kayıplar ayrı raporlansın”):
//!
//! - **numeric**: how far each vertex read back lies from the original's geometry (its lines and arcs exactly), the
//!   heights' and the measures' differences, and the length's and the area's;
//! - **semantic**: objects not written, a kind changed (a circle read back as an area), curves become chords, heights
//!   or measures lost or added, attributes lost, changed or added, an object's own style lost, its persistent id lost,
//!   objects read back that match none.
//!
//! The file is written under the system's temporary folder and removed; the open drawing is not changed. The trip is
//! KentOS's writer and reader together: what another program reads from the file is its own (the ADR's independent
//! check reads the same files with GDAL).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FILE_EXCHANGE_CHECK: &str = "file.exchange.check";
pub const FILE_EXCHANGE_CHECK_VERSION: u32 = 1;

/// The tolerance a check takes (metres): `cad.drawing.compare`'s.
pub const EXCHANGE_TOLERANCE: f64 = 0.001;
/// The most persistent ids a loss lists (its count goes on).
pub const EXCHANGE_UIDS_LISTED: usize = 100;

/// The losses' kinds, in the order they are listed.
pub const EXCHANGE_LOSSES: [&str; 12] = [
    "notWritten",
    "kindChanged",
    "chorded",
    "zLost",
    "zAdded",
    "mLost",
    "attrLost",
    "attrChanged",
    "attrAdded",
    "styleLost",
    "identityLost",
    "extra",
];

/// The format of the trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExchangeFormat {
    Gpkg,
    Parquet,
    Fgb,
    Gml,
    Shp,
    Dxf,
}

impl ExchangeFormat {
    pub const ALL: [ExchangeFormat; 6] = [Self::Gpkg, Self::Parquet, Self::Fgb, Self::Gml, Self::Shp, Self::Dxf];

    /// The file's extension.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Gpkg => "gpkg",
            Self::Parquet => "parquet",
            Self::Fgb => "fgb",
            Self::Gml => "gml",
            Self::Shp => "zip",
            Self::Dxf => "dxf",
        }
    }

    /// Its name as the windows say it.
    pub fn title(self) -> &'static str {
        match self {
            Self::Gpkg => "GeoPackage",
            Self::Parquet => "GeoParquet",
            Self::Fgb => "FlatGeobuf",
            Self::Gml => "GML",
            Self::Shp => "Shapefile",
            Self::Dxf => "DXF",
        }
    }

    /// The commands that write and read it.
    pub fn commands(self) -> (&'static str, &'static str) {
        match self {
            Self::Gpkg => (crate::FILE_EXPORT_GPKG, "file.import.gpkg"),
            Self::Parquet => (crate::FILE_EXPORT_PARQUET, "file.import.parquet"),
            Self::Fgb => (crate::FILE_EXPORT_FGB, "file.import.fgb"),
            Self::Gml => (crate::FILE_EXPORT_GML, "file.import.gml"),
            Self::Shp => (crate::FILE_EXPORT_SHP, "file.import.shp"),
            Self::Dxf => (crate::FILE_EXPORT_DXF, "file.import.dxf"),
        }
    }
}

/// Input of `file.exchange.check` v1: the format, the layers (by id; none: every layer the format's writer takes by
/// default) and the tolerance (metres, above 0, at most 10; none: [`EXCHANGE_TOLERANCE`]).
///
/// Refusals: `invalid_tolerance`, `layer_not_found` (`layers[i]`), the writer's own refusal (its code, the path
/// `export.<its path>`), `exchange_unreadable` (KentOS's reader refused the file KentOS wrote: its words).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeCheck {
    pub format: ExchangeFormat,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// A kind's count (objects not written by kind: `text`, `dimension` …).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeKindCount {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
}

/// A semantic loss: its kind (one of [`EXCHANGE_LOSSES`]), how many, what changed into what where it is one change
/// (`circle → polygon`, an attribute's name), and the original objects' persistent ids (at most 100).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeLoss {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub detail: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    pub uids: Vec<String>,
}

/// The numeric differences, the greatest over the objects matched (metres, square metres; none: nothing to measure),
/// the object of the greatest offset, and how many lie farther than the tolerance.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeNumeric {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offset_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub measure: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub above: u32,
}

/// Output of `file.exchange.check` v1: the format and tolerance, the objects of the layers checked, how many were
/// matched with one read back, those not written by kind, the numeric differences, the semantic losses (in
/// [`EXCHANGE_LOSSES`]' order, then by detail) and the writer's and the reader's warnings as they said them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeChecked {
    pub format: ExchangeFormat,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub matched: u32,
    pub not_written: Vec<ExchangeKindCount>,
    pub numeric: ExchangeNumeric,
    pub losses: Vec<ExchangeLoss>,
    pub writer: Vec<String>,
    pub reader: Vec<String>,
}

pub const FILE_EXCHANGE_MATRIX: &str = "file.exchange.matrix";
pub const FILE_EXCHANGE_MATRIX_VERSION: u32 = 1;

/// A support matrix's rows (docs/adr/1880), in order: the objects written, their kinds, curves, heights, measures,
/// attributes, style, persistent ids, the vertices' places, and objects read back that match none.
pub const EXCHANGE_ASPECTS: [&str; 10] =
    ["objects", "kinds", "curves", "z", "m", "attributes", "style", "identity", "position", "extra"];

/// Input of `file.exchange.matrix` v1 (Destek matrisi; plan/map-cad.md MAP-17, docs/adr/1880): the drawing's round trip
/// through each of `formats` (empty: all six, in [`ExchangeFormat::ALL`]'s order; one asked twice is tried once) as
/// `file.exchange.check` makes it, with its `layers` and `tolerance`, and what each kept, approximated, converted, added
/// or lost, aspect by aspect.
///
/// Refusals: `file.exchange.check`'s (`invalid_tolerance`, `layer_not_found`, the writer's own, `exchange_unreadable`,
/// `file_unwritable`), the format's path `formats[i]` beside the check's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeMatrix {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ExchangeFormat>>", optional))]
    pub formats: Vec<ExchangeFormat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// What a trip did to an aspect: kept it, approximated it (curves to chords, vertices beyond the tolerance), converted it
/// (a kind into another, an attribute's value), added something, or lost it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExchangeState {
    Kept,
    Approximated,
    Converted,
    Added,
    Lost,
}

/// A cell: the format, what it did to the aspect, how many objects it touched and what (the kinds not written, `circle →
/// polygon`, the attributes' names, the greatest offset in metres).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeCell {
    pub format: ExchangeFormat,
    pub state: ExchangeState,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub detail: Option<String>,
}

/// A row: its aspect (one of [`EXCHANGE_ASPECTS`]) and a cell for each format tried.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeRow {
    pub aspect: String,
    pub cells: Vec<ExchangeCell>,
}

/// Output of `file.exchange.matrix` v1: the tolerance, the rows in [`EXCHANGE_ASPECTS`]' order, and each format's
/// `file.exchange.check` answer the rows are read from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExchangeMatrixed {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
    pub rows: Vec<ExchangeRow>,
    pub checks: Vec<ExchangeChecked>,
}
