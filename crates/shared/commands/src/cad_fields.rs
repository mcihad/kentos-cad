//! `cad.fields.usage` (docs/adr/0414 §3): where a layer's field is named, the forms' drafts and
//! published releases, the lists from another layer, the related records, the relations and the
//! layer's own rules. The places and their words are `kentos_forms::field_refs`'s.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::FieldUse;
use crate::cad::REVISION_TEXT;

pub const CAD_FIELDS_USAGE: &str = "cad.fields.usage";
pub const CAD_FIELDS_USAGE_VERSION: u32 = 1;

/// Input of `cad.fields.usage` v1: a layer's field by its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsUsage {
    pub layer: String,
    pub field: String,
}

/// Output of `cad.fields.usage` v1: the places, in the order of docs/adr/0414 §1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsUsed {
    pub layer: String,
    pub field: String,
    pub uses: Vec<FieldUse>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

pub const CAD_FIELDS_MIGRATE: &str = "cad.fields.migrate";
pub const CAD_FIELDS_MIGRATE_VERSION: u32 = 1;

/// Input of `cad.fields.migrate` v1 (docs/adr/0439 §8): a layer's values written before its fields
/// were typed, turned into their fields' canonical texts (`1,5` into `1.5`, `17.10.2026` into
/// `2026-10-17`, `evet` into `true`, a listed value's label into its code). Nothing is lost: a value
/// that does not fit is kept as written and counted, an empty one stays empty, an attribute no field
/// names stays as it is.
///
/// Refusals, checked in this order: `invalid_revision`, `revision_conflict` (status `conflict`);
/// `layer_not_found`, `not_a_layer`, `field_not_found`, `layer_locked`. Warnings: `values_unfit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsMigrate {
    /// The layer's id.
    pub layer: String,
    /// The fields' keys; none: every field of the layer that is not text without a list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fields: Vec<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What a field's values became.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldMigration {
    /// The field's key.
    pub field: String,
    /// Values written anew in their canonical text.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    /// Values already canonical.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    /// Values that do not fit the field, kept as written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unfit: u32,
    /// Empty values (an attribute holding only white space), kept empty.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub blank: u32,
    /// Objects without the attribute.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub absent: u32,
    /// The first values that do not fit (at most five), as written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub samples: Vec<String>,
}

/// Output (and plan) of `cad.fields.migrate` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldsMigrated {
    pub layer: String,
    /// Each field, in the layer's order.
    pub fields: Vec<FieldMigration>,
    /// The objects one of whose values changed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The attributes no field of the layer names, kept as they are (their names, sorted).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub unknown: Vec<String>,
    /// Whether anything changed (a plan: would change).
    pub changed: bool,
    /// The document revision afterwards (a plan: as it is).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
