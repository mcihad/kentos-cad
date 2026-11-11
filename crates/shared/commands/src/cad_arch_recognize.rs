//! Planı tanı (plan/architecture-advanced.md M48, docs/adr/8048): `cad.arch.recognize` reads an exploded plan — an
//! office's DWG or DXF whose walls, doors and windows came in as plain lines and arcs on their layers — back as KentOS
//! walls with their doors, windows and bare openings, one undo step “Planı tanı”. The geometry is
//! `kentos_geometry_core::ops::plan_recognition`'s; the desktop's handler is
//! `crates/native/application/src/arch_recognize.rs`; it passes the shared cases in
//! `fixtures/commands/v1/cad.arch.recognize.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::ArchInfo;
#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Entity;

pub const CAD_ARCH_RECOGNIZE: &str = "cad.arch.recognize";
pub const CAD_ARCH_RECOGNIZE_VERSION: u32 = 1;
/// The thinnest wall read, metres, when none is given.
pub const DEFAULT_RECOGNIZE_MIN_WIDTH: f64 = 0.05;
/// The thickest wall read, metres, when none is given.
pub const DEFAULT_RECOGNIZE_MAX_WIDTH: f64 = 0.6;
/// The widest opening read, metres, when none is given.
pub const DEFAULT_RECOGNIZE_MAX_OPENING: f64 = 3.0;
/// Lines this many degrees apart are parallel, when none is given.
pub const DEFAULT_RECOGNIZE_ANGLE: f64 = 1.0;

/// What a layer's line work is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PlanRole {
    /// Duvar: its lines are walls' faces, jambs and components' lines.
    Wall,
    /// Kapı: its arcs are doors' swings, its lines their leaves and frames.
    Door,
    /// Pencere: its lines along a wall's gap make it a window.
    Window,
    /// Kolon: its closed outlines are columns a wall runs through.
    Column,
    /// Yok sayılır: not read (axes, dimensions, texts, furniture).
    Ignore,
}

/// A layer and what its line work is read as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlanLayerRole {
    pub layer_id: String,
    pub role: PlanRole,
}

/// Input of `cad.arch.recognize` v1: the plan's objects (a window selection over it); each layer's role (absent: from
/// its name — duvar, perde, bölme, wall a wall's; kapı, door a door's; pencere, window, glaz a window's; kolon, column a
/// column's; tarama, ölçü, yazı, aks, mobilya and the others none; when no layer is a wall's, every layer not named is);
/// the walls' architectural information (their storey, Mevcut for a tadilat's existing building).
///
/// The walls go on `wallLayerId`, else their first line's layer; the doors and windows on `openingLayerId`, else the
/// first layer of the door's or window's role, a bare opening its wall's. A door's type is Kapı 80 or 90 (by its
/// width), İki kanatlı kapı 140 with two leaves; a window's Pencere 120; a bare opening's Boşluk 100; each as wide as
/// read. The sources read whole are taken away unless kept; the others stay and are said.
///
/// Refusals, in this order: `no_entities`, `invalid_uid`, `entity_not_found`, `layer_locked` (every source locked),
/// `layer_not_found` (a role's layer), `not_finite`, `recognize_option` (a width not above zero, the thinnest not
/// below the thickest, an opening not above zero, an angle outside 0–45°), `level_not_found` (`arch.level`),
/// `plan_unrecognized` (no wall read), `invalid_wall`, `opening_off_wall`, the layers written to's `layer_not_found`,
/// `not_a_layer`, `layer_locked`, then `invalid_revision`, `revision_conflict`. Warnings: `layer_locked` (sources
/// skipped), `layer_hidden` (a layer written to), `plan_unrecognized` (objects not read, by why).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchRecognize {
    /// The plan's objects' persistent ids, at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// Layers' roles over those their names give.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PlanLayerRole>>", optional))]
    pub roles: Vec<PlanLayerRole>,
    /// Every new wall's architectural information: its storey, its tadilat durumu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall_layer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opening_layer_id: Option<String>,
    /// The thinnest wall, metres. Absent: 0.05 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min_width: Option<f64>,
    /// The thickest wall, metres. Absent: 0.6 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_width: Option<f64>,
    /// The widest opening, metres; a wider gap ends the wall. Absent: 3 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_opening: Option<f64>,
    /// Degrees two lines may be apart and be parallel. Absent: 1°.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: Option<f64>,
    /// Kaynağı koru: the sources stay. Absent: those read whole go.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub keep_sources: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.arch.recognize` v1: the walls and openings written (in the order the plan reads them: walls by
/// their first line, openings by wall and along it), how many doors, windows and bare openings, the sources taken
/// away and those left.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchRecognized {
    pub walls: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub openings: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub doors: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub windows: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub bare: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub left: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.arch.recognize` would write (plan mode): the walls and openings as they would be stored (their `id` 0,
/// an opening's host its wall's provisional id), the sources that would go and those left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchRecognizePlan {
    pub walls: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Entity>>", optional))]
    pub openings: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub left: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
