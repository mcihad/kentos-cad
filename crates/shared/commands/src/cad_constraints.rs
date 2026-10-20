//! The constraint commands of the product command catalog (docs/adr/0335; AutoCAD's GEOMCONSTRAINT, DIMCONSTRAINT,
//! DELCONSTRAINT and CONSTRAINTSETTINGS' sense): constraints added to and taken from the drawing by the parts they
//! name, the drawing solved and written with them as one undo step, and the read-only query that lists them with the
//! solver's diagnosis.
//!
//! A constraint's sense (a tangent's side, a distance's side of a segment, an angle's quadrant, the sign of a
//! horizontal or vertical distance) is read from the drawing as it is when the constraint is added, unless the input
//! gives it; the solve keeps it, and the edit is refused when a shape would turn inside out (docs/adr/0335 §4). Lengths
//! are metres and angles radians on the wire, whatever the project shows (docs/adr/0165 §2); a dimension's value is an
//! expression in the drawing's unit and the project's angle unit (`constraint_expr`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::constraints::{CadParameter, Constraint, ConstraintDiagnosis, ConstraintRule};
use crate::entity::Entity;

/// Adds, takes away or changes the drawing's constraints.
pub const CAD_CONSTRAINTS_EDIT: &str = "cad.constraints.edit";
pub const CAD_CONSTRAINTS_EDIT_VERSION: u32 = 1;

/// Lists the drawing's constraints and parameters with the solver's diagnosis (read only).
pub const CAD_CONSTRAINTS_QUERY: &str = "cad.constraints.query";
pub const CAD_CONSTRAINTS_QUERY_VERSION: u32 = 1;

/// Where a new constraint's sense comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConstraintSense {
    /// From the drawing as it is: a tangent's side, a distance's side of its segment, the angle's quadrant under
    /// 180°, a horizontal or vertical distance taken from the lower to the higher (its parts swapped when needed).
    #[default]
    Drawing,
    /// As the rule gives it.
    Given,
}

fn drawing(s: &ConstraintSense) -> bool {
    *s == ConstraintSense::Drawing
}

/// A constraint to add.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewConstraint {
    /// The rule and its parts. A dimension's `parameter` names its parameter: empty for the next free name
    /// (`d1`, `ang1`, `rad1`, `dia1`); a name not in use makes a new parameter.
    pub rule: ConstraintRule,
    /// Where its sense comes from.
    #[serde(default, skip_serializing_if = "drawing")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<ConstraintSense>"))]
    pub sense: ConstraintSense,
    /// A dimension's value: an expression in the drawing's unit and the project's angle unit (`3.5`, `120cm`,
    /// `genislik / 2`); none keeps the value the drawing measures now. Not for a reference dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<String>,
    /// A dimension that only measures (AutoCAD's reference parameter): it drives nothing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reference: bool,
}

/// What changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConstraintEdit {
    /// These constraints are added, in order, and the drawing solved with them. One that repeats the ones before
    /// (redundant) is refused: a dimension can be added as a reference instead.
    Add { constraints: Vec<NewConstraint> },
    /// These constraints (by id) are taken away; a dimension's parameter goes with it unless another parameter
    /// reads it, when it stays as a user parameter with its value. Nothing moves.
    Remove { ids: Vec<String> },
    /// These dimensions drive (`reference` false: their parameter takes the value they measure now) or only
    /// measure (`reference` true: their parameter's expression is cleared).
    Mode { ids: Vec<String>, reference: bool },
    /// The constraint's other solution (AutoCAD's flip): a tangent on the other side, a distance on the other side
    /// of its segment, the angle's other quadrant; the drawing solved again.
    Flip { id: String },
    /// Solves the drawing with its constraints as they are (after an edit made without them).
    Solve,
}

/// Input of `cad.constraints.edit` v1: one undo step “Kısıtlar”: the constraints and every object the solve moves.
/// Objects on locked layers do not move. The edit is all or nothing: refused, nothing is written.
///
/// Refusals (`CommandError.code`), checked in this order: `no_constraints` (an empty list), `invalid_uid`,
/// `invalid_constraint` (parts that do not fit the rule, a reference rule not a dimension), `invalid_parameter`
/// (a name not valid), `parameter_expression` (a value that does not read, or given to a reference),
/// `invalid_revision`, `revision_conflict` (status `conflict`), `entity_not_found`, `element_not_found` (a part the
/// object does not have), `constraint_not_found`, `parameter_in_use` (a parameter another dimension has),
/// `parameter_kind` (a parameter of another kind), the parameters' values (`parameter_expression`,
/// `parameter_missing`: a name no parameter has, `parameter_cycle`, `parameter_range`: out of its range, or a radius
/// or diameter not above nought), `invalid_constraint` (the set's rules), `layer_locked` (a constraint whose objects
/// are all locked does not hold), `constraint_conflict` (`uids`: the constraints that cannot hold with the others),
/// `constraint_singular`, `not_converged`, `constraint_redundant` (`uids`: the constraints added that repeat),
/// `shape_flipped` (`uids`: the objects that would turn inside out).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstraintsEdit {
    /// What changes.
    pub edit: ConstraintEdit,
    /// Objects (persistent ids) this solve does not move, as AutoCAD keeps the object picked first and moves the
    /// second; they are not held afterwards.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub hold: Vec<String>,
    /// Writes even when a shape would turn inside out (the user chose that solution).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub allow_flip: bool,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// The answer of `cad.constraints.edit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstraintsEdited {
    /// The constraints added, their new ids in the input's order.
    pub added: Vec<String>,
    /// The constraints taken away.
    pub removed: Vec<String>,
    /// The objects the solve moved.
    pub changed: Vec<String>,
    /// The drawing's constraints after the write, as the solver finds them.
    pub diagnosis: ConstraintDiagnosis,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.constraints.edit` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstraintsEditPlan {
    /// The constraints as they would be, the new ones last with their ids.
    pub constraints: Vec<Constraint>,
    /// The parameters as they would be.
    pub parameters: Vec<CadParameter>,
    /// The objects the solve would move, as they would be written, with their slots.
    pub changed: Vec<Entity>,
    /// The solver's diagnosis of the result.
    pub diagnosis: ConstraintDiagnosis,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.constraints.query` v1: the constraints of the drawing, or those naming these objects.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstraintsQuery {
    /// Only the constraints naming one of these objects (persistent ids); none: all of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
}

/// A constraint as the query lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ListedConstraint {
    pub constraint: Constraint,
    /// Its Turkish name (`Paralel`).
    pub label: String,
    /// A dimension's value now: metres or radians, as the drawing measures it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "constraint")))]
    pub measured: Option<f64>,
    /// Whether it holds now.
    pub holds: bool,
}

/// The answer of `cad.constraints.query`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstraintsListed {
    pub constraints: Vec<ListedConstraint>,
    pub parameters: Vec<CadParameter>,
    /// The solver's diagnosis of the drawing as it is (the constraints asked for and those they are solved with).
    pub diagnosis: ConstraintDiagnosis,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
