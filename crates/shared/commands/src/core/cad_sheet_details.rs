//! `cad.sheets.details` v1 (“Detay paftaları”, docs/adr/1090 §2): general 2D detail sheets of one model at exact
//! ratios, each detail a locked viewport with its own layer state, overrides and annotation representation; no
//! architectural level data. One change of the book, one undo, as `cad.sheets.levels`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_SHEETS_DETAILS: &str = "cad.sheets.details";
pub const CAD_SHEETS_DETAILS_VERSION: u32 = 1;

fn empty_options() -> serde_json::Value {
    serde_json::json!({})
}

/// A detail's model source: an extent, or a model table's part (docs/adr/1030 §7: a broken table's parts go on
/// sheets of their own, docs/adr/1090 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DetailTablePart {
    /// The model table's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::cad::UID_TEXT)))]
    pub table: String,
    /// Its part, from 0 (the first part, left).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub part: u32,
}

/// One detail of `cad.sheets.details`: exactly one of `bounds` and `table`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DetailRequest {
    pub name: String,
    /// The model extent, canonical metres: min east, min north, max east, max north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub bounds: Option<[f64; 4]>,
    /// A model table's part instead of an extent: the part's outline, half a text height around it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<DetailTablePart>,
    /// Paper:model in the same unit (1:5 is `{paper:1, model:5}`).
    pub ratio: kentos_contracts::PaperScaleRatio,
    /// Which layers it draws (`{type:"theme", name}` for a layer state); absent: the drawing's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "Record<string, unknown>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<kentos_sheet::kinds::MapLayers>")
    )]
    pub layers: Option<serde_json::Value>,
    /// Layers drawn otherwise in this viewport only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "Array<Record<string, unknown>>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<Vec<kentos_sheet::kinds::LayerOverride>>")
    )]
    pub layer_overrides: Option<Vec<serde_json::Value>>,
    /// The annotation representation (`{type:"fixed", scale}`); absent: the viewport's own scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "Record<string, unknown>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Option<kentos_sheet::viewport::AnnotationScale>")
    )]
    pub annotation: Option<serde_json::Value>,
    /// The camera locked; absent: locked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locked: Option<bool>,
}

/// Input of `cad.sheets.details` v1: the details packed in their order into the template's first fixed map frame, row
/// by row, running on to further sheets of the set; the sheets numbered on from `first` (D-501) and titled `title`.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, `entity_not_found` (a table part's table),
/// `not_a_table`, `table_part_missing`, `ambiguous_source` (both or neither of bounds and table), `unknown_template`,
/// `invalid_options`, `bad_value`, `invalid_extent`, `invalid_ratio`, `detail_too_small`, `not_fixed_map`,
/// `bad_drawing_info`, `detail_does_not_fit`, then the sheet core's own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsDetails {
    pub details: Vec<DetailRequest>,
    /// The template; absent: KentOS's ISO technical sheet (`sys:cad-teknik`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<crate::SheetTemplateSource>,
    /// The template's paper and answers (cad.sheets.create's options but the name, centre and scale).
    #[serde(default = "empty_options")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(feature="schema",schemars(with="kentos_sheet::template::InstanceOptions", extend("x-raw-record"=true)))]
    pub options: serde_json::Value,
    /// The first sheet's number; absent: D-501.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub first: Option<String>,
    /// The sheets' title; absent: Detaylar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(regex(pattern=crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.sheets.details` v1 (its plan's too): the book change and where each detail went.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsDetailed {
    #[serde(flatten)]
    pub edited: crate::SheetsEdited,
    /// The sheets made, in order.
    #[cfg_attr(
        feature = "ts",
        ts(type = "Array<{sheet: string, number: string, name: string}>")
    )]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<kentos_sheet::details::DetailSheet>")
    )]
    pub made: Vec<serde_json::Value>,
    /// Each detail in the input's order: its sheet, viewport and title ids, paper frame (µm) and denominator.
    #[cfg_attr(feature = "ts", ts(type = "Array<Record<string, unknown>>"))]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<kentos_sheet::details::DetailPlaced>")
    )]
    pub details: Vec<serde_json::Value>,
}
