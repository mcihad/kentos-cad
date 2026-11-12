//! The semantic commands of view marks and a space's finishes (docs/adr/0345 §3, §4; plan/cad.md `CAD-ARCH-37`–`40`):
//! marks put (`cad.viewmarks.create`), changed, pointed elsewhere or let go (`cad.viewmarks.edit`), the marks and their
//! targets listed (`cad.viewmarks.references`, read-only), and a space's floor pattern or ceiling grid written as a hatch
//! tied to it (`cad.spaces.finish`). Each that writes is one undo step.
//!
//! Lengths are metres on the wire, whatever unit the project shows (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};
use crate::{ViewMarkForm, ViewMarkLabel, ViewTarget};

pub const CAD_VIEWMARKS_CREATE: &str = "cad.viewmarks.create";
pub const CAD_VIEWMARKS_CREATE_VERSION: u32 = 1;
pub const CAD_VIEWMARKS_EDIT: &str = "cad.viewmarks.edit";
pub const CAD_VIEWMARKS_EDIT_VERSION: u32 = 1;
pub const CAD_VIEWMARKS_REFERENCES: &str = "cad.viewmarks.references";
pub const CAD_VIEWMARKS_REFERENCES_VERSION: u32 = 1;
pub const CAD_SPACES_FINISH: &str = "cad.spaces.finish";
pub const CAD_SPACES_FINISH_VERSION: u32 = 1;
/// The most marks one call writes.
pub const MAX_VIEWMARKS: usize = 1_000;
/// The layer a floor pattern goes on when the call names none.
pub const FLOOR_LAYER: &str = "A-ZEMN-DSN";
/// The layer a ceiling grid goes on when the call names none.
pub const CEILING_LAYER: &str = "A-TAVN-IZG";

/// One mark to put.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewViewMark {
    pub form: ViewMarkForm,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub points: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: Option<f64>,
    /// A detail's bubble; absent: on the circle's upper right at twice its radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub flip: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub mark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub target: Option<ViewTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<f64>,
}

/// Input of `cad.viewmarks.create` v1 (“Görünüm işareti”): the marks on a named layer, their words written from their
/// targets, as one undo step.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_viewmark`, `target_not_found`, `invalid_line_weight`,
/// `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also
/// `slots_exhausted`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarksCreate {
    pub layer_id: String,
    pub marks: Vec<NewViewMark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.viewmarks.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarksCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.viewmarks.create` would write (plan mode): the marks as they would be stored, their words written, `id` 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarksPlan {
    pub marks: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The fields of marks to change; absent ones stay, a nullable one's null takes it away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarkChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mark: Option<String>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<Option<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub flip: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: Option<f64>,
}

/// What `cad.viewmarks.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ViewMarksOperation {
    /// The marks' number, words' height, side, direction or radius changed.
    Set(Box<ViewMarkChange>),
    /// The marks point at another target.
    Retarget { target: ViewTarget },
    /// The marks let go of their target: the bottom half writes “—”.
    Detach,
}

/// Input of `cad.viewmarks.edit` v1 (“Görünüm işaretini düzenle”): one undo step, all or none.
///
/// Refusals, in this order: `no_entities`, `invalid_uid`, `entity_not_found`, `not_a_viewmark`, `nothing_to_set`,
/// `not_finite`, `invalid_viewmark`, `target_not_found`, `invalid_revision`, `revision_conflict`, `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarksEdit {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub operation: ViewMarksOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.viewmarks.edit` v1 (its plan's too: the marks as they would be, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarksEdited {
    pub marks: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.viewmarks.references` v1 (“Başvurular”), read-only.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarksReferences {}

/// A mark as `cad.viewmarks.references` lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MarkReference {
    pub uid: String,
    pub form: ViewMarkForm,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub mark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub target: Option<ViewTarget>,
    pub words: ViewMarkLabel,
    /// Its target is gone.
    pub broken: bool,
    /// Where to look: its first point.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
}

/// A target as `cad.viewmarks.references` lists it: a named view, or a map frame on a sheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TargetReference {
    pub target: ViewTarget,
    /// The named view's name, or the sheet's name and the frame's name joined by “ › ”.
    pub name: String,
    /// What a mark pointing at it writes in its bottom half.
    pub number: String,
    /// A sheet frame's order among its sheet's frames, from 1; none for a named view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub order: Option<u32>,
    /// The marks pointing at it, in the drawing's order.
    pub marks: Vec<String>,
}

/// Output of `cad.viewmarks.references` v1 (its plan's too).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarksReferenced {
    /// In the drawing's order.
    pub marks: Vec<MarkReference>,
    /// The named views in the project's order, then the sheets' map frames in the book's order.
    pub targets: Vec<TargetReference>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Which face of a space a finish covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FinishSurface {
    /// Döşeme deseni.
    #[default]
    Floor,
    /// Tavan ızgarası.
    Ceiling,
}

impl FinishSurface {
    pub fn label(self) -> &'static str {
        match self {
            FinishSurface::Floor => "Döşeme",
            FinishSurface::Ceiling => "Tavan",
        }
    }
}

/// Where a ceiling grid's lines start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GridOrigin {
    /// The space's bounding box's middle: the tiles are cut alike on opposite walls.
    #[default]
    Center,
    /// The space's bounding box's lower left corner.
    Corner,
}

/// A ceiling grid: two families of lines `dx` and `dy` apart, turned by `angle`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FinishGrid {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dx: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dy: f64,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: f64,
    #[serde(default)]
    pub origin: GridOrigin,
}

/// Input of `cad.spaces.finish` v1 (“Döşeme ve tavan”): a hatch tied to each space, its floor pattern (`pattern`, a
/// library's or the project's pattern's name; absent: the space's floor finish's name if a pattern is so named, else
/// ANSI31) or its ceiling grid (`grid`), on a layer (absent: A-ZEMN-DSN or A-TAVN-IZG, opened if missing); a space's
/// earlier hatch of the same face is replaced. One undo step.
///
/// Refusals, in this order: `no_entities`, `invalid_uid`, `entity_not_found`, `not_a_space`, `not_finite`,
/// `invalid_finish`, `unknown_pattern`, `invalid_revision`, `revision_conflict`, `not_a_layer`, `layer_locked`; on the
/// desktop also `slots_exhausted`. Warnings: `finish_replaced`, `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesFinish {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub surface: FinishSurface,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pattern: Option<String>,
    /// The pattern's scale; absent: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grid: Option<FinishGrid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.spaces.finish` v1 (its plan's too: the hatches as they would be, `id` 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesFinished {
    pub hatches: Vec<Entity>,
    /// The earlier hatches taken away, by their persistent ids.
    pub replaced: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
