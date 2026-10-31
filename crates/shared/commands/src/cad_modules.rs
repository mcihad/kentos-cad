//! The room module commands (docs/adr/0775, ADV-05): a module defined from a bathroom's or a room's objects, placed
//! again as linked instances, its source updated from one instance and pushed to the others, an instance's objects kept
//! as local exceptions or given back, an instance detached, a module renamed or removed (`cad.modules.edit`); and the
//! modules read with their instances' state (`cad.modules.query`, read-only). Every write is one undo step “Oda
//! modülü”; its plan is the preview: what is made and changed, which instances update, which are skipped and why.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{ArchModule, ArchPhase, ModuleNumbering, Vec2};

pub const CAD_MODULES_EDIT: &str = "cad.modules.edit";
pub const CAD_MODULES_EDIT_VERSION: u32 = 1;
pub const CAD_MODULES_QUERY: &str = "cad.modules.query";
pub const CAD_MODULES_QUERY_VERSION: u32 = 1;

/// What `cad.modules.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ModulesOperation {
    /// Modül tanımla: the objects `uids` (walls, openings, spaces, fixtures, dimensions, tags, texts on one level) become
    /// the module `name`'s definition, `base` its base point; they stay where they are as its first instance. An object
    /// they lean on outside them (an opening's wall, a space's sources, a dimension's points, a tag's object) refuses it.
    Define {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        base: Vec2,
    },
    /// Modül yerleştir: a new instance of `module` (its id or name), its base at `at`, mirrored across its own y axis
    /// when asked, then turned `rotation` radians counter-clockwise; on `level` (absent: none), in `phase`, its
    /// numbers wrapped by `numbering`.
    Place {
        module: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        #[cfg_attr(feature = "ts", ts(optional))]
        rotation: Option<f64>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        mirror: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        level: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        phase: Option<ArchPhase>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        numbering: Option<ModuleNumbering>,
    },
    /// Kaynağı güncelle: the instance `from`'s objects (its local exceptions aside) become the definition, its version
    /// one up; the other linked instances (`instances`, none: all) take it, each keeping its exceptions and its
    /// annotations' places. An instance with an object on a locked layer stays at its version (skipped).
    Update {
        module: String,
        from: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        instances: Vec<String>,
    },
    /// Yerel istisna yap: these objects of an instance keep their own values; a source update passes them by.
    Except {
        module: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Kaynağa döndür: these objects of an instance give up their own values and take the definition again.
    Restore {
        module: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Bağımsızlaştır: the instance leaves the module; its objects stay as they are, following nothing.
    Detach { module: String, instance: String },
    /// Yeniden adlandır.
    Rename { module: String, name: String },
    /// Kaynağı sil: the definition goes; instances keep their objects as they last were, their link broken
    /// (`sourceGone`) until detached. A module with no instances goes whole.
    Remove { module: String },
}

/// Input of `cad.modules.edit` v1.
///
/// Refusals: `invalid_input`, `invalid_uid`, `entity_not_found`, `layer_locked`, `level_not_found`, `module_exists`,
/// `module_not_found`, `module_removed`, `module_kind`, `module_outside`, `module_levels`, `module_member_taken`,
/// `module_instance_not_found`, `module_member_missing`, `not_in_module`, `not_finite`, `nothing_to_set`,
/// `invalid_revision`, `revision_conflict`. Warnings: `module_instance_skipped`, `module_exception_kept`,
/// `module_member_gone`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModulesEdit {
    pub operation: ModulesOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Why an instance did not take a source update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ModuleSkipWhy {
    /// One of its objects is on a locked layer: it stays at its version.
    Locked,
}

/// An instance left at its version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleSkipped {
    pub instance: String,
    pub why: ModuleSkipWhy,
}

/// An object a source update passed by: kept as a local exception, or gone from the drawing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleConflict {
    pub instance: String,
    pub key: String,
    /// Its object; none when it is gone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    pub why: ModuleConflictWhy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ModuleConflictWhy {
    /// Its object keeps its own values (a local exception).
    Exception,
    /// Its object was deleted from the drawing.
    Gone,
}

/// What a write did, or would do.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleReport {
    /// The instances at the new version, in their order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub updated: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ModuleSkipped>>", optional))]
    pub skipped: Vec<ModuleSkipped>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ModuleConflict>>", optional))]
    pub conflicts: Vec<ModuleConflict>,
    /// The definition's members whose values change, by key.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub changed_keys: Vec<String>,
}

/// Output of `cad.modules.edit` (and its plan): the module as it is after, the objects made and changed, the report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModulesEdited {
    /// The module after; none once removed whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub module: Option<ArchModule>,
    /// The instance made (Yerleştir, Tanımla) or the one the operation named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub instance: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub created: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub changed: Vec<String>,
    #[serde(default)]
    pub report: ModuleReport,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.modules.query` v1 (read-only): the modules (one: `module`), or the module and instance an object
/// (`uid`) belongs to.
///
/// Refusals: `module_not_found`, `invalid_uid`, `entity_not_found`, `not_in_module`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModulesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub module: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: Option<String>,
}

/// An instance as `cad.modules.query` reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleInstanceRow {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    /// At an older version than its module's.
    pub stale: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The objects kept as local exceptions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub exceptions: Vec<String>,
    /// The keys whose objects are gone from the drawing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub missing: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<crate::ModuleBreak>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    pub mirror: bool,
}

/// A module as `cad.modules.query` reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModuleRow {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub version: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub members: u32,
    pub removed: bool,
    pub instances: Vec<ModuleInstanceRow>,
}

/// Output of `cad.modules.query`: the modules; with `uid`, the instance it is in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModulesQueried {
    pub modules: Vec<ModuleRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub instance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
