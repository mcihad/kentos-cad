//! `cad.casework.run` v1 (“Mutfak dizisi”, docs/adr/0706, 0727): base cabinets along a straight, L or U path, their
//! sink, cooktop and appliances, a countertop cut for the sink and the cooktop and, if asked, wall cabinets above, as
//! one undo step, kept as the drawing's run; `cad.casework.edit` changes or releases a run and `cad.casework.query`
//! reads them. The run's recipe and record are `kentos-contracts`' (`casework.rs`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Entity;
use crate::{
    CaseworkBreak, CaseworkDoor, CaseworkFridge, CaseworkLeg, CaseworkMember, CaseworkModule, CaseworkModuleKind,
    CaseworkRun,
};

/// The command's name.
pub const CAD_CASEWORK_RUN: &str = "cad.casework.run";
/// Its version.
pub const CAD_CASEWORK_RUN_VERSION: u32 = 1;
/// The run's edit (docs/adr/0727 §6).
pub const CAD_CASEWORK_EDIT: &str = "cad.casework.edit";
pub const CAD_CASEWORK_EDIT_VERSION: u32 = 1;
/// The runs read (docs/adr/0727 §6).
pub const CAD_CASEWORK_QUERY: &str = "cad.casework.query";
pub const CAD_CASEWORK_QUERY_VERSION: u32 = 1;

/// Plan of `cad.casework.run` (and of an edit that lays a run anew): the objects it would write (`id` 0), the library
/// blocks it would bring by name, its modules, legs, fridge and fillers (0: none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkPlan {
    pub entities: Vec<Entity>,
    pub blocks: Vec<String>,
    pub modules: Vec<CaseworkModule>,
    pub legs: Vec<CaseworkLeg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fridge: Option<CaseworkFridge>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub filler: f64,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Output of `cad.casework.run` and of `cad.casework.edit`: the run's id, its objects in the plan's order, the library
/// blocks brought, its modules, legs, fridge and fillers; a released run's objects and `released`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkBuilt {
    pub run: String,
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    pub blocks: Vec<String>,
    pub modules: Vec<CaseworkModule>,
    pub legs: Vec<CaseworkLeg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fridge: Option<CaseworkFridge>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub filler: f64,
    /// The run was released: its objects stay, no longer a run.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub released: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

// ── cad.casework.edit and cad.casework.query ───────────────────────────────

/// What `cad.casework.edit` changes (docs/adr/0727 §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CaseworkChange {
    /// A module's own values become `width` and `hand` (absent: the run's); `holds`, when given, what it holds.
    Module {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        module: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        width: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        hand: Option<CaseworkDoor>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        holds: Option<CaseworkModuleKind>,
    },
    /// The run laid anew by this recipe (its objects kept by their places in it).
    Recipe { recipe: Box<CaseworkRun> },
    /// The run released: its objects stay, no longer a run.
    Release,
}

/// Input of `cad.casework.edit` v1: the run (its id, or the persistent id of one of its objects) and the change, as
/// one undo step. Refusals: `casework_not_found`, a recipe's (`recipe.` before its path) and the revision's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkEdit {
    pub run: String,
    pub edit: CaseworkChange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Input of `cad.casework.query` v1: one run (its id or one of its objects') or, absent, every run.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub run: Option<String>,
}

/// A run as `cad.casework.query` reads it: the module table and what keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkInfo {
    pub id: String,
    pub recipe: CaseworkRun,
    pub legs: Vec<CaseworkLeg>,
    pub modules: Vec<CaseworkModule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fridge: Option<CaseworkFridge>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub filler: f64,
    pub members: Vec<CaseworkMember>,
    /// Its objects the drawing no longer holds, by persistent id.
    pub missing: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<CaseworkBreak>,
}

/// Output of `cad.casework.query`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CaseworkRuns {
    pub runs: Vec<CaseworkInfo>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
