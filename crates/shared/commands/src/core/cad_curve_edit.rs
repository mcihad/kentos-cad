//! The curve edit command of the product command catalog (docs/adr/0303; AutoCAD's PEDIT and SPLINEDIT, Eğriyi
//! düzenle): a polyline or a NURBS curve named by its persistent id, edited by what changes rather than by its new
//! numbers; the command computes the curve (the shared core's `geom::nurbs_build`, `geom::nurbs`).
//!
//! Lengths are metres on the wire, whatever unit the project shows (docs/adr/0165 §2). Points and control points
//! are numbered from 0 in the curve's order; a closed (periodic) curve's control points as they are drawn, its
//! wrapped copies not counted.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;

/// Edits a polyline or a NURBS curve.
pub const CAD_CURVES_EDIT: &str = "cad.curves.edit";
pub const CAD_CURVES_EDIT_VERSION: u32 = 1;

/// What changes (AutoCAD PEDIT's Close, Open, Decurve, Spline; SPLINEDIT's Close, Open, Fit data, Edit vertex,
/// Refine, convert to Polyline).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CurveEdit {
    /// Kapat: a polyline's last vertex joins its first (its closing edge's bulge and widths as stored); a NURBS
    /// curve closes smoothly: one with fit points drawn again through them, closed, one by control points with its
    /// first `degree` control points wrapped (periodic). Its end tangents go.
    Close,
    /// Aç: a closed polyline's closing edge goes; a closed NURBS curve opens: one with fit points drawn again
    /// through them, open, one by control points clamped on its own control points (the wrapped copies dropped).
    Open,
    /// Düzleştir: every arc edge of a polyline straight.
    Decurve,
    /// Eğriye çevir: a polyline becomes a NURBS curve of `degree` (absent: 3; lowered to fit the points): by its
    /// vertices as control points, or with `through` through them (its fit points). Arc edges are taken by their
    /// vertices.
    ToSpline {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 11)))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        degree: Option<u32>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        through: bool,
    },
    /// A fit point added before `index` (its count: after the last); the curve drawn again through them.
    AddFit {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        index: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// The fit point `index` moved to `at`; the curve drawn again.
    MoveFit {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        index: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// The fit point `index` removed; the curve drawn again.
    RemoveFit {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        index: u32,
    },
    /// An open curve's start and end tangent directions through its fit points (absent: free); the curve drawn
    /// again leaving and arriving along them.
    Tangents {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        start: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        end: Option<Vec2>,
    },
    /// The control point `index` moved to `at`. The fit points go (the curve is its control points now).
    MoveControl {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        index: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// The control point `index` weighs `weight` (above 0). The fit points go.
    Weight {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        index: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        weight: f64,
    },
    /// The control point `index` removed; the knots made uniform again (an open curve clamped). The fit points go.
    RemoveControl {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        index: u32,
    },
    /// A knot inserted at the parameter `at`, inside the curve's domain: one more control point, the curve itself
    /// unchanged (Refine). The fit points go.
    InsertKnot {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        at: f64,
    },
    /// Çoklu çizgiye çevir: a NURBS curve becomes a polyline whose every chord stays within `tolerance` of it (its
    /// ends exact), closed when the curve is.
    ToPolyline {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        tolerance: f64,
    },
}

/// Input of `cad.curves.edit` v1: the polyline or NURBS curve `uid` edited by `edit`, as one undo step “Eğriyi
/// düzenle”. The object keeps its layer, look and data; its kind changes with Eğriye çevir and Çoklu çizgiye
/// çevir. The answer is `cad.entities.edit`'s.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid`, `not_finite` (a point, weight, knot or
/// tolerance), `invalid_weight` (not above 0), `invalid_tolerance` (not above 0), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`, `layer_locked`, `not_a_curve` (the edit is not
/// for this kind, or the curve is already so), `multi_part`, `no_fit_points` (a fit edit on a curve without
/// them), `invalid_index`, `invalid_knot` (outside the domain or a full knot), `coincident_points` and
/// `invalid_nurbs` (the curve the edit makes), `too_few_points`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CurvesEdit {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// What changes.
    pub edit: CurveEdit,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
