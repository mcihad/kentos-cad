//! `cad.readjustment.sheets` v1 (plan/map-cad.md MAP-11, docs/adr/2680): Ayırma çapı paftaları. One sheet for each
//! parcel partly inside a land readjustment's boundary, as Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik
//! (RG 22.02.2020 / 31047) Article 13 has it: a separation sheet per parcel, every piece outside on the same sheet
//! (13(3)), showing the registered and computed area, their difference and the tolerance, and the parts inside and
//! outside with their areas, boundaries and coordinates (13(2)). The numbers and the parts are `cad.readjustment.scope`'s;
//! the parts' corners are the sheet's numbered points (the scope's `points`).
//!
//! The points are written in the drawing, each parcel's on a layer of its own (named by the parcel) under the group
//! “Ayırma çapı noktaları”, labelled with their numbers: the sheet's map shows them, its coordinate list lists them, and
//! the other parcels' point layers are frozen in that map, so a corner two parcels share shows one number on each
//! sheet. The tables of areas and parts are written with the policy's places. One undo step. A draft: the regulation
//! has the separation sheets approved with the parcellation plan (13(2)); KentOS approves nothing. The shared cases are
//! `fixtures/commands/v1/cad.readjustment.sheets.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{ReadjustmentScope, SheetTemplateSource};

pub const CAD_READJUSTMENT_SHEETS: &str = "cad.readjustment.sheets";
pub const CAD_READJUSTMENT_SHEETS_VERSION: u32 = 1;

fn empty_options() -> serde_json::Value {
    serde_json::json!({})
}

/// Input of `cad.readjustment.sheets` v1: the scope (`cad.readjustment.scope`'s input: the boundary, the parcels, the
/// registered area's attribute, the places, rounding and piece tolerance), the template (KentOS's separation sheet,
/// `sys:ayirma-capi`, without one; a template of one's own needs a fixed map with the id `harita`, and may have a
/// coordinate list `koordinatlar` and fixed tables `yuzolcumleri` and `kisimlar`), and the template's options: its
/// paper, a scale (without one, each map takes the standard scale its parcel fits at with a 5% margin) and answers to
/// its questions. Each sheet is named “Ayırma çapı ‹Ada/Parsel›”; the template's `il`, `ilce`, `mahalle`, `ada` and
/// `parsel` are answered from the parcel's own attributes.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`, the scope's own at `scope.…`, `no_partial_parcels`
/// (no parcel is partly inside), `no_identity` and `duplicate_parcel` (a partial parcel without Ada or Parsel, or with
/// another's), `unknown_template`, `invalid_options` (options that do not read; a name, a centre or an answer to a
/// parcel's attribute: those are each sheet's own), `not_fixed_map` (no fixed map `harita`), `separation_sheet_exists`
/// (a sheet of the name, or a point layer of the parcel, is there already), then the sheet core's own. `numbers` that do
/// not fit (a parcel not partial in the scope, not one number for each outside part, an empty or a repeated one) are
/// `invalid_input` at `numbers.‹uid›`; a `numberHeight` not above zero is `invalid_input` at `numberHeight`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentSheets {
    pub scope: ReadjustmentScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<SheetTemplateSource>,
    /// The template's paper, scale and answers (cad.sheets.create's options but the name and the centre).
    #[serde(default = "empty_options")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    #[cfg_attr(feature = "schema", schemars(with = "kentos_sheet::template::InstanceOptions", extend("x-raw-record" = true)))]
    pub options: serde_json::Value,
    /// The parcel numbers the parts of a partial parcel outside the area are registered with (Yönetmelik m. 13/3,
    /// docs/adr/3080), by the parcel's persistent id, one for each outside part in the parts' order: written beside each
    /// outside part in the table of parts. The regulation gives no rule for them; KentOS writes none of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub numbers: Option<std::collections::BTreeMap<String, Vec<String>>>,
    /// The outside parts' numbers' height on the sheet, paper millimetres (docs/adr/4280; the user's, KentOS gives
    /// none): given, each number is written as a text fitted inside its part, horizontal or along the part's longest
    /// straight edge, its height stepped down by tenths of this until it fits; none: as a labelled point (docs/adr/3480).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub number_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = crate::cad::REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A separation sheet: its id and name, its parcel (uid and `Ada/Parsel`), the layer its points are on (none in a plan),
/// its map's scale (1:N) and centre (east, north; m), how many points it numbers, and the rows written in its tables of
/// areas (the item, square metres) and parts (the part, its boundary as point numbers, square metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SeparationSheet {
    pub sheet: String,
    pub name: String,
    pub uid: String,
    pub parcel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub center: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub points: u32,
    pub areas: Vec<Vec<String>>,
    pub parts: Vec<Vec<String>>,
    /// The numbered outside parts written on the drawing (docs/adr/3480): each part's name, its parcel number and
    /// where the number stands inside it; none when no numbers are given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SeparationLabel>>"))]
    pub labels: Vec<SeparationLabel>,
}

/// An outside part's parcel number on the drawing (docs/adr/3480): the part's name on the sheet (“Girmeyen 1”), the
/// number as given (trimmed) and its place: the middle of the widest stretch of the part's horizontal line at the half
/// height of its outer ring's corners (inside the part whatever its shape).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SeparationLabel {
    pub part: String,
    pub number: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub at: [f64; 2],
    /// With `numberHeight` (docs/adr/4280): the text's height, metres on the drawing, its turn (degrees, counter-clockwise
    /// from east), its share of the height asked (1, 0.9 … 0.1) and whether its box lies inside the part; `at` is then
    /// its middle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub share: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fits: Option<bool>,
}

/// Output of `cad.readjustment.sheets` v1, and its plan: the point layers' group (none in a plan), the sheets in the
/// scope's order, the points written (none in a plan) and the drawing's revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentSheetsMade {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    pub sheets: Vec<SeparationSheet>,
    pub uids: Vec<String>,
    pub revision: String,
}
