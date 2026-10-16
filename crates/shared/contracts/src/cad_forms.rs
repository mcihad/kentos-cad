//! The forms' commands of the product command catalog (docs/adr/0401 §9):
//!
//! - `cad.forms.edit` creates a form (made of its layer's fields, or given), saves its draft, renames,
//!   removes, publishes the draft as a release and turns the layer's write profile on or off. Forms
//!   are a project setting (an edit, no undo step), as the named views are.
//! - `cad.forms.validate` answers what a form and a record are worth, writing nothing: the form's
//!   preflight, the record's items (shown, required, read-only) and its issues.
//! - `cad.forms.submit` writes a record through a form: the values by the layer fields' keys, checked
//!   by the form's rules and its fields', one undo step.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::forms::{FormBody, FormIssue, FormItemState, ProjectForm};

/// Creates, saves, renames, removes, publishes and enforces the project's forms.
pub const CAD_FORMS_EDIT: &str = "cad.forms.edit";
pub const CAD_FORMS_EDIT_VERSION: u32 = 1;
/// A form's preflight and a record's state and issues; read-only.
pub const CAD_FORMS_VALIDATE: &str = "cad.forms.validate";
pub const CAD_FORMS_VALIDATE_VERSION: u32 = 1;
/// Writes a record through a form.
pub const CAD_FORMS_SUBMIT: &str = "cad.forms.submit";
pub const CAD_FORMS_SUBMIT_VERSION: u32 = 1;

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
/// Refusals: `invalid_uid`; `unknown_form`, `form_unpublished`, `entity_not_found`,
/// `form_layer_mismatch`.
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
/// Refusals: `invalid_uid`; `invalid_revision`, `revision_conflict` (status `conflict`);
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
