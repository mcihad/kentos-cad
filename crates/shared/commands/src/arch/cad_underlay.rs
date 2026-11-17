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
