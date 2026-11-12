//! Toplu yazma in the attribute table (docs/adr/0507, plan/gis.md CBS-20): a layer's attribute written over many
//! objects in one undo step, an expression's value per object (Alan hesapla from a column's header) or the values
//! pasted from a spreadsheet (one row per object). Every value is checked as `cad.entities.set` checks it: the
//! field's rules and canonical text, required fields, the layer's subtypes, value combinations, record rules and
//! unique fields. By default the write is whole or nothing: one value that does not fit refuses it all, with each
//! problem said by object and field; `policy: valid` writes the fitting values and reports the rest. `plan` shows
//! what would change (each object's old and new value) without writing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_FIELDS_WRITE: &str = "cad.fields.write";
pub const CAD_FIELDS_WRITE_VERSION: u32 = 1;

/// At most this many changes and problems are listed in an answer (all are counted).
pub const FIELDS_WRITE_LISTED: usize = 200;

/// What to do when a value does not fit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WritePolicy {
    /// Every value or none (`invalid_values`).
    #[default]
    All,
    /// The values that fit; the others reported (`values_skipped`).
    Valid,
}

/// What an empty result of the expression does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EmptyResult {
    /// The old value stays.
    #[default]
    Keep,
    /// The value is taken away.
    Clear,
}

/// One object's pasted values: field → value (null takes the value away).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldRow {
    /// The object's persistent id.
    pub uid: String,
    pub values: BTreeMap<String, Option<String>>,
}

/// What is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldsWriteOp {
    /// Alan hesapla: the expression's value (İfadeyle seç's language, the layer's fields typed, geometry values in
    /// the project's drawing unit) into `field` of each object, as text in the field's canonical form.
    #[serde(rename_all = "camelCase")]
    Calculate {
        field: String,
        expression: String,
        /// The objects (persistent ids); empty: every object of the layer.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
        uids: Vec<String>,
        #[serde(default, skip_serializing_if = "is_keep")]
        #[cfg_attr(feature = "ts", ts(as = "Option<EmptyResult>", optional))]
        empty: EmptyResult,
    },
    /// Yapıştır: the values of each row into its object.
    #[serde(rename_all = "camelCase")]
    Values { rows: Vec<FieldRow> },
}

fn is_keep(e: &EmptyResult) -> bool {
    *e == EmptyResult::Keep
}

fn is_all(p: &WritePolicy) -> bool {
    *p == WritePolicy::All
}

/// Input of `cad.fields.write` v1. Refusals, in order: `layer_not_found`, `not_a_layer`, `layer_locked`,
/// `nothing_to_set` (no rows, or a row without values), `invalid_attribute` (a field's name empty), `invalid_uid`,
/// `entity_not_found` (an object not in the drawing or not on the layer), `field_not_found` (a field the layer's
/// fields do not name, on a layer with fields), `invalid_expression`, `invalid_values` (with `policy: all`, a value
/// that does not fit: the answer's first problem, every one in the plan). Warning: `values_skipped` (with
/// `policy: valid`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsWrite {
    /// The layer (its id).
    pub layer: String,
    pub write: FieldsWriteOp,
    #[serde(default, skip_serializing_if = "is_all")]
    #[cfg_attr(feature = "ts", ts(as = "Option<WritePolicy>", optional))]
    pub policy: WritePolicy,
}

/// A value changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldChange {
    pub uid: String,
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub after: Option<String>,
}

/// A value that does not fit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ValueProblem {
    pub uid: String,
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<String>,
    /// The code `cad.entities.set` would refuse it with (`invalid_attribute`, `attribute_required`, `rule_broken`,
    /// `not_unique`, `subtype_misfit`, `invalid_combination`).
    pub code: String,
    pub message: String,
}

/// What `cad.fields.write` wrote, or with `plan` would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsWritten {
    /// Objects whose values change, objects already as asked, empty results kept, objects with a problem.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unchanged: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub invalid: u32,
    /// The first changes and problems, in the objects' order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<FieldChange>>", optional))]
    pub changes: Vec<FieldChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ValueProblem>>", optional))]
    pub problems: Vec<ValueProblem>,
    pub revision: String,
}
