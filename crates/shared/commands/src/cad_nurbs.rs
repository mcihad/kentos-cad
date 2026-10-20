//! The semantic NURBS command of the product command catalog
//! (docs/adr/0259 §3): one NURBS curve on a named layer, by its control
//! points (Denetim noktalı eğri) or through points it passes (Geçiş noktalı
//! eğri), open or closed, as one undo step “Ekle”.
//!
//! Its input is what the user gave, not the result: the knots of a curve by
//! its control points, the wrapped control points of a closed one, and the
//! whole curve through fit points (chord-length parameters, averaged knots,
//! the collocation system solved) are computed by the shared geometry core
//! (`geom::nurbs_build`) inside the command, so the tool, Python and an
//! agent giving the same points get the same curve, bit for bit.
//!
//! Coordinates are metres on the wire (x east (Y), y north (X)), whatever
//! unit the project shows (docs/adr/0165 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::{Entity, Vec2};

/// Writes one NURBS curve on a named layer.
pub const CAD_NURBS_CREATE: &str = "cad.nurbs.create";
pub const CAD_NURBS_CREATE_VERSION: u32 = 1;
/// The most fit points a closed curve is built through (its system is solved whole).
pub const MAX_CLOSED_FIT_POINTS: usize = 1000;

/// How the curve is given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NurbsBy {
    /// Denetim noktalı: the control points, the curve pulled towards them,
    /// passing through the first and the last of an open one.
    #[serde(rename_all = "camelCase")]
    Control {
        /// The polynomial degree, 1 to 11; at least one less than the
        /// control points of an open curve.
        #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 11)))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        degree: u32,
        /// The control points, in order.
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        ctrl: Vec<Vec2>,
        /// Each control point's weight, above 0. Absent: all 1.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        weights: Option<Vec<f64>>,
        /// The knot vector of an open curve, `ctrl.len() + degree + 1`
        /// numbers, never decreasing. Absent: clamped and uniform (the
        /// curve starts at the first control point and ends at the last). A
        /// closed curve's knots are the command's: give none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        knots: Option<Vec<f64>>,
    },
    /// Geçiş noktalı: the points the curve passes through, in order.
    #[serde(rename_all = "camelCase")]
    Fit {
        /// The points, two or more (three or more for a closed curve); no
        /// two in a row at the same place.
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        pts: Vec<Vec2>,
        /// The degree, 1 to 11. Absent: 3. Lowered to one less than the
        /// points when they are fewer.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 11)))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        degree: Option<u32>,
    },
}

/// Input of `cad.nurbs.create` v1: one NURBS curve on a named layer,
/// written as one undo step. The curve tools give what was drawn: the
/// control points or the points it passes, the degree, whether it is
/// closed; the command computes the curve (TODOS.md CMD-07: it reads no
/// active layer, colour or view).
///
/// An open curve by control points keeps them as given, with the knots
/// given or clamped and uniform. A closed one wraps its first `degree`
/// control points after the last and takes uniform knots: a periodic
/// curve, as smooth where it closes as anywhere. A curve through points
/// takes chord-length parameters and averaged knots (Piegl and Tiller's
/// global interpolation; a closed one periodic, its knots the parameters,
/// or their midpoints when the degree is even) and passes through every
/// point; the points are kept with it as its fit points.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite` (a
/// coordinate, weight or knot, at its place), `coincident_points` (two
/// points in a row at one place; a closed curve's last and first too),
/// `invalid_nurbs` (a degree, count or knot vector that makes no curve),
/// `invalid_line_weight`, `invalid_revision`, `revision_conflict` (status
/// `conflict`), `layer_not_found`, `not_a_layer`, `layer_locked`; on the
/// desktop also `slots_exhausted`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NurbsCreate {
    /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// The curve: by control points or through points.
    pub by: NurbsBy,
    /// Kapalı: the curve closes on itself, smoothly. Absent: open.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub closed: bool,
    /// Colour override (`EntityBase.color`). Absent: the layer's colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Its own line weight, paper mm (0 to 100; docs/adr/0139). Absent: the layer's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    /// Its own line type (docs/adr/0260 §2): what the tools give a new object
    /// from the current line type. Absent: its layer's (katmana göre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<crate::layer::LineType>,
    /// GIS attributes, text in v1. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// The text shown beside it (`EntityBase.label`). Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// Its own symbol, a library item's id (`EntityBase.symbol`; docs/adr/0176). Absent: the layer's style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    /// When given and the document is no longer at it, nothing is written and
    /// the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.nurbs.create` v1: the object written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NurbsCreated {
    /// Its persistent id (UUIDv7, docs/adr/0014).
    pub uid: String,
    /// Its slot in the open document (`Entity.id`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: u32,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.nurbs.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NurbsPlan {
    /// The curve as it would be stored. Its `id` is 0: the slot is given when it is written.
    pub entity: Entity,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
