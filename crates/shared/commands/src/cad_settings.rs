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
use crate::document::{
    AngleUnit, AreaUnit, CadWork, DrawingFont, DrawingUnit, LengthStyle, Workspace,
};

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
///   yet); a project without a type chooses once; an existing type cannot change (ADR 0622);
/// - `cadWork`: what a CAD project is drawn for (docs/adr/0801), `architecture`, `map` or `detail`: its KentOS
///   program and its ribbon. Only a CAD project has one (with `projectType: cad` a project without a type takes
///   both); a CAD project without a work chooses once, an existing work cannot change;
/// - `srid`: the project's coordinate system, an EPSG code of the registry,
///   or 0 for a local project without one; it is assigned, the coordinates
///   stay as they are (CLAUDE.md §5). It takes the place of a defined system
///   (docs/adr/0168 §1); a system makes the drawing unit metres;
/// - `secondSrid`: the second system (docs/adr/0167 §1), an EPSG code of the
///   registry other than the project's own, or 0 for none; it takes the
///   place of a defined second system;
/// - `drawingUnit`: a local project's unit (docs/adr/0165 §2), mm, cm, m,
///   in or ft (docs/adr/0262 §1);
/// - `lengthStyle`: `decimal`, or in an inch or foot drawing
///   `architectural`, `engineering`, `fractional` (docs/adr/0262 §2); a
///   metric unit takes the style back to decimal;
/// - `lengthDecimals`, `areaDecimals`: 0 to 4;
/// - `areaUnit`, `angleUnit`;
/// - `plotScale`: a whole denominator from 1 to 1 000 000; the texts,
///   leaders, dimensions and tables at the old general height take the new
///   one in one undo step “Yazı yüksekliklerini uydur” (docs/adr/0205 §3);
/// - `drawingFont`: the drawing's typeface;
/// - `arch`: the project's levels and the detail its walls are drawn at
///   (docs/adr/0339 §3), whole: the given settings take the place of the
///   old ones, an empty object takes them away; with them `activeLevel` (a level's id, null for Katsız) and
///   `levelView` (`active`, `below`; `all` is no setting), the level and view the drawing opens with
///   (docs/adr/0720 §1);
/// - `heights`: what the drawing's elevations are (docs/adr/0519 §1), whole: `ellipsoidal`, or `orthometric` with
///   the vertical system (`verticalSrid`, one KentOS knows) and the geoid model (`geoid`: name, SHA-256 of its
///   file); `{"kind": "unknown"}` takes them away. Assigned: no elevation changes;
/// - `coordinateEpoch`: the coordinates' epoch, a decimal year from 1900 to 2100; 0 takes it away.
///
/// Settings are an edit of the project, no undo step (docs/adr/0049).
///
/// Refusals (`CommandError.code`), checked in this order: `nothing_to_set`
/// (no key), `empty_name`, `invalid_project_type`, `invalid_decimals`
/// (`lengthDecimals`, then `areaDecimals`), `invalid_scale`, `unknown_srid`
/// (`srid`, then `secondSrid`: not 0 and not in the registry), `invalid_arch`
/// (two levels of one id or name, a level's elevation or height, an `activeLevel` the levels lack, an active level or
/// view without levels), `invalid_heights`, `invalid_epoch`; then
/// `invalid_revision`, `revision_conflict` (status `conflict`);
/// `project_type_locked` (an existing project type changed),
/// `cad_work_needs_cad` (a work for a project that is not a CAD project), `cad_work_locked` (an existing work
/// changed),
/// `second_needs_system` (a second system for a project without one),
/// `second_is_own` (the second system the project's own), `unit_needs_local`
/// (a unit other than metres for a project with a system),
/// `style_needs_imperial` (a feet-and-inches style for a metric unit),
/// `level_in_use` (a level taken away that a wall, room, room separator, column, grid, stair or ramp names, or a
/// sheet's map shows: said by its name with how many of each, the objects' `uids` given; docs/adr/0720 §3),
/// `heights_need_system` (ellipsoidal heights or an epoch for a project without a system).
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
    /// What a CAD project is drawn for (docs/adr/0801): its KentOS program and its ribbon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cad_work: Option<CadWork>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub second_srid: Option<u32>,
    /// The map's system (docs/adr/0583 §3); 0 takes it away (the project's own). No coordinate changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub map_srid: Option<u32>,
    /// Otomatik seçim (docs/adr/0622 §5): true, a click and a box select any shown layer's objects; false, only
    /// the active layer's. No object changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub auto_select: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_unit: Option<DrawingUnit>,
    /// How lengths are written (docs/adr/0262 §2): decimal, or architectural,
    /// engineering and fractional feet and inches in an inch or foot drawing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub length_style: Option<LengthStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub length_decimals: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub area_decimals: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area_unit: Option<AreaUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub angle_unit: Option<AngleUnit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub plot_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_font: Option<DrawingFont>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    /// Whole architectural project preferences, including optional room name/usage lists and level prefixes, and the
    /// level and view the drawing opens with (`activeLevel`, `levelView`, docs/adr/0720).
    /// Copy existing levels/detail/phase when changing suggestions; this is not a nested patch.
    /// These suggestions do not rename or renumber existing rooms. Settings retain the existing history policy.
    pub arch: Option<crate::ArchSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub heights: Option<crate::HeightReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "year")))]
    pub coordinate_epoch: Option<f64>,
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
    /// (`drawingUnit` too when a system made the unit metres, `lengthStyle`
    /// when a metric unit took it back to decimal).
    pub changed: Vec<String>,
    /// The annotations that took the new plot scale's height (one undo step).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub followed: u32,
    /// The annotations a locked layer kept at the old height.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub locked: u32,
    /// The document's revision afterwards (plan: the one it was made
    /// against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
