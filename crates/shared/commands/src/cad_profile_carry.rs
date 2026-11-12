//! `cad.profiles.carry` v1, Kataloğa taşı (plan/map-cad.md MAP-04, docs/adr/5340): the drawing's data carried from one
//! catalog (a data profile) to another as `cad.profiles.compare`'s report traces each class: a class kept, changed,
//! renamed or merged sends its objects to its new class's layer (made with its fields when the drawing has none, as
//! `cad.profiles.apply` makes it); a split class's objects go each to the new class holding its own field code; a retired
//! class's objects, and a split class's whose code no new class holds, stay where they are and are said. Values travel
//! with their objects; no value is dropped or recoded. One undo step, all or nothing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{CatalogClassChangeKind, DataProfile};

pub const CAD_PROFILES_CARRY: &str = "cad.profiles.carry";
pub const CAD_PROFILES_CARRY_VERSION: u32 = 1;

/// Input of `cad.profiles.carry` v1: the old catalog (`from`, a profile's name, or `fromProfile`, the profile itself),
/// the new one (`to` or `toProfile`), the project's file for its folder's profiles (`drawing`), the field holding an
/// object's field code (none: `Kod`). Refusals, in this order: `invalid_revision`, `revision_conflict`;
/// `unknown_profile`, `invalid_profile` (as `cad.profiles.compare`'s); `layer_locked` (a layer an object leaves or
/// goes to is locked); `layer_refused` (a receiving class's path leads to a group or into a layer); `nothing_to_set`
/// (no object to carry).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesCarry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from_profile: Option<DataProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub to_profile: Option<DataProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// What became of an old class's objects: its id, the new classes, the report's kind, how many moved and stayed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CarriedClass {
    pub from: String,
    pub to: Vec<String>,
    pub kind: CatalogClassChangeKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: u32,
}

/// Output of `cad.profiles.carry` v1: each old class with objects (the old catalog's order), the objects moved and
/// left, the layers and fields made (`layer id`, `layer id/field`), the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfilesCarried {
    pub classes: Vec<CarriedClass>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub moved: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    pub revision: String,
}
