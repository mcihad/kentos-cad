//! The associative dimension's command of the product command catalog
//! (docs/adr/0249 §3): a dimension's points bound to new places on objects,
//! or freed, as one undo step “Ölçüyü yeniden bağla”.
//!
//! Its input names places, not coordinates: the command reads each point
//! from its place on the object now (`ops::subref::point`), so a tool,
//! Python and an agent binding the same place get the same point.
//! Coordinates are metres on the wire (x east (Y), y north (X)).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Entity;
use crate::subref::SubPlace;

/// Binds a dimension's points to places on objects again.
pub const CAD_DIMENSIONS_REASSOCIATE: &str = "cad.dimensions.reassociate";
pub const CAD_DIMENSIONS_REASSOCIATE_VERSION: u32 = 1;

/// A dimension's measured point: `a`, `b` or `c` (its third point: an
/// angle's vertex, a radius's centre and the like).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionPoint {
    A,
    B,
    C,
}

impl DimensionPoint {
    /// Its name as `DimensionRefs` and the wire say it.
    pub fn name(self) -> &'static str {
        match self {
            DimensionPoint::A => "a",
            DimensionPoint::B => "b",
            DimensionPoint::C => "c",
        }
    }
}

/// A place on an object named by the object's persistent id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlaceOn {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub place: SubPlace,
}

/// One point's new binding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointBinding {
    pub point: DimensionPoint,
    /// The place the point follows from now on; null frees it (it keeps its
    /// coordinates and follows nothing).
    pub to: Option<PlaceOn>,
}

/// Input of `cad.dimensions.reassociate` v1: a dimension's points bound to
/// places on objects, or freed, as one undo step “Ölçüyü yeniden bağla”. A
/// bound point moves to its place on the object now and follows it from
/// then on (docs/adr/0249 §3); a broken reference given a place is live
/// again. A binding that leaves a point and its reference as they are
/// changes nothing; when none changes, nothing is written (`changed` 0).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid` (the
/// dimension's id, then each place's object's, in order), `no_entities` (no
/// binding), `invalid_refs` (a point bound twice, a place's index too big),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (the dimension, then each place's object, at
/// `points[i].to.uid`), `not_a_dimension`, `layer_locked` (the dimension's
/// layer), `invalid_refs` (a point the dimension's kind does not measure:
/// `c` of a dimension without one; a place bound to the dimension itself),
/// `no_such_place` (a place its object does not have, at
/// `points[i].to.place`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsReassociate {
    /// The dimension's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The points bound or freed, at least one, each once.
    pub points: Vec<PointBinding>,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.dimensions.reassociate` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsReassociated {
    /// The points whose binding or coordinates changed (an undo step was
    /// written when any did).
    pub changed: u32,
    /// The dimension's measured value now, in its unit (`unit`).
    pub value: f64,
    /// "length" (metres), "angle" (radians), "percent" or "coordinate".
    pub unit: String,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.dimensions.reassociate` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsReassociatePlan {
    /// The dimension as execute would write it.
    pub dimension: Entity,
    /// The points whose binding or coordinates would change.
    pub changed: u32,
    /// Its measured value then, and the value's unit, as in the output.
    pub value: f64,
    pub unit: String,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_point_bound_and_a_point_freed_read() {
        let input: DimensionsReassociate = serde_json::from_value(json!({
            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
            "points": [
                { "point": "a", "to": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60",
                    "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 1 } } },
                { "point": "b", "to": null }
            ]
        }))
        .expect("reads");
        assert_eq!(input.points[0].point, DimensionPoint::A);
        assert_eq!(input.points[1].to, None);
        assert_eq!(input.points[1].point.name(), "b");
    }
}
