//! Changing a local project's drawing unit (docs/adr/0256 §1; plan/cad.md
//! CAD-DATA-04): `cad.units.change` names the unit it changes from and the
//! one it changes to, and what becomes of the drawing. Two operations, not
//! one: keeping the sizes changes only how lengths are read and typed (a
//! 4000 mm wall reads 4 m), keeping the numbers scales the drawing by the
//! units' ratio (it reads 4000 m). Geometry is in metres on the wire either
//! way (docs/adr/0165 §2, docs/adr/0245 §3).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::document::DrawingUnit;

/// Changes a local project's drawing unit.
pub const CAD_UNITS_CHANGE: &str = "cad.units.change";
pub const CAD_UNITS_CHANGE_VERSION: u32 = 1;

/// What becomes of the drawing when its unit changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum UnitGeometry {
    /// The sizes stay: only the unit lengths are read and typed in changes
    /// (4000 mm reads 4 m). Nothing of the drawing moves.
    Keep,
    /// The numbers stay: the drawing is scaled by the ratio of the two
    /// units about 0,0 (4000 mm reads 4000 m, a thousand times larger).
    Scale,
}

/// Input of `cad.units.change` v1: the drawing unit goes from `from` to
/// `to`, with `geometry` saying what becomes of the drawing, as one undo
/// step “Birimi değiştir” (the unit and the scaled drawing undone together).
///
/// With `scale`, everything the drawing measures in its unit is scaled by
/// one whole factor (×1000 from mm to m, ÷10 from cm to mm): every object's
/// coordinates, lengths, radii, elevations, its text, dimension, leader and
/// table heights and sizes, a paragraph's box, a curved text's curve, a
/// hatch's spacing and pattern scale, a picture's and a raster's frame; the
/// block definitions (base point, objects, attribute definitions); the home
/// view and the drawing's anchor. Angles, ratios, an insert's own scale,
/// paper sizes and a route's measures stay. A point cloud's points are in
/// its files and stay (`point_clouds_kept`); the sheet book stays
/// (`sheets_kept`); the topology tolerance and the networks' tolerances are
/// the project's precision and stay (`tolerances_kept`).
///
/// Refusals (`CommandError.code`), checked in this order: `same_unit`
/// (`from` is `to`), `invalid_revision`, `revision_conflict` (status
/// `conflict`), `unit_needs_local` (the project has a coordinate system:
/// its unit is the system's), `unit_mismatch` (`from` is not the drawing's
/// unit, which the message names), `layer_locked` (with `scale`: objects on
/// a locked layer, whose `uids` the error lists, the first 100).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnitsChange {
    /// The drawing's unit now: said, not assumed, so a script written for
    /// another drawing does not scale this one.
    pub from: DrawingUnit,
    /// The unit it changes to.
    pub to: DrawingUnit,
    /// Whether the sizes or the numbers stay.
    pub geometry: UnitGeometry,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.units.change` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnitsChanged {
    pub from: DrawingUnit,
    pub to: DrawingUnit,
    pub geometry: UnitGeometry,
    /// What the drawing's lengths were multiplied by: 1 with `keep`; with
    /// `scale` the units' ratio (1000 from mm to m, 0.1 from cm to mm).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub factor: f64,
    /// The objects scaled (0 with `keep`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The block definitions scaled (0 with `keep`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub blocks: u32,
    /// The document's revision afterwards (plan: the one it was made
    /// against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
