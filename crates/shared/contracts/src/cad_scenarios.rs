//! The scenarios' command of the product command catalog (docs/adr/0210 §9,
//! §11): `cad.scenarios.edit` makes a scenario (a group whose layers copy
//! base layers, each standing for its source) or applies one to the field
//! state (Mevcut durum). Each is one undo step. Senaryo oluştur and
//! Senaryoyu uygula write through it on the web (`apps/web/src/product`) and
//! on the desktop (`crates/native/application`); both pass the shared cases
//! in `fixtures/commands/v1`. Showing a scenario is a view (layers'
//! visibility), not a command.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Makes or applies a scenario.
pub const CAD_SCENARIOS_EDIT: &str = "cad.scenarios.edit";
pub const CAD_SCENARIOS_EDIT_VERSION: u32 = 1;

/// What `cad.scenarios.edit` does; it names the undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ScenarioOperation {
    /// “Senaryo oluştur”.
    Create,
    /// “Senaryoyu uygula”.
    Apply,
}

/// Input of `cad.scenarios.edit` v1.
///
/// - `create`: `name`; `layers`, the base layers it copies (absent or empty:
///   an empty scenario); `copyObjects` (absent: true), `note`. A group named
///   `name` goes on top of the tree, marked a scenario; in it, for each
///   source in the order given, a layer of the source's name, style, fields,
///   time setting and snapping, standing for it (`replaces`), with copies of
///   its objects under new ids when `copyObjects`.
/// - `apply`: `scenario`, the scenario group. Each of its layers standing for
///   a base layer of the tree replaces that layer's objects with its own (they
///   keep their ids) and goes; the group is a scenario no more: an ordinary
///   group keeping its other layers (new base layers), or gone when it keeps
///   none. The active layer on a scenario layer that goes moves to its base layer.
///
/// Refusals (`CommandError.code`), checked in this order: for `create`,
/// `empty_name` (absent, or empty or only white space), `invalid_name`
/// (longer than 80 characters), `invalid_note` (`ScenarioInfo::problem`),
/// `duplicate_layer` (a layer listed twice); for `apply`, `no_scenario`;
/// then `invalid_revision`, `revision_conflict` (status `conflict`); for
/// `create`, `layer_not_found`, `not_a_base_layer` (a group, a layer in a
/// scenario, or one drawn from a service), `layer_locked` (a source locked,
/// itself or through a group, when its objects are copied: docs/adr/0037);
/// for `apply`, `scenario_not_found` (no such node, or not a scenario group),
/// `layer_locked` (a layer of the scenario or a base layer it replaces, itself
/// or through a group).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosEdit {
    pub operation: ScenarioOperation,
    /// `create`: the scenario's name (trimmed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// `create`: the base layers it copies, in order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
    /// `create`: whether their objects are copied (absent: true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copy_objects: Option<bool>,
    /// `create`: the scenario's note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    /// `apply`: the scenario group's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scenario: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A base layer and the scenario layer standing for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenarioPair {
    pub base: String,
    pub layer: String,
}

/// Output of `cad.scenarios.edit` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosEdited {
    /// The scenario group made, or the one applied (gone now).
    pub scenario: String,
    /// `create`: each source with its copy; `apply`: each base layer with the scenario layer applied onto it.
    pub layers: Vec<ScenarioPair>,
    /// `apply`: the scenario's other layers, base layers now, in its group (an ordinary group now).
    pub kept: Vec<String>,
    /// Objects copied (`create`) or moved into base layers (`apply`).
    pub objects: u32,
    /// `apply`: the base layers' objects removed.
    pub removed: u32,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.scenarios.edit` would write (plan mode); nothing is written. The
/// same as the output, the ids those execute would give, and the revision
/// the plan was made against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScenariosEditPlan {
    pub scenario: String,
    pub layers: Vec<ScenarioPair>,
    pub kept: Vec<String>,
    pub objects: u32,
    pub removed: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
