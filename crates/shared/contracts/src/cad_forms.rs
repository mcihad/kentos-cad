//! The forms' commands of the product command catalog (docs/adr/0401 §9):
//!
//! - `cad.forms.edit` creates a form (made of its layer's fields, or given), saves its draft, renames,
//!   removes, publishes the draft as a release and turns the layer's write profile on or off. Forms
//!   are a project setting (an edit, no undo step), as the named views are.
//! - `cad.forms.validate` answers what a form and a record are worth, writing nothing: the form's
//!   preflight, the record's items (shown, required, read-only) and its issues.
//! - `cad.forms.submit` writes a record through a form: the values by the layer fields' keys, checked
//!   by the form's rules and its fields', one undo step.
//! - `cad.forms.fill` writes the same values to many records through a form, each checked and
//!   completed by itself, every record or none, one undo step (docs/adr/0403 §5).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::forms::{FormBody, FormIssue, FormItemState, ProjectForm, ProjectRelation};

/// Creates, saves, renames, removes, publishes and enforces the project's forms.
pub const CAD_FORMS_EDIT: &str = "cad.forms.edit";
pub const CAD_FORMS_EDIT_VERSION: u32 = 1;
/// A form's preflight and a record's state and issues; read-only.
pub const CAD_FORMS_VALIDATE: &str = "cad.forms.validate";
pub const CAD_FORMS_VALIDATE_VERSION: u32 = 1;
/// Writes a record through a form.
pub const CAD_FORMS_SUBMIT: &str = "cad.forms.submit";
pub const CAD_FORMS_SUBMIT_VERSION: u32 = 1;
/// Every record of a form's layer checked by its rules; read-only (docs/adr/0406 §1).
pub const CAD_FORMS_AUDIT: &str = "cad.forms.audit";
pub const CAD_FORMS_AUDIT_VERSION: u32 = 1;
/// Creates, updates and removes the project's relations (docs/adr/0405 §3).
pub const CAD_RELATIONS_EDIT: &str = "cad.relations.edit";
pub const CAD_RELATIONS_EDIT_VERSION: u32 = 1;
/// Writes the same values to many records through a form.
pub const CAD_FORMS_FILL: &str = "cad.forms.fill";
pub const CAD_FORMS_FILL_VERSION: u32 = 1;
/// Completes every record of a form's layer by the form's rules today (docs/adr/0421).
pub const CAD_FORMS_RECOMPUTE: &str = "cad.forms.recompute";
pub const CAD_FORMS_RECOMPUTE_VERSION: u32 = 1;
/// Runs a form's scenarios (docs/adr/0420).
pub const CAD_FORMS_TEST: &str = "cad.forms.test";
pub const CAD_FORMS_TEST_VERSION: u32 = 1;

/// What `cad.forms.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FormsOp {
    /// Yeni form: a form of a layer. No body: one made of the layer's fields; no id: one made of the
    /// name.
    Create {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        id: Option<String>,
        name: String,
        /// The layer's id.
        layer: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        body: Option<FormBody>,
    },
    /// Kaydet: the draft replaced. `body.revision` is the draft's revision it was made on; the draft
    /// kept is one past it.
    Update { id: String, body: FormBody },
    /// Yeniden adlandır.
    Rename { id: String, name: String },
    /// Sil.
    Remove { id: String },
    /// Yayınla: the draft frozen as the next release (its preflight without an error).
    Publish {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        note: Option<String>,
    },
    /// Kuralları uygula: whether the latest release is the layer's write profile.
    Enforce { id: String, on: bool },
}

/// Input of `cad.forms.edit` v1.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_form_id`, `invalid_form_name`,
/// `invalid_form`; `invalid_revision`, `revision_conflict` (status `conflict`); `unknown_form`,
/// `form_exists`, `too_many_forms`, `layer_not_found`, `not_a_layer`, `form_draft_changed` (status
/// `conflict`), `form_preflight`, `form_unpublished`. Warnings: `form_release_dropped`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsEdit {
    pub edit: FormsOp,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A form as the lists show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormSummary {
    pub id: String,
    pub name: String,
    pub layer: String,
    /// The draft's revision.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// The latest release's version; none: never published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub enforce: bool,
}

/// Output of `cad.forms.edit` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsEdited {
    /// The project's forms afterwards, in order.
    pub forms: Vec<FormSummary>,
    /// The form created, saved, renamed, published or enforced, as kept; none for a remove.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub form: Option<ProjectForm>,
    /// The draft's preflight afterwards (none for a remove).
    pub issues: Vec<FormIssue>,
    /// Whether the settings changed.
    pub changed: bool,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.forms.validate` v1: a form, and a record to ask it about.
///
/// Refusals: `invalid_uid`, `invalid_moment`, `invalid_user`; `unknown_form`, `form_unpublished`,
/// `entity_not_found`, `form_layer_mismatch`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsValidate {
    /// The form's id.
    pub form: String,
    /// Its draft instead of its latest release (the designer's preview).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
    /// An object of the form's layer (lowercase UUID text with hyphens); none: the form alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: Option<String>,
    /// Values by the layer fields' keys, as a submit would write them (empty: no value); asked of
    /// the record afterwards.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub values: BTreeMap<String, String>,
    /// The form's defaults in the record's empty fields first (a new record).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub defaults: bool,
    /// The moment the rules' `@tarih` and `@simdi` read: a date, or a date and a local time
    /// (`2026-10-16T14:30:00`); none: they read empty, and a calculation or default that reads them is
    /// left out (docs/adr/0402 §5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moment: Option<String>,
    /// The person the rules' `@kullanici` reads (1–64 characters); none: it reads empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub user: Option<String>,
}

/// Output of `cad.forms.validate` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsValidated {
    /// The release asked; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    /// The preflight's issues, then the record's.
    pub issues: Vec<FormIssue>,
    /// Whether the record (or the form alone) has no error.
    pub valid: bool,
    /// The record's items, in the form's order; empty without a record.
    pub items: Vec<FormItemState>,
    /// The record's attributes afterwards; none without a record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
}

/// Input of `cad.forms.submit` v1.
///
/// Refusals: `invalid_uid`, `invalid_moment`, `invalid_user`; `invalid_revision`, `revision_conflict`
/// (status `conflict`);
/// `unknown_form`, `form_unpublished`, `form_preflight`, `entity_not_found`, `form_layer_mismatch`,
/// `layer_locked`, `form_invalid` (the first issue, `path` its field's value).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsSubmit {
    /// The form's id.
    pub form: String,
    /// Through its draft instead of its latest release (a form not yet published).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
    /// The object (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// Values by the layer fields' keys (empty: no value).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub values: BTreeMap<String, String>,
    /// The form's defaults in the record's empty fields first (a new record).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub defaults: bool,
    /// The moment the rules' `@tarih` and `@simdi` read: a date, or a date and a local time
    /// (`2026-10-16T14:30:00`); none: they read empty, and a calculation or default that reads them is
    /// left out (docs/adr/0402 §5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moment: Option<String>,
    /// The person the rules' `@kullanici` reads (1–64 characters); none: it reads empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub user: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.forms.submit` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsSubmitted {
    pub uid: String,
    /// The release written through; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    /// The record's attributes afterwards.
    pub attrs: BTreeMap<String, String>,
    /// Whether the object changed.
    pub changed: bool,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.forms.fill` v1 (docs/adr/0403 §5).
///
/// Refusals: `no_entities`, `invalid_uid` (at `uids[i]`), `invalid_moment`, `invalid_user`;
/// `invalid_revision`, `revision_conflict` (status `conflict`); `unknown_form`, `form_unpublished`,
/// `form_preflight`, `entity_not_found` and `form_layer_mismatch` (at `uids[i]`), `layer_locked`,
/// `form_invalid` (the first record's first issue: `path` its field's value, `uids` the record).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsFill {
    /// The form's id.
    pub form: String,
    /// Through its draft instead of its latest release (a form not yet published).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
    /// The objects, each once (lowercase UUID texts with hyphens; a repeated one is written once).
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// Values by the layer fields' keys (empty: no value), the same for every record.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub values: BTreeMap<String, String>,
    /// The moment the rules' `@tarih` and `@simdi` read (as `cad.forms.submit`'s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moment: Option<String>,
    /// The person the rules' `@kullanici` reads (as `cad.forms.submit`'s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub user: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.forms.fill` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsFilled {
    /// The records that changed, in the input's order.
    pub changed: Vec<String>,
    /// The release written through; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.relations.edit` does (docs/adr/0405 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RelationsOp {
    /// A new relation.
    Create { relation: ProjectRelation },
    /// The relation of the same id replaced.
    Update { relation: ProjectRelation },
    /// The relation removed; its links stay in the records.
    Remove { id: String },
}

/// Input of `cad.relations.edit` v1.
///
/// Refusals, in this order: `invalid_relation`; `invalid_revision`, `revision_conflict` (status
/// `conflict`); `unknown_relation`, `relation_exists`, `too_many_relations`, `layer_not_found`,
/// `not_a_layer`, `relation_field_unknown`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RelationsEdit {
    pub edit: RelationsOp,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.relations.edit` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RelationsEdited {
    /// The project's relations afterwards, in order.
    pub relations: Vec<ProjectRelation>,
    /// Whether the settings changed.
    pub changed: bool,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.forms.audit` v1 (docs/adr/0406 §1).
///
/// Refusals: `invalid_count` (at `limit`), `invalid_moment`, `invalid_user`; `unknown_form`,
/// `form_unpublished`, `form_preflight`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsAudit {
    /// The form's id.
    pub form: String,
    /// Through its draft instead of its latest release.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
    /// The records with only warnings are listed too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub warnings: bool,
    /// The most records listed (1–10000); none: 500. The counts are of every record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub limit: Option<u32>,
    /// As `cad.forms.submit`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moment: Option<String>,
    /// As `cad.forms.submit`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub user: Option<String>,
}

/// A record `cad.forms.audit` lists: its persistent id and its issues.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AuditedRecord {
    pub uid: String,
    pub issues: Vec<FormIssue>,
}

/// Output of `cad.forms.audit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsAudited {
    /// The release asked; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    /// The layer's records checked.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub checked: u32,
    /// The records with an error.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub failing: u32,
    /// The records with warnings and no error.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub warned: u32,
    /// The records listed, in the layer's order: those with an error, and with `warnings` those
    /// with only warnings.
    pub records: Vec<AuditedRecord>,
    /// More records had issues than `limit` lists.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub more: bool,
}

/// Input of `cad.forms.test` v1 (docs/adr/0420): a form's scenarios run again, writing nothing.
///
/// Refusals, in this order: `unknown_form`, `form_unpublished` (without `draft`, a form never
/// published), `form_preflight` (the body has an error), `unknown_test` (a name the body has not).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsTest {
    /// The form's id.
    pub form: String,
    /// Its draft's scenarios on its draft, instead of its latest release's on that release.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
    /// Only these scenarios, by name, in this order; empty: all, in the body's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub names: Vec<String>,
}

/// One scenario's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormTestResult {
    pub name: String,
    pub passed: bool,
    /// What differs from what it expects, one sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub differences: Vec<String>,
}

/// Output of `cad.forms.test` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsTested {
    pub results: Vec<FormTestResult>,
    /// How many passed and how many there were.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub passed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: u32,
}

/// Input of `cad.forms.recompute` v1 (Hesapları yenile, docs/adr/0421): every record of the form's
/// layer completed by its rules (calculations, hidden fields emptied when the form says so), the ones
/// that change written as one undo step.
///
/// Refusals, in this order: `invalid_moment`, `invalid_user`, `invalid_revision`, `revision_conflict`
/// (status `conflict`), `unknown_form`, `form_unpublished`, `form_preflight`, `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsRecompute {
    /// The form's id.
    pub form: String,
    /// Through its draft instead of its latest release.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
    /// The moment the rules' `@tarih` and `@simdi` read (as `cad.forms.submit`'s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moment: Option<String>,
    /// The person the rules' `@kullanici` reads (as `cad.forms.submit`'s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub user: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.forms.recompute` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsRecomputed {
    /// How many records the layer holds.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub checked: u32,
    /// The records that changed, in the layer's order.
    pub changed: Vec<String>,
    /// The records left as they were because a calculation's value does not fit its field (or the
    /// form does not settle on them), in the layer's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub failed: Vec<String>,
    /// The release used; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
