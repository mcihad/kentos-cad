//! The semantic divide command of the product command catalog
//! (docs/adr/0254 §5; plan/cad.md CAD-CMD-01, 05): points placed along an
//! object at equal parts or at a step (AutoCAD's DIVIDE and MEASURE), named
//! by the object and the division rather than the points.
//!
//! Where the points fall is the shared geometry core's (`division_points`:
//! an ellipse's on the true curve), the rule Böl has always used. Their
//! output and plan are `cad.entities.create`'s (`EntitiesCreated`,
//! `EntitiesCreatePlan`). Lengths and coordinates are metres on the wire
//! (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_region::NewLook;

/// Places points along an object.
pub const CAD_ENTITIES_DIVIDE: &str = "cad.entities.divide";
pub const CAD_ENTITIES_DIVIDE_VERSION: u32 = 1;

/// The most points a division places, and the most parts it is asked for.
pub const MAX_DIVISION_POINTS: u32 = 10_000;

/// How the object is divided.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Division {
    /// Into `count` equal parts (2 to 10 000): a point between each two, and
    /// on a closed object one at its start too.
    Parts { count: u32 },
    /// A point every `length` metres from the start (over 0).
    Step { length: f64 },
}

/// Input of `cad.entities.divide` v1: points along the object at
/// `division`, measured from its start (or its end, `fromEnd`), on
/// `layerId`, as one undo step “Böl” written by `cad.entities.create`'s
/// rules. Lines, polylines, areas, arcs, circles, ellipses and splines divide.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid`,
/// `invalid_count` (parts out of 2 to 10 000), `invalid_distance` (a step
/// not over 0 or not finite), `invalid_revision`, `revision_conflict`
/// (status `conflict`), `entity_not_found`, `not_divisible` (another kind),
/// `no_points` (a step longer than the object), `too_many_points` (over
/// 10 000), then the layer's and the look's as `cad.entities.create` checks
/// them (`layer_not_found`, `not_a_layer`, `layer_locked`, `invalid_line_weight`);
/// a hidden layer gives a `layer_hidden` warning; on the desktop also
/// `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesDivide {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// Into parts, or at a step.
    pub division: Division,
    /// The points are measured from the object's end, not its start (the
    /// tool: from the end nearer the click).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub from_end: bool,
    /// The layer the points go to.
    pub layer_id: String,
    /// How the points look and what they carry.
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: NewLook,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_division_reads_by_its_kind() {
        let input: EntitiesDivide = serde_json::from_value(json!({
            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
            "division": { "kind": "step", "length": 2.5 }, "layerId": "nokta"
        }))
        .expect("reads");
        assert_eq!(input.division, Division::Step { length: 2.5 });
        assert!(!input.from_end);
    }
}
