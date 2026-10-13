//! The semantic fillet and chamfer commands of the product command catalog
//! (docs/adr/0250; plan/cad.md CAD-CMD-04): a corner rounded with a radius
//! (Köşe yuvarla) or cut with two distances (Pah), named by what it is: a
//! path's vertex, two lines each keeping the side it was picked on, or every
//! corner of paths (Tüm köşeleri yuvarla, Tüm köşelere pah).
//!
//! The sides and the arc or the cut are the shared geometry core's
//! (`corner_in_path`, `fillet_lines`, `chamfer_lines`, `fillet_arc`,
//! `chamfer_line`, `all_corners`), the rule the tools have always used.
//! Lengths and coordinates are metres on the wire.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_trim::TrimPick;
use crate::entity::Entity;

/// Rounds corners.
pub const CAD_ENTITIES_FILLET: &str = "cad.entities.fillet";
pub const CAD_ENTITIES_FILLET_VERSION: u32 = 1;
/// Cuts corners.
pub const CAD_ENTITIES_CHAMFER: &str = "cad.entities.chamfer";
pub const CAD_ENTITIES_CHAMFER_VERSION: u32 = 1;

/// Which corner, or corners.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CornerPick {
    /// A polyline's or a closed area's vertex between two straight edges;
    /// an area's vertices counted part after part (docs/adr/0143), its holes
    /// not.
    Vertex {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        index: u32,
    },
    /// Two lines; each keeps the side of its pick.
    Lines { first: TrimPick, second: TrimPick },
    /// Every corner between straight edges of these polylines and areas;
    /// at least one. Kırp is always on.
    All {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
}

/// Input of `cad.entities.fillet` v1: the corner rounded with `radius`, as
/// one undo step “Köşe yuvarla”. With Kırp (`trim`, the default) the sides
/// take their new geometry and two lines get the arc as a new object from
/// the first; a vertex is rounded in its path. Without it the sides stay
/// and the arc alone is added (a radius above zero). A radius of zero joins
/// two lines at their meeting point.
///
/// With `all`, a corner that cannot be rounded (too short a side) is left
/// as it is and counted in `skipped`; an object none of whose corners can
/// be is left out with a `fillet_failed` warning at `corner.uids[i]`.
///
/// Refusals (`CommandError.code`), in this order: `no_entities` (`all`
/// without ids), `invalid_uid`, `not_finite` (the radius, a pick's x and
/// y), `invalid_distance` (a negative radius; zero without Kırp or with
/// `all`), `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found`, `layer_locked` (an object of the corner), `not_a_corner`
/// (a vertex out of its path or not between straight edges, two lines that are
/// one or parallel, an object of another kind), `fillet_failed` (the
/// radius does not fit; with `all`, none fits).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesFillet {
    pub corner: CornerPick,
    /// The arc's radius, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: f64,
    /// Kırp: the sides are cut back to the arc (the default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trim: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Input of `cad.entities.chamfer` v1: the corner cut `distance` along its
/// first side and `second` (absent: the same) along its other, as one undo
/// step “Pah”; Kırp, `all` and the refusals as `cad.entities.fillet`'s, the
/// codes `chamfer_failed` and `invalid_distance` (a negative distance; both
/// zero without Kırp or with `all`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesChamfer {
    pub corner: CornerPick,
    /// The cut's distance along the first side (a vertex's: the edge before it), metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    /// Along the other side; absent: `distance`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub second: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trim: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.fillet` v1 and `cad.entities.chamfer` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCornered {
    /// The objects that took new geometry.
    pub changed: Vec<String>,
    /// The arc or the cut, when it is a new object.
    pub created: Vec<String>,
    /// The corners rounded or cut, and those left as they were (`all`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub done: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.fillet` or `.chamfer` would write (plan mode); nothing
/// is written. Execute against the plan's revision writes exactly this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCornerPlan {
    pub changed: Vec<Entity>,
    pub created: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub done: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_corner_reads_by_its_kind() {
        let f: EntitiesFillet = serde_json::from_value(json!({
            "corner": { "kind": "vertex", "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "index": 2 },
            "radius": 1.5
        }))
        .expect("reads");
        assert_eq!(f.trim, None);
        let c: EntitiesChamfer = serde_json::from_value(json!({
            "corner": { "kind": "lines",
                "first": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "at": { "x": 1.0, "y": 0.0 } },
                "second": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60", "at": { "x": 0.0, "y": 1.0 } } },
            "distance": 1.0, "second": 2.0, "trim": false
        }))
        .expect("reads");
        assert!(matches!(c.corner, CornerPick::Lines { .. }));
    }
}
