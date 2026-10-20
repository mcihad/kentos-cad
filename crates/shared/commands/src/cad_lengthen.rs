//! The semantic lengthen command of the product command catalog
//! (docs/adr/0298; AutoCAD's LENGTHEN, Uzat-kısalt): an object's length
//! changed at one end, named by the object, the end and how the length
//! changes rather than by the new geometry.
//!
//! The new shape is the shared geometry core's (`ops::lengthen::lengthen`):
//! a line, an arc, an elliptical arc or an open polyline; shortening cuts the
//! path at the new length, lengthening continues its end edge along its own
//! direction, circle or ellipse. Lengths are metres on the wire, whatever unit
//! the project shows (docs/adr/0165 §2); angles radians.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;

/// Changes an object's length at one end.
pub const CAD_ENTITIES_LENGTHEN: &str = "cad.entities.lengthen";
pub const CAD_ENTITIES_LENGTHEN_VERSION: u32 = 1;

/// Which end moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LengthenEnd {
    /// The first point: a line's `a`, an arc's start, a polyline's first vertex.
    Start,
    /// The last point.
    End,
}

/// How the length changes (AutoCAD LENGTHEN's DElta, Percent, Total and DYnamic).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LengthenChange {
    /// Fark: the length changed by `length` (negative shortens), or an arc's
    /// included angle by `angle` radians; exactly one of them.
    Delta {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: Option<f64>,
    },
    /// Yüzde: the new length as this percentage of the current one (above zero).
    Percent {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
        percent: f64,
    },
    /// Toplam: the new total `length`, or an arc's new included `angle`
    /// radians; exactly one of them.
    Total {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: Option<f64>,
    },
    /// Dinamik: the moving end as near `to` as the object's own path lets it:
    /// past the end along the end edge's line, circle or ellipse, before it
    /// cut back to the nearest point.
    Dynamic {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        to: Vec2,
    },
}

/// Input of `cad.entities.lengthen` v1: the object's length changed at `end`
/// by `change`, as one undo step “Uzat-kısalt”. A line, an arc, an
/// elliptical arc or an open single-part polyline; a closed polyline, an
/// area, a circle, a whole ellipse and a curve are refused
/// (`not_lengthenable`). Lengthening keeps every vertex and edge but the
/// moving end's: a polyline's edges keep their widths, and the moving end's
/// elevation and measure (M) run on along the end edge by length
/// (extrapolated from its two ends). Shortening cuts the path; the kept
/// vertices keep theirs, the new end takes the cut edge's by length. An
/// associative dimension's point on the object follows it.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid`,
/// `not_finite` (the change's numbers), `invalid_change` (Fark or Toplam
/// with neither or both of `length` and `angle`), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`,
/// `layer_locked`, `not_lengthenable`, `multi_part`, `angle_needs_arc`
/// (an angle for other than an arc), `invalid_length` (the new length not
/// above zero), `arc_closes` (an arc or an arc edge would close on itself).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesLengthen {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The end that moves.
    pub end: LengthenEnd,
    /// How the length changes.
    pub change: LengthenChange,
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
    fn the_changes_read_by_their_kind() {
        let input: EntitiesLengthen = serde_json::from_value(json!({
            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "end": "end",
            "change": { "kind": "delta", "length": -1.5 }
        }))
        .expect("reads");
        assert_eq!(
            input.change,
            LengthenChange::Delta {
                length: Some(-1.5),
                angle: None
            }
        );
        let dynamic: LengthenChange =
            serde_json::from_value(json!({ "kind": "dynamic", "to": { "x": 1.0, "y": 2.0 } }))
                .expect("reads");
        assert!(matches!(dynamic, LengthenChange::Dynamic { .. }));
    }
}
