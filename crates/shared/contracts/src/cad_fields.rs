//! `cad.fields.usage` (docs/adr/0414 §3): where a layer's field is named, the forms' drafts and
//! published releases, the lists from another layer, the related records, the relations and the
//! layer's own rules. The places and their words are `kentos_forms::field_refs`'s.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_FIELDS_USAGE: &str = "cad.fields.usage";
pub const CAD_FIELDS_USAGE_VERSION: u32 = 1;

/// How a place names a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldUseHow {
    /// By its key (its stable id, else its name): a form item, a list, a relation.
    Key,
    /// By its name: an attribute's name (the time, the card's fields, a table rule's field).
    Name,
    /// In an expression (İfadeyle seç's language).
    Expression,
    /// In a template's `{…}` expressions.
    Template,
}

/// One place that names a field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldUse {
    /// The place in words: “Bina denetimi” formu › “Not” öğesi › Görünme koşulu.
    pub place: String,
    /// Where it is in the drawing: `settings/forms/0/draft/items/2/visibleWhen`, `layers/<id>/…`.
    pub path: String,
    pub how: FieldUseHow,
    /// In a published release of a form, which a rename does not change. Only `true` is written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub kept: bool,
}

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
