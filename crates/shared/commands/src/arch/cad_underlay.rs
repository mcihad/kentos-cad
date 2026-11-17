//! `cad.underlay.edit` v1 (“Altlık model”, docs/adr/8101; plan/ifc-uyum.md IFC-09, B18): the drawing's underlay models
//! (`settings.arch.underlays`). A file is added — read for its SHA-256, its size and schema, and placed by its map
//! conversion onto the project's system (or at the origin when asked); its name, discipline, revision, placement and
//! whether it is shown are set; it is read again when its file changed; it is taken away. One undo step “Altlık model”;
//! the drawing's objects never change and the model is never copied into the drawing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{ModelUnderlay, UnderlayPlacement};

/// The command's name.
pub const CAD_UNDERLAY_EDIT: &str = "cad.underlay.edit";
/// Its version.
pub const CAD_UNDERLAY_EDIT_VERSION: u32 = 1;

/// One change of the underlays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum UnderlayChange {
    /// An IFC file shown under the drawing: its absolute path; the name the list shows (its file's name when none);
    /// the discipline and revision the user gives it; how it is placed (by its map conversion when none).
    #[serde(rename_all = "camelCase")]
    Add {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        file: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        discipline: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        revision: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        placement: Option<UnderlayPlacement>,
    },
    /// Its name, discipline, revision (an empty text takes it away), placement (the file is read again to place it)
    /// or whether it is shown; at least one.
    #[serde(rename_all = "camelCase")]
    Set {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        discipline: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        revision: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        placement: Option<UnderlayPlacement>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        shown: Option<bool>,
    },
    /// Its file read again (a new revision on disk, or the file moved to `file`): the digest, size and schema
    /// renewed, the placement checked again.
    #[serde(rename_all = "camelCase")]
    Reload {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        file: Option<String>,
    },
    /// Taken away; the file stays where it is.
    #[serde(rename_all = "camelCase")]
    Remove { id: String },
}

/// Input of `cad.underlay.edit` v1.
///
/// Refusals, in this order: `invalid_uid`, `model_underlay_not_found`, `duplicate_model_underlay`, `invalid_file_path`
/// (not an absolute path), `file_not_found`, `model_underlay_unreadable` (not an IFC file the reader reads),
/// `model_underlay_not_placed` (another system, no map conversion, a project with no system, other height datums, a
/// scale the file's unit does not explain), `nothing_to_set`, `invalid_model_underlay` (its field's path),
/// `too_many_model_underlays`, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayEdit {
    pub change: UnderlayChange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.underlay.edit` v1, and its plan: the underlay changed (its id), the drawing's underlays then, the
/// revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayEdited {
    pub id: String,
    pub underlays: Vec<ModelUnderlay>,
    pub revision: String,
}

/// The alignment and clash check's name (docs/adr/8102; plan/bim-plan.md BIM-17).
pub const CAD_UNDERLAY_CHECK: &str = "cad.underlay.check";
/// Its version.
pub const CAD_UNDERLAY_CHECK_VERSION: u32 = 1;
/// The tolerance when none is given, metres: the drawing's own millimetre.
pub const UNDERLAY_TOLERANCE: f64 = 0.001;
/// The smallest tolerance, metres: a tenth of a millimetre, above what the bodies' float32 corners may stray (a body
/// is kept as float32 corners about a float64 origin).
pub const MIN_UNDERLAY_TOLERANCE: f64 = 0.0001;
/// The largest tolerance, metres.
pub const MAX_UNDERLAY_TOLERANCE: f64 = 0.5;

/// Input of `cad.underlay.check` v1: the underlay checked against the drawing's model; two bodies whose boxes overlap by
/// no more than `tolerance` metres on an axis are not looked at, and two elements of one class whose boxes stand within
/// it are the same element.
///
/// Refusals, in this order: `invalid_uid`, `model_underlay_not_found`, `invalid_tolerance` (0.0001 to 0.5 m),
/// `file_not_found`, `model_underlay_changed` (the file is not the revision the drawing names: read it again first),
/// `model_underlay_unreadable`, `model_underlay_not_placed` (another system or height datum stops it before any geometry
/// is looked at).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayCheck {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// A project level and the file's storey paired with it by elevation (the drawing's heights, metres): either may be
/// missing; the difference is the storey's less the level's. A storey whose file says no elevation is listed alone,
/// without one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayLevelCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub storey: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub project: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub file: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub difference: Option<f64>,
}

/// A grid axis by its label: how far the file's stands from the project's (metres) and the turn between them
/// (degrees), when both have it; else which has it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayAxisCheck {
    pub label: String,
    pub in_project: bool,
    pub in_file: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub turn: Option<f64>,
}

/// A clash: the drawing's object (its persistent id, IFC class and name) and the file's (its GlobalId, class and name),
/// their boxes' overlap in the drawing (x, y, z from, x, y, z to) and whether one holds the other whole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayClash {
    pub uid: String,
    pub class: String,
    pub name: String,
    pub object: String,
    pub object_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub object_name: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub overlap: Vec<f64>,
    pub inside: bool,
}

/// An element of a class both models have that only one of them has where it stands: the drawing's (its persistent
/// id) or the file's (its GlobalId), its class and name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayUnmatched {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub object: Option<String>,
    pub class: String,
    pub name: String,
}

/// Output of `cad.underlay.check` v1: the levels and axes set beside each other, how many elements both models have
/// where they stand, the clashes, what only one model has, the tolerance used, the drawing's revision.
///
/// Orders: the levels' pairs in the project's level order, then the project's unpaired levels, then the file's unpaired
/// storeys (in the file's order); the axes by label; the clashes in the file's order, each file object's by the
/// drawing's persistent id; the unmatched the drawing's first (by persistent id), then the file's (in its order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayChecked {
    pub id: String,
    pub levels: Vec<UnderlayLevelCheck>,
    pub axes: Vec<UnderlayAxisCheck>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub matched: u32,
    pub clashes: Vec<UnderlayClash>,
    pub unmatched: Vec<UnderlayUnmatched>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
    pub revision: String,
}
