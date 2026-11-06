//! KentOS Yol's plan–profile sheets (plan/road.md Y10.5, docs/adr/0908 §10; ROAD-SEC-11): `road.sheets.plan` lays an
//! alignment out on sheets as Civil 3D's view frames and Create Sheets do: each sheet shows a stretch of the alignment
//! in plan (turned so it runs left to right) and, with a profile, the profile view of the same stations drawn for it in
//! the model at the same horizontal scale; the sheet set keeps the sources' revisions. The desktop's handler is
//! `crates/native/application/src/road_sheets.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/road.sheets.plan.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::SheetTemplateSource;

pub const ROAD_SHEETS_PLAN: &str = "road.sheets.plan";
pub const ROAD_SHEETS_PLAN_VERSION: u32 = 1;
pub const ROAD_SHEETS_SECTIONS: &str = "road.sheets.sections";
pub const ROAD_SHEETS_SECTIONS_VERSION: u32 = 1;

fn empty_options() -> serde_json::Value {
    serde_json::json!({})
}

/// Input of `road.sheets.plan` v1: the alignment by its id or name, its stations from `from` to `to` (internal, m; its
/// ends without them), laid on sheets of `source`'s template (KentOS's plan–profile sheet, `sys:yol-plan-boykesit`,
/// without one; KGM's A1 one, `sys:yol-plan-boykesit-kgm`, with `bands: kgm`, docs/adr/0981) at 1:`scale` (the template's plan map's without it). A sheet's stretch is the plan map's width on the
/// ground; the next starts `overlap` metres before its end (0 without it). With a `profile` (on the same alignment) each
/// sheet's profile view is drawn on `layerId`, the first's lower left at `at`, each next under the one before, and the
/// template's second map shows it. The set is named `name` (“‹alignment› paftaları” without it) and the sheets numbered
/// on from `first` (Y-101 without it).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_alignment`,
/// `unknown_road_profile`, `invalid_road_sheets` (a parallel axis, a profile on another alignment, a profile without a
/// layer or a place, a range off the alignment or backwards, a scale not in [1, 1 000 000], an overlap not under half a
/// sheet's stretch, more than 500 sheets, a set's name taken), the layer's, `unknown_template`, `invalid_options`,
/// `not_fixed_map`, `bad_drawing_info`, then the sheet core's and the profile drawing's. Warnings: `road_sheet_outside`
/// (the alignment or a profile view leaves its map on some sheets), `layer_hidden`, `profile_stale`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetsPlan {
    pub alignment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: Option<f64>,
    /// The profile views' bands (docs/adr/0980): absent or `kentos` KentOS's, `kgm` KGM's 2019 order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bands: Option<crate::ProfileBands>,
    /// With `bands: kgm`, the superelevation schedule the views' Dever band draws (its id or name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub superelevation: Option<String>,
    /// With `bands: kgm`, the corridor whose slopes and platform the rows over each view say (docs/adr/0989).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub overlap: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<SheetTemplateSource>,
    /// The template's paper and answers, every sheet's alike (cad.sheets.create's options but the name, centre and scale).
    #[serde(default = "empty_options")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(feature="schema",schemars(with="kentos_sheet::template::InstanceOptions", extend("x-raw-record"=true)))]
    pub options: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub first: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(regex(pattern=crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Input of `road.sheets.sections` v1: a sample line set's section views laid on sheets as Civil 3D's production
/// section views and Create Section Sheets do (docs/adr/0908 §11): the set's views (`stations`, all without them) drawn
/// on `layerId` in groups, each group the template's map (KentOS's technical sheet, `sys:cad-teknik`, without one) wide
/// and high on the ground at 1:`scale` (the map's without it), filled in rows from the upper left as many views as fit;
/// the first group's upper left at `at`, each next under the one before. Each group is a sheet whose map shows it. The
/// views are `road.sections.draw`'s (`exaggeration`, `height`). The set is named `name` (“‹set› paftaları” without
/// it) and the sheets numbered on from `first` (E-101 without it).
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `not_a_road_project`, `unknown_section_set`, the
/// section drawing's (`invalid_label`, `invalid_places`, `no_terrain`, the layer's), `invalid_road_sheets` (a scale not
/// in [1, 1 000 000], a view larger than the map at this scale, more than 500 sheets, a set's name taken),
/// `unknown_template`, `invalid_options`, `not_fixed_map`, `bad_drawing_info`. Warnings: the section drawing's
/// (`layer_hidden`, `section_set_stale`, `section_open` …).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetsSections {
    pub set: String,
    pub layer_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: [f64; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub stations: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub exaggeration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<SheetTemplateSource>,
    /// The template's paper and answers (cad.sheets.create's options but the name, centre and scale).
    #[serde(default = "empty_options")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(feature="schema",schemars(with="kentos_sheet::template::InstanceOptions", extend("x-raw-record"=true)))]
    pub options: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub first: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(regex(pattern=crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A sheet laid: the book's sheet, its number and title, its stations and km, the plan map's centre and turn (degrees,
/// the content clockwise on paper), the profile view drawn for it (its frame's lower left and upper right; on a section
/// sheet the group's region) and whether the alignment or the profile view leaves its map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetLaid {
    pub sheet: String,
    pub number: String,
    pub title: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub from: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub to: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub center: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub view: Option<[[f64; 2]; 2]>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub outside: bool,
}

/// Output of `road.sheets.plan` v1, and its plan: the set's id and name, its alignment and profile, its scale, a sheet's
/// stretch on the ground (m) and its plan map's height there, the sheets, the objects written (none in a plan) and the
/// drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RoadSheetsLaid {
    pub set: String,
    pub name: String,
    pub alignment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    /// A section sheet set's sample line set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sections: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: f64,
    pub sheets: Vec<RoadSheetLaid>,
    pub uids: Vec<String>,
    pub revision: String,
}
