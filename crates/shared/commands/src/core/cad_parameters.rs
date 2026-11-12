//! The parameter manager's command (docs/adr/0336; AutoCAD's PARAMETERS): the drawing's parameters added, changed,
//! renamed and taken away by name, the drawing solved with their new values and written with them as one undo step.
//!
//! A parameter is a dimension's (its constraint names it: its expression drives the dimension, or is empty when the
//! dimension only measures) or the user's (no constraint names it; other expressions read it). Values are written in
//! the parameter's own unit: a length in the drawing's unit, an angle in the project's angle unit (`constraint_expr`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::constraints::{CadParameter, ConstraintDiagnosis, ParameterKind};
use crate::entity::Entity;

/// Changes the drawing's parameters.
pub const CAD_PARAMETERS_EDIT: &str = "cad.parameters.edit";
pub const CAD_PARAMETERS_EDIT_VERSION: u32 = 1;

/// A bound's change: kept when absent, cleared when null.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BoundChange {
    /// No bound.
    Clear,
    /// This bound, in the parameter's own unit.
    To {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "parameter")))]
        value: f64,
    },
}

/// What changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ParameterEdit {
    /// A user parameter: its name, kind, expression and optional range and description.
    Add { parameter: CadParameter },
    /// A parameter's expression, range or description; the absent ones stay. A dimension's that only measures takes
    /// no expression.
    Set {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        expression: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        min: Option<BoundChange>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        max: Option<BoundChange>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        description: Option<String>,
    },
    /// A parameter's new name: the expressions that read it and its dimension follow.
    Rename { name: String, to: String },
    /// A user parameter no other parameter reads.
    Remove { name: String },
}

/// Input of `cad.parameters.edit` v1: one undo step “Parametreler”: the parameters and every object the solve moves.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_parameter` (a name not valid),
/// `parameter_expression` (an expression that does not read), `invalid_revision`, `revision_conflict` (status
/// `conflict`), `parameter_not_found`, `parameter_exists` (a name in use), `parameter_kind` (a kind its dimension does
/// not take), `parameter_measured` (an expression for a dimension that only measures), `parameter_read` (a parameter
/// another reads, or a dimension's, taken away), then the parameters' values (`parameter_missing`,
/// `parameter_cycle`, `parameter_range`) and the solve's (`layer_locked`, `constraint_conflict`,
/// `constraint_singular`, `not_converged`, `shape_flipped`), as `cad.constraints.edit` answers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParametersEdit {
    /// The changes, in order.
    pub edits: Vec<ParameterEdit>,
    /// Writes even when a shape would turn inside out.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub allow_flip: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A parameter with its value now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParameterValue {
    pub name: String,
    pub kind: ParameterKind,
    /// Its value in its own unit.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "parameter")))]
    pub value: f64,
}

/// The answer of `cad.parameters.edit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParametersEdited {
    /// Every parameter's value after the write, in the table's order.
    pub values: Vec<ParameterValue>,
    /// The objects the solve moved.
    pub changed: Vec<String>,
    pub diagnosis: ConstraintDiagnosis,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.parameters.edit` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParametersEditPlan {
    pub parameters: Vec<CadParameter>,
    pub values: Vec<ParameterValue>,
    pub changed: Vec<Entity>,
    pub diagnosis: ConstraintDiagnosis,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
