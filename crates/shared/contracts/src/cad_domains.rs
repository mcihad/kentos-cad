//! `cad.domains.edit` v1 (Değer listeleri, docs/adr/0430): the project's value lists made, changed and
//! linked from layer fields; the shared cases are `fixtures/commands/v1/cad.domains.edit.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::REVISION_TEXT;
use crate::domains::FieldDomain;
use crate::fields::{FieldChoice, LayerFieldKind};

pub const CAD_DOMAINS_EDIT: &str = "cad.domains.edit";
pub const CAD_DOMAINS_EDIT_VERSION: u32 = 1;

/// What `cad.domains.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DomainsOp {
    /// Yeni liste: a coded list (`values`) or a range (`min`, `max`); no id: one made of the name.
    Create {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        description: Option<String>,
        /// The kind of field it fits (`domainKind` in the input: `kind` tags the operation).
        domain_kind: LayerFieldKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        values: Option<Vec<FieldChoice>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        min: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        max: Option<String>,
        /// The range's ends left out (docs/adr/0438 §4); written only as `true`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        min_exclusive: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        max_exclusive: bool,
        /// A number list's unit (docs/adr/0438 §4).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        unit: Option<String>,
    },
    /// Alanın listesinden: a list made of a field's own values (or bounds), the field linked to it.
    FromField {
        /// The layer's id.
        layer: String,
        /// The field's key.
        field: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
    },
    /// Kaydet: the list replaced, its kind kept; `recode` moves the linked fields' records from a code
    /// to another of the new list in the same step (a code a record holds may not just go).
    Update {
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        values: Option<Vec<FieldChoice>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        min: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        max: Option<String>,
        /// The range's ends left out (docs/adr/0438 §4); written only as `true`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        min_exclusive: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        max_exclusive: bool,
        /// A number list's unit (docs/adr/0438 §4).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        unit: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
        recode: BTreeMap<String, String>,
    },
    /// Sil: a list no field links; `detach` lets the linked fields keep their copies.
    Remove {
        id: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        detach: bool,
    },
    /// İçe aktar (docs/adr/0439 §7): the lists of a package (`cad.domains.export`'s) added; a list of
    /// the same id kept when equal, else refused (`domain_exists`) unless `replace`, which replaces
    /// it as Kaydet does (its kind kept, a code records hold not dropped).
    Import {
        domains: Vec<FieldDomain>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        replace: bool,
    },
    /// Bağla: a field linked to a list (none: unlinked, its copy kept as its own).
    Link {
        layer: String,
        field: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        domain: Option<String>,
    },
}

/// Input of `cad.domains.edit` v1.
///
/// Refusals, checked in this order: `invalid_revision`, `revision_conflict` (status `conflict`);
/// `unknown_domain`, `layer_not_found`, `not_a_layer`, `field_not_found`, `domain_exists`,
/// `invalid_domain`, `domain_mismatch`, `invalid_recode`, `domain_code_in_use`, `domain_in_use`,
/// `layer_locked`. Warnings: `domain_values_outside`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DomainsEdit {
    pub edit: DomainsOp,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A field a list's change reached: its layer's id and its key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DomainField {
    pub layer: String,
    pub field: String,
}

/// Output (and plan) of `cad.domains.edit` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DomainsEdited {
    /// The list made, changed or taken away (a link: the list linked, none when unlinked).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub domain: Option<String>,
    /// The fields whose definition changed, in the layer tree's order.
    pub fields: Vec<DomainField>,
    /// The records `recode` moved to another code.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub recoded: u32,
    /// The lists an import added or replaced, in the package's order (docs/adr/0439 §7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub imported: Vec<String>,
    /// Whether anything changed (a plan: would change).
    pub changed: bool,
    /// The document revision afterwards (a plan: as it is).
    pub revision: String,
}

pub const CAD_DOMAINS_EXPORT: &str = "cad.domains.export";
pub const CAD_DOMAINS_EXPORT_VERSION: u32 = 1;

/// The format of a value lists' package (docs/adr/0439 §7).
pub const DOMAINS_PACKAGE: &str = "kentos.domains";
pub const DOMAINS_PACKAGE_VERSION: u32 = 1;

/// Input of `cad.domains.export` v1: the lists by their ids, in the order given; none: every list in the
/// project's order.
///
/// Refusals: `unknown_domain`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DomainsExport {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub ids: Vec<String>,
}

/// A value lists' package (`.kliste`): what `cad.domains.export` gives and `cad.domains.edit`'s import
/// takes (docs/adr/0439 §7). It carries the lists alone: no record, link or permission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DomainsPackage {
    /// `kentos.domains`.
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    pub domains: Vec<FieldDomain>,
}
