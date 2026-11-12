//! 2B düzleme indir (docs/adr/1061 §4; plan/cad-advanced.md `CAD-ADV-17`): the elevations (Z) of objects, and when
//! asked their measures (M), removed or set to one level, as one undo step, with the plan as its preview: which
//! objects change, how many vertices lose their elevation and between which elevations they were. Opening a drawing
//! never does this by itself: a file's Z and M are kept until this command is run.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

pub const CAD_ENTITIES_FLATTEN: &str = "cad.entities.flatten";
pub const CAD_ENTITIES_FLATTEN_VERSION: u32 = 1;
/// The undo step's name.
pub const FLATTEN_STEP: &str = "2B düzleme indir";

/// Input of `cad.entities.flatten` v1: the objects (points, lines, polylines and areas with their parts and holes)
/// whose vertex elevations go, or become `plane`; with `measures`, whose measures go too. An object with neither
/// to change is left as it is; nothing to change at all is `nothing_to_flatten`. Other kinds hold no elevation and
/// are passed over.
///
/// Refusals, checked in this order: `no_entities`, `invalid_uid`, `not_finite` (`plane`), `invalid_revision`,
/// `revision_conflict`, `entity_not_found`, `layer_locked` (every object on a locked layer; some: left out with the
/// warning `layer_locked`), `nothing_to_flatten`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesFlatten {
    /// The objects' persistent ids, at least one; a repeated one counts once.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The level every elevation becomes, m; absent: the elevations are removed (a vertex without one is not 0).
    /// A vertex without an elevation stays without one either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub plane: Option<f64>,
    /// Remove the measures (M) too; absent: they stay (a route's measure is not geometry).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub measures: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.flatten` v1 (its plan's too: what it would do, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesFlattened {
    /// The objects changed, by persistent id, in the input's order.
    pub changed: Vec<String>,
    /// Their slots, in the same order.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    /// The vertices whose elevation went or changed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub vertices: u32,
    /// The measures removed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub measures: u32,
    /// The lowest and highest elevation those vertices had, m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub z_max: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
