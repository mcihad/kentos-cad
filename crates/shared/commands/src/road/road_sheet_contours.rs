//! `road.terrains.sheetcontours` v1 (plan/map-cad.md MAP-14, docs/adr/1820): Pafta eş yükselti eğrileri, a surface's
//! contour lines drawn for a sheet by BÖHHBÜY's Article 78(1)(c)–(g) and Ek-3's signs 0001–0005, the values a rule
//! package's (`catalog/v1/rules/esyukselti-bohhbuy-2018.json`), not constants in code:
//! - (c) the interval by the scale; (d) every fifth line major, 0.25 mm, the others 0.13 mm;
//! - (e) where the lines would lie under 2 mm apart on the sheet only the major ones, where over 20 mm apart a dashed
//!   auxiliary line half way between (judged triangle by triangle from its slope);
//! - (f) on the major lines, every 200 mm, the line's height written along it, its top uphill, its box masked (the
//!   gap the article asks for, the line itself kept whole);
//! - (g) no line across the closed objects of the detail layers (buildings, roads and rivers drawn as areas, slope
//!   boundaries): the parts inside are left out.
//!
//! Written as 3D polylines and texts on a layer in one undo step “Pafta eş yükselti eğrileri”. The shared cases are
//! `fixtures/commands/v1/road.terrains.sheetcontours.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const ROAD_TERRAINS_SHEET_CONTOURS: &str = "road.terrains.sheetcontours";
pub const ROAD_TERRAINS_SHEET_CONTOURS_VERSION: u32 = 1;

/// A scale's contour interval (metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ContourInterval {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub interval: f64,
}

/// A sheet contour rule package: who issues it, the document, when it holds, its release; the interval by scale,
/// every how many lines a major one, the lines' weights (paper mm), the sheet spacing (mm) under which only major
/// lines are drawn and over which an auxiliary one is, the labels' spacing and height (paper mm), the spot heights'
/// text height (paper mm, Ek-3 0004; none: the package gives none, and spot heights are refused, docs/adr/2320) and
/// the auxiliary line's pattern on paper (mm: a dash positive, a gap negative; Ek-3 0003's 5 mm dash and 1 mm gap; none:
/// the built-in dashed type, docs/adr/2720); docs/adr/4740: the spacing of the heights in built-up areas and along
/// roads (paper mm, m. 63(2)'s about 2 cm; none: such heights are refused) and the section check's limit (m. 91(2),
/// m. 93(1)(c); none: no comparison).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ContourRules {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub release: u32,
    pub jurisdiction: String,
    pub source: String,
    pub effective: String,
    pub intervals: Vec<ContourInterval>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub major_every: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub major_weight: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub minor_weight: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub auxiliary_weight: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub steep_below: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub auxiliary_above: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub label_every: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub label_height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub spot_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub auxiliary_pattern: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height_every: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub section_check: Option<SectionCheck>,
}

/// The section check's limit (BÖHHBÜY m. 91(2) and m. 93(1)(c), docs/adr/4740): at the scale's denominator S,
/// KOH_X,Y in centimetres is `xy[0]·S² + xy[1]·S + xy[2]`, KOH_Z is `zFactor` times it, and the RMS of the differences
/// between measured heights and the heights read from the contour lines is under `factor` times KOH_Z. Docs/adr/5120:
/// the section's points are at most `spacing` metres apart along it (m. 93(1)(b)'s 20 m; none: not checked).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SectionCheck {
    /// The formula's coefficients of S², S and 1, giving centimetres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub xy: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub z_factor: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub spacing: Option<f64>,
}

/// KentOS's default package: BÖHHBÜY 2018's Article 78(1)(c)–(g) and Ek-3 0001–0005.
pub fn contour_rules_default() -> Option<ContourRules> {
    serde_json::from_str(include_str!("../../../../../catalog/v1/rules/esyukselti-bohhbuy-2018.json")).ok()
}

/// Input of `road.terrains.sheetcontours` v1: the surface (its id or name), the layer the lines and labels go on, the
/// scale's denominator (none: the project's plot scale), the interval (none: the package's for the scale) and the base
/// (none: 0), the detail layers whose closed objects no line crosses (none: no such rule), whether the steep rule,
/// the auxiliary lines and the labels apply (none: they do), whether the summits' and pits' spot heights are written
/// (`spots`, none: not; docs/adr/2320), the layers of the slopes' tops and bottoms whose lines' corners take spot
/// heights (`slopes`, none: no such heights; docs/adr/3120), the built-up areas' and roads' heights (`builtUp`, none:
/// none; docs/adr/4740), the rule package (none: KentOS's BÖHHBÜY 2018).
///
/// Drawn, its drawing is kept with the surface and every layer it reads (docs/adr/4740): it goes stale when they
/// change and `road.project.rebuild` draws it again.
///
/// Refusals, in this order: `not_a_road_project`, `invalid_contours` (a scale of 0, an interval not above zero or none
/// the package gives for the scale, a base not finite, a package not whole, spot heights asked of a package without
/// their text height, built-up heights asked without a spacing above nought or with neither areas nor roads, a grid
/// origin or rotation not finite),
/// `layer_not_found`, `not_a_layer`, `layer_locked` (`layerId`), `layer_not_found` (`details`, `slopes.top`,
/// `slopes.bottom`, `points`, `builtUp.areas`, `builtUp.roads`), `invalid_revision`, `revision_conflict`,
/// `unknown_terrain`, `terrain_unbuildable`, `invalid_contours` (more than 10 000 levels), `no_contours`. Warning
/// `section_check_exceeded`: the chosen points' RMS is not under the package's limit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsSheetContours {
    pub surface: String,
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub interval: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub details: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub steep: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub auxiliary: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub labels: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub spots: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slopes: Option<SheetSlopes>,
    /// The layers of the points the surveyor chose (BÖHHBÜY m. 78(1)(ğ)'s other points, docs/adr/3520): each point
    /// takes a spot height, its own measured height when it has one, else the surface's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub points: Vec<String>,
    /// The heights of built-up areas and roads (BÖHHBÜY m. 63(2), docs/adr/4740).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub built_up: Option<SheetBuiltUp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<ContourRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// The slopes whose characteristic points take spot heights (BÖHHBÜY m. 78(1)(ğ), docs/adr/3120): the layers of the
/// slopes' top lines and of their bottom lines. Every corner of a line or polyline there is such a point, its height
/// the surface's at it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetSlopes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub top: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub bottom: Vec<String>,
}

/// The heights of built-up areas and roads (BÖHHBÜY m. 63(2): “yerleşik alanlar ve yollarda harita üzerinde yaklaşık 2
/// cm'de bir”, docs/adr/4740): the layers of the built-up areas' closed boundaries and of the roads' lines, and the
/// spacing on the sheet (paper mm; none: the package's `heightEvery`). Along each road line from its start a height
/// every spacing, the first at half of it; then on the grid of the spacing inside the built-up areas (even-odd over
/// their rings), outside the details and farther than half the spacing from every height already placed. Heights are
/// the surface's, written as the summits' are; a place off the surface is counted. Docs/adr/5120: the grid runs from
/// `origin` (none: the coordinates' origin), turned by `rotation` (radians counter-clockwise from east; none: 0); its
/// places are the origin plus whole multiples of the spacing along the turned east and north. The regulation gives
/// neither: they are the surveyor's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetBuiltUp {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub areas: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub roads: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub every: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: Option<f64>,
}

/// The chosen points' own heights against the surface's (docs/adr/4740; BÖHHBÜY m. 93(1)(c)'s limit): the points
/// compared (an own height, on the surface, not one of the surface's corners: a corner's difference is nought), the
/// corners left out, the RMS of the differences and the largest one with its place (metres), KOH_Z for the scale by
/// m. 91(2) and the limit, `factor` times it (metres), and whether the RMS is under the limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetHeightCheck {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub compared: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub corners: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rms: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub largest: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub koh_z: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit: f64,
    pub within: bool,
}

/// A line as drawn: its level, its kind (`major`, `minor`, `auxiliary`), whether it closes, its places.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetContourLine {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    pub kind: String,
    pub closed: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub points: Vec<[f64; 2]>,
}

/// A height written on a major line: its level, its text, where its middle is and its turn (degrees counter-clockwise
/// from east, its top uphill).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetContourLabel {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    pub text: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: f64,
}

/// A characteristic point's height (BÖHHBÜY m. 78(1)(ğ)): a `summit` or a `pit` (docs/adr/2320), the surface's vertex
/// with the highest (lowest) height inside an innermost closed major or minor line, or a `slopeTop` or `slopeBottom`
/// (docs/adr/3120), a corner of a slope's top or bottom line with the surface's height there, or a `chosen` point
/// (docs/adr/3520), the surveyor's, with its own height (the surface's when it has none), or (docs/adr/4740, m. 63(2)) a
/// `road` height along a road line or a `builtUp` one on the grid of a built-up area; its place and height, the
/// height to the decimetre as written, and for a chosen point with its own height the surface's there (none off it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetSpot {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z: f64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub surface: Option<f64>,
}

/// Output of `road.terrains.sheetcontours` v1, and its plan: the package (id, source), the scale and the interval used,
/// the lines (by level, auxiliary ones at theirs, then by their first places) and the labels (along each major line in
/// its order), the triangles where only major lines are drawn and where an auxiliary one is, the length of line left
/// out inside the details (metres), the project line type the auxiliary lines are drawn with (the package's pattern as
/// “Yardımcı eğri (‹package›)”, added to the project's types when it has none of the name, a project setting that undo
/// leaves (docs/adr/0313); none without auxiliary lines or a pattern; docs/adr/2720), the objects written (a plan:
/// none), the revision after it and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TerrainsSheetContoured {
    pub rules: String,
    pub source: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub interval: f64,
    pub lines: Vec<SheetContourLine>,
    pub labels: Vec<SheetContourLabel>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub steep: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub gentle: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub cut: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SheetSpot>>", optional))]
    pub spots: Vec<SheetSpot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub check: Option<SheetHeightCheck>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
    pub notes: Vec<String>,
}
