//! Proje gezgini (docs/adr/0778; Revit's Project Browser, Archicad's Navigator): the project's levels, named views and
//! their recipes, sheets and their maps, schedules, and how they link — and, given one object, where it shows: its level,
//! the views that draw it, the sheets' maps that draw it, the schedules that list it and the tags that name it.
//! `cad.navigator.read` reads; it writes nothing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

pub const CAD_NAVIGATOR_READ: &str = "cad.navigator.read";
pub const CAD_NAVIGATOR_READ_VERSION: u32 = 1;

/// Input of `cad.navigator.read` v1 (“Proje gezgini”): the tree; with `uid`, where that object shows.
///
/// Refusals, in this order: `entity_not_found` (at `uid`), `book_unreadable`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavigatorRead {
    /// An object's persistent id; absent: no `object`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: Option<String>,
}

/// A sheet's map, by the sheet's and the item's ids.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavMap {
    pub sheet: String,
    pub item: String,
}

/// A level of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavLevel {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub elevation: f64,
    /// The objects on it (an opening on its wall's, a tag on its object's).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The named views whose recipe draws it, by stable id, in the views' order.
    pub views: Vec<String>,
    /// The maps that draw it (their view's recipe's level, else their own), in the book's order.
    pub maps: Vec<NavMap>,
}

/// A named view of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavView {
    /// Its stable id (docs/adr/0345 §2).
    pub id: String,
    pub name: String,
    /// Its recipe's level, by id; none: every level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// Its recipe's version; none without a recipe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub recipe: Option<u32>,
    /// Its recipe's scale (1:N); none: the view's box fitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<u32>,
    /// The maps linked to it, in the book's order.
    pub maps: Vec<NavMap>,
}

/// A map of a sheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavSheetMap {
    pub item: String,
    pub name: String,
    /// Its linked view, by stable id; none: its own camera.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<String>,
    /// Its linked view is gone.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub missing: bool,
    /// The level it draws, by id; none: every level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// Its scale (1:N); none for a map without a fixed view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<f64>,
}

/// A sheet of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavSheet {
    pub id: String,
    pub name: String,
    /// `A-1` … or `Diğer` (`cad.sheets.set`'s rule).
    pub series: String,
    pub maps: Vec<NavSheetMap>,
}

/// A schedule (a table reading the model) of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavSchedule {
    /// The table's persistent id.
    pub uid: String,
    /// What it lists (`openings`, `spaces`, `columns`, `walls`, `fixtures`).
    pub of: crate::ScheduleOf,
    /// The objects it lists.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
}

/// Where one object shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavObject {
    pub uid: String,
    /// Its kind (`wall`, `opening` …).
    pub kind: String,
    /// Its level, by id; none: on no level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// The named views that draw it (inside their box, on their recipe's level or every level, its category and phase
    /// not left out), in the views' order.
    pub views: Vec<String>,
    /// The maps that draw it (`cad.sheets.set`'s `shows`), in the book's order.
    pub maps: Vec<NavMap>,
    /// The schedules that list it, by persistent id, in the drawing's order.
    pub schedules: Vec<String>,
    /// The tags that name it, by persistent id, in the drawing's order.
    pub tags: Vec<String>,
}

/// Output of `cad.navigator.read` v1 (its plan's too).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NavigatorReadResult {
    /// The project's levels, in their order.
    pub levels: Vec<NavLevel>,
    /// The objects on no level.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unleveled: u32,
    /// The named views, in their order.
    pub views: Vec<NavView>,
    /// The sheets, in the book's order.
    pub sheets: Vec<NavSheet>,
    /// The schedules, in the drawing's order.
    pub schedules: Vec<NavSchedule>,
    /// With `uid`: where it shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub object: Option<NavObject>,
    /// The drawing's revision it was read at.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
