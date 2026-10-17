//! `cad.hatches.edit` v1 (docs/adr/0329 §4): a hatch's boundary made objects again (AutoCAD's HATCHEDIT “Recreate
//! boundary”), its region found again around a point (“Reselect”), islands added and taken away, its tie broken;
//! each as one undo step named after the edit.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_region::HatchRegion;
use crate::entity::Entity;

pub const CAD_HATCHES_EDIT: &str = "cad.hatches.edit";
pub const CAD_HATCHES_EDIT_VERSION: u32 = 1;

/// What a hatch's boundary is made again as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BoundaryForm {
    /// One area, its holes the hatch's holes; arcs exact, an ellipse's or a curve's edges its chords within 0.1 mm.
    #[default]
    Area,
    /// One object a loop: a closed polyline of its lines and arcs, a whole ellipse an ellipse, a whole closed curve a
    /// NURBS curve; a loop mixing curves and lines a closed polyline of their chords.
    Polyline,
}

/// One edit of `cad.hatches.edit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchEdit {
    /// Sınırı yeniden oluştur: the hatch's boundary written as new objects on `layerId` (absent: the hatch's layer)
    /// in `form`; with `associative` (absent: true) the hatch then follows them.
    #[serde(rename_all = "camelCase")]
    Recreate {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        form: Option<BoundaryForm>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layer_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        associative: Option<bool>,
    },
    /// Sınırı yeniden seç: the hatch's region found again as Tarama finds it (`region`, a point; its pattern, layer
    /// and look kept), its tie and boundary the new region's.
    Reselect {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        region: HatchRegion,
    },
    /// Ada ekle: closed objects left open in the hatch. A tied hatch names them among its islands and is cut again
    /// from its objects; another has their areas taken away from its region.
    AddIslands {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Ada çıkar: islands hatched over again: a tied hatch's by their objects (`uids`, among its islands), any
    /// hatch's by its holes' places (`holes`, 0 the first).
    RemoveIslands {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        holes: Vec<u32>,
    },
    /// İlişkiyi kopar: the hatches follow nothing; they stay as they are.
    Untie {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
}

/// Input of `cad.hatches.edit` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_hatch`, `layer_locked`,
/// `layer_not_found`, `no_entities`, `not_closed`, `not_enclosed`, `island_not_found`, `invalid_region`,
/// `not_finite`, `invalid_gap`; `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchesEdit {
    pub edit: HatchEdit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.hatches.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchesEdited {
    /// The hatches that changed, by their persistent ids.
    pub changed: Vec<String>,
    /// The objects Sınırı yeniden oluştur wrote.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub created: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.hatches.edit` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchesPlan {
    /// The hatches as execute would write them.
    pub hatches: Vec<Entity>,
    /// The objects Sınırı yeniden oluştur would write, `id` 0.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Entity>>", optional))]
    pub created: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
