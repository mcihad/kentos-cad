//! `cad.asbuilt.check` v1 (plan/map-cad.md MAP-19, docs/adr/1700): Uygulama kontrolü, read only. The points measured
//! in the field (a layer of points with their names and heights) set against the design: the design's points (matched
//! by name, else, when a search radius is given, by the nearest design point within it, said as a proposal; two within
//! it are ambiguous and matched to neither), the design's surface (the height difference at every measured point) and
//! the design's lines (the signed normal offset to the nearest, left positive, for a point no design point matches
//! and, a radius given, within it of a line: a kerb's points, not the stake points beside it). Every limit is the user's (the
//! administration's or the contract's acceptance profile): a limit not given is not checked. A measured point's own
//! quality (its σ in an attribute) beyond the limit is reported apart from its difference: a poor solution is not a
//! construction fault.
//!
//! The design's digest (its points' names, places and heights) is in the answer. A check given the digest the field
//! record was made against says when the design has changed since, and still compares, so a record of an older design
//! is never matched to a newer one silently. Given the field package itself (docs/adr/2420), the check reads the digest,
//! the acceptance limits and the system from it. Nothing is written. The shared cases are
//! `fixtures/commands/v1/cad.asbuilt.check.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ASBUILT_CHECK: &str = "cad.asbuilt.check";
pub const CAD_ASBUILT_CHECK_VERSION: u32 = 1;

/// The most measured points a check takes.
pub const ASBUILT_LIMIT: usize = 100_000;

/// A measured point's flags, in the order they are listed.
pub const ASBUILT_FLAGS: [&str; 8] = [
    "proposed",
    "outsideHorizontal",
    "outsideVertical",
    "outsideSurface",
    "outsideLine",
    "lowQuality",
    "noHeight",
    "offSurface",
];

/// Input of `cad.asbuilt.check` v1. `measured` is a layer's id; `design` and `lines` too (at least one of `design`,
/// `surface`, `lines`); `surface` a terrain surface's id or name (Harita and Yol projects); `field` the attribute the
/// names are in (none: `Ad`); `search` the radius of a match by place (metres; none: by name only); the tolerances in
/// metres; `qualityField` the attribute of a point's σ (metres) and `quality` its limit; `expectedDesign` the design
/// digest the field record was made against.
///
/// Refusals, in this order: `invalid_asbuilt_check` (no design at all, a limit or radius not above zero, an empty
/// attribute's name, `quality` without `qualityField`); the package's (docs/adr/2420; at `package`): `invalid_file_path`
/// (empty, relative, not `.zip`), `file_not_found`, `invalid_asbuilt_check` (not a field package; at `expectedDesign`:
/// another digest than the package's); `layer_not_found` (`measured`, `design`, `lines`),
/// `not_a_road_project` and the surface's own (`unknown_terrain`, `terrain_unbuildable`), `invalid_asbuilt_check`
/// (more than 100 000 measured points).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltCheck {
    pub measured: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub design: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lines: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub search: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quality_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_design: Option<String>,
    /// The field package (`cad.field.package`'s `.zip`, an absolute path) the record was made with (docs/adr/2420): its
    /// design digest is the one expected, its acceptance limits apply where this input gives none (its quality limit
    /// only with `qualityField`), its system, epoch and heights are set against the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub package: Option<String>,
}

/// The field package a check was given (docs/adr/2420): the file, its stamp, the drawing's revision and the design
/// digest it was written with; its system (`EPSG:5256`), epoch and heights in words and whether each is the project's
/// now; its acceptance limits (metres) and which of them the check took (`horizontal`, `vertical`, `quality`: those the
/// input did not give).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltPackage {
    pub path: String,
    pub stamp: String,
    pub revision: String,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub epsg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub epoch: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub heights: Option<String>,
    pub same_system: bool,
    pub same_epoch: bool,
    pub same_heights: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub taken: Vec<String>,
}

/// A measured point as checked: its object and name; the design point it is matched to (`by`: `name`, `place` or
/// none) with the differences measured − design (east, north, height) and the horizontal distance; the difference to
/// the surface (measured − surface); the signed normal offset to the nearest design line (left of its direction
/// positive) and that line (a line point only); its σ; its status and flags (of [`ASBUILT_FLAGS`]). The status:
/// `ambiguous`; `unmatched` when neither a design point nor a line takes it (a surface alone: every point is checked
/// against it); else `outside` when a limit is broken, `within`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltPoint {
    pub uid: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub design_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub design_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub de: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dn: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dh: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub surface: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub normal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub sigma: Option<f64>,
    pub status: String,
    pub flags: Vec<String>,
}

/// A design point no measured point matched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltMissing {
    pub uid: String,
    pub name: String,
}

/// Output of `cad.asbuilt.check` v1: the design's digest and whether it differs from the one expected; every measured
/// point in the drawing's order; the design points not measured; the counts by status and of the low-quality points;
/// the root mean square and the greatest of the matched points' horizontal distances and height differences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AsbuiltChecked {
    pub design_digest: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub design_changed: bool,
    pub points: Vec<AsbuiltPoint>,
    pub missing: Vec<AsbuiltMissing>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub within: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unmatched: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ambiguous: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub low_quality: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms_horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms_vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_vertical: Option<f64>,
    /// The field package given (docs/adr/2420).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub package: Option<AsbuiltPackage>,
}
