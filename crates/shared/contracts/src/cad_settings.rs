//! The project settings' command of the product command catalog
//! (docs/adr/0248 §5): `cad.settings.set` changes a few of the project's own
//! settings by their keys: its name and type, its coordinate systems
//! (assigned, never a transformation of the coordinates), a local project's
//! unit, the shown decimals and units, the plot scale and the drawing's
//! typeface. A user's and a device's settings are not the drawing's and
//! have no key here.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::document::{AngleUnit, AreaUnit, DrawingFont, DrawingUnit, Workspace};

/// Changes project settings by key.
pub const CAD_SETTINGS_SET: &str = "cad.settings.set";
pub const CAD_SETTINGS_SET_VERSION: u32 = 1;

/// The most decimals a length or an area is shown with.
pub const MAX_SHOWN_DECIMALS: u32 = 4;
/// The largest plot scale denominator (1:1 000 000).
pub const MAX_PLOT_SCALE: f64 = 1_000_000.0;

/// Input of `cad.settings.set` v1: the keys given change, the others stay.
/// Unknown keys are refused by the schema (a user's or a device's setting,
/// a theme, a key of another command): they are not the project's.
///
/// - `name`: the drawing's name, trimmed;
/// - `projectType`: `cad` or `gis` (`plan3d` and `disaster` cannot be chosen
///   yet);
/// - `srid`: the project's coordinate system, an EPSG code of the registry,
///   or 0 for a local project without one; it is assigned, the coordinates
///   stay as they are (CLAUDE.md §5). It takes the place of a defined system
///   (docs/adr/0168 §1); a system makes the drawing unit metres;
/// - `secondSrid`: the second system (docs/adr/0167 §1), an EPSG code of the
///   registry other than the project's own, or 0 for none; it takes the
///   place of a defined second system;
/// - `drawingUnit`: a local project's unit (docs/adr/0165 §2);
/// - `lengthDecimals`, `areaDecimals`: 0 to 4;
/// - `areaUnit`, `angleUnit`;
/// - `plotScale`: a whole denominator from 1 to 1 000 000; the texts,
///   leaders, dimensions and tables at the old general height take the new
///   one in one undo step “Yazı yüksekliklerini uydur” (docs/adr/0205 §3);
/// - `drawingFont`: the drawing's typeface.
///
/// Settings are an edit of the project, no undo step (docs/adr/0049).
///
/// Refusals (`CommandError.code`), checked in this order: `nothing_to_set`
/// (no key), `empty_name`, `invalid_project_type`, `invalid_decimals`
/// (`lengthDecimals`, then `areaDecimals`), `invalid_scale`, `unknown_srid`
/// (`srid`, then `secondSrid`: not 0 and not in the registry); then
/// `invalid_revision`, `revision_conflict` (status `conflict`);
/// `second_needs_system` (a second system for a project without one),
/// `second_is_own` (the second system the project's own), `unit_needs_local`
/// (a unit other than metres for a project with a system).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingsSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub project_type: Option<Workspace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub srid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub second_srid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_unit: Option<DrawingUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub length_decimals: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area_decimals: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area_unit: Option<AreaUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub angle_unit: Option<AngleUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plot_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_font: Option<DrawingFont>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.settings.set` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SettingsChanged {
    /// The keys whose value changed, in the input's documented order
    /// (`drawingUnit` too when a system made the unit metres).
    pub changed: Vec<String>,
    /// The annotations that took the new plot scale's height (one undo step).
    pub followed: u32,
    /// The annotations a locked layer kept at the old height.
    pub locked: u32,
    /// The document's revision afterwards (plan: the one it was made
    /// against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
