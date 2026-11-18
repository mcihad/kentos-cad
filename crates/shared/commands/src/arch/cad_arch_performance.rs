//! `cad.arch.performance` v1 (“Yangın ve performans”, docs/adr/8068; plan/bim-plan.md BIM-07): what the author declares
//! for the fire regulation and the model (Dijital Proje Yön. EK-6): an object's own fire resistance class and a door's or
//! a stair's emergency exit role; a wall style's and a door or window type's fire resistance class, which their objects
//! carry unless they have their own; a material's strength class (the model names it `Ad_Dayanım`); the building's fire
//! protection class. Each declaration keeps its basis: the author's statement or a document. Nothing is a default: a
//! value taken away (null) is missing again. One undo step “Yangın ve performans”; the drawing's geometry never changes.
//! A wall's or a slab's exterior role and a column's load-bearing role change what the drawing says of them
//! (docs/adr/8124); `exits` writes the doors' and stairs' exit roles the escape routes suggest, once confirmed.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{Declared, DeclaredBearing, DeclaredExit, DeclaredExternal};

/// The command's name.
pub const CAD_ARCH_PERFORMANCE: &str = "cad.arch.performance";
/// Its version.
pub const CAD_ARCH_PERFORMANCE_VERSION: u32 = 1;

/// What `cad.arch.performance` declares.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PerformanceOperation {
    /// Nesneler: the objects' own fire resistance class (walls, slabs, columns, beams, roofs, railings, stairs, ramps,
    /// doors, windows), exit role (doors, stairs; a roof as an escape route, BYKHY Ek-3/B 4-a) and load-bearing role
    /// (walls but curtain walls, Ek-3/B 2; docs/adr/8090). A field absent stays; null takes the object's own away (a wall
    /// then carries its style's class, a door its type's).
    #[serde(rename_all = "camelCase")]
    Objects {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<Declared>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<Declared>"))]
        fire_rating: Option<Option<Declared>>,
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<DeclaredExit>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<DeclaredExit>"))]
        exit: Option<Option<DeclaredExit>>,
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<DeclaredBearing>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<DeclaredBearing>"))]
        load_bearing: Option<Option<DeclaredBearing>>,
        /// A wall's or a slab's exterior role (docs/adr/8124, EK-6 `IsExternal`): over what the drawing says; null: the
        /// drawing's again.
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<DeclaredExternal>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<DeclaredExternal>"))]
        external: Option<Option<DeclaredExternal>>,
    },
    /// Acil çıkışlar (docs/adr/8124): the doors and stairs whose emergency exit role is written at once, the escape
    /// routes' suggestion the author confirmed: those in `yes` are exits, those in `no` are not; each the author's
    /// statement. A uid in both, none at all or one that is no door or stair is refused.
    #[serde(rename_all = "camelCase")]
    Exits {
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        yes: Vec<String>,
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        no: Vec<String>,
    },
    /// Duvar stili: a wall style's class (by its id or name); null: none.
    #[serde(rename_all = "camelCase")]
    WallStyle {
        style: String,
        #[serde(default)]
        fire_rating: Option<Declared>,
    },
    /// Kapı ve pencere tipi: a type's class (by its id or name); null: none.
    #[serde(rename_all = "camelCase")]
    OpeningType {
        #[serde(rename = "type")]
        opening_type: String,
        #[serde(default)]
        fire_rating: Option<Declared>,
    },
    /// Malzeme: a material's strength class, reaction to fire class (by its id or name; docs/adr/8091) and a roof
    /// covering's external fire performance class (`roofClass`, TS EN 13501-5; docs/adr/8134). A field absent stays;
    /// null takes it away.
    #[serde(rename_all = "camelCase")]
    Material {
        material: String,
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
        strength: Option<Option<String>>,
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<Declared>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<Declared>"))]
        reaction: Option<Option<Declared>>,
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<Declared>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<Declared>"))]
        roof_class: Option<Option<Declared>>,
    },
    /// Bina: the building's fire protection class; null: none.
    #[serde(rename_all = "camelCase")]
    Building {
        #[serde(default)]
        fire_protection_class: Option<Declared>,
    },
}

/// Input of `cad.arch.performance` v1.
///
/// Refusals, in this order: `no_entities`, `invalid_uid`, `entity_not_found`, `field_not_for_kind` (a class on a room,
/// a grid, a line …; an exit role on anything but a door, a stair or a roof, in `exits` a door or a stair; a load-bearing
/// role on anything but a wall that is no curtain wall or a column that is no post; an exterior role on anything but a
/// wall or a slab that is no covering), `uid_in_both_lists` (`exits`), `nothing_to_set` (an objects' or a material's operation that gives no field), `style_not_found`,
/// `opening_type_not_found`, `material_not_found`, `invalid_declared`, `invalid_strength`, `invalid_revision`,
/// `revision_conflict`, `layer_locked` (an object named on a locked layer: nothing is written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchPerformance {
    pub operation: PerformanceOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.arch.performance` v1, and its plan: the objects that carry the value declared now (an object's own,
/// or a type's on its walls or openings without their own; none for a material's or the building's), by persistent id
/// in the drawing's order; the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchPerformanceDone {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
