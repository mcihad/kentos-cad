//! The semantic divide command of the product command catalog
//! (docs/adr/0254 §5; plan/cad.md CAD-CMD-01, 05): points or a block's
//! inserts placed along an object at equal parts or at a step (AutoCAD's
//! DIVIDE and MEASURE), named by the object and the division rather than the
//! points; the step's first distance, the object's own ends and the inserts'
//! alignment are the input's (docs/adr/0299).
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
    Parts {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        count: u32,
    },
    /// A point every `length` metres from the start (over 0).
    Step {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: f64,
    },
}

/// Which of an open object's own ends also get a place (docs/adr/0299 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DivideEnds {
    /// Neither (AutoCAD's DIVIDE and MEASURE).
    #[default]
    None,
    /// The object's start.
    Start,
    /// The object's end.
    End,
    /// Both.
    Both,
}

impl DivideEnds {
    fn is_none(&self) -> bool {
        *self == DivideEnds::None
    }
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

fn one() -> f64 {
    1.0
}

fn is_one(v: &f64) -> bool {
    *v == 1.0
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A block's inserts at the places instead of points (AutoCAD's Block option, docs/adr/0299 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DivideBlock {
    /// The block's name in the drawing.
    pub name: String,
    /// Each insert turned by the object's direction at its place (the way the object runs from its start), on
    /// top of `rotation`; true by default.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub align: bool,
    /// The inserts' scale (above zero); 1 by default.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    /// The inserts' own turn, radians counter-clockwise; 0 by default.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
}

/// Input of `cad.entities.divide` v1: points along the object at
/// `division`, measured from its start (or its end, `fromEnd`), on
/// `layerId`, as one undo step “Böl” written by `cad.entities.create`'s
/// rules. Lines, polylines, areas, arcs, circles, ellipses and splines divide.
/// With `block` the places get the block's inserts (aligned to the object
/// by default); with `start` a step's first place is that far from the
/// measuring end (0: at it); `ends` also marks the open object's own start or
/// end where no place is already.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid`,
/// `invalid_count` (parts out of 2 to 10 000), `invalid_distance` (a step
/// not over 0 or not finite), `not_finite`, `invalid_distance` and
/// `start_needs_step` for `start` (below 0; with parts), `not_finite` and
/// `invalid_scale` for the block's numbers, `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`,
/// `not_divisible` (another kind), `unknown_block`, `no_points` (a step or
/// first distance longer than the object), `too_many_points` (over
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
    /// A step's first place, metres from the measuring end (0 and above); absent: at the step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub start: Option<f64>,
    /// The open object's own ends that also get a place.
    #[serde(default, skip_serializing_if = "DivideEnds::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<DivideEnds>"))]
    pub ends: DivideEnds,
    /// A block's inserts instead of points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<DivideBlock>,
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
        assert_eq!(
            (input.start, input.ends, input.block),
            (None, DivideEnds::None, None)
        );
    }

    #[test]
    fn a_block_aligns_at_scale_one_by_default() {
        let input: EntitiesDivide = serde_json::from_value(json!({
            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "division": { "kind": "parts", "count": 4 },
            "layerId": "nokta", "block": { "name": "Direk" }, "ends": "both", "start": 0
        }))
        .expect("reads");
        let block = input.block.expect("a block");
        assert!(block.align && block.scale == 1.0 && block.rotation == 0.0);
        assert_eq!(input.ends, DivideEnds::Both);
        assert_eq!(input.start, Some(0.0));
    }
}
