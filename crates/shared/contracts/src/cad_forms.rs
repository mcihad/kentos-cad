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
/// Takes a record a step in its form's workflow (docs/adr/0428).
pub const CAD_FORMS_TRANSITION: &str = "cad.forms.transition";
pub const CAD_FORMS_TRANSITION_VERSION: u32 = 1;
/// Compares two versions of a form (docs/adr/0427).
pub const CAD_FORMS_COMPARE: &str = "cad.forms.compare";
pub const CAD_FORMS_COMPARE_VERSION: u32 = 1;
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
    /// Yayından kaldır (docs/adr/0445 §1): no record is written through the form until it is put back
    /// (`on: false`); a form whose rules are enforced is not withdrawn.
    Withdraw { id: String, on: bool },
    /// Yorum (docs/adr/0441 §5): a reviewer's comment on the form or an item of its draft.
    Comment {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        item: Option<String>,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        author: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        at: Option<String>,
    },
    /// A comment settled or opened again.
    Resolve {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        comment: u32,
        resolved: bool,
    },
}

/// Input of `cad.forms.edit` v1.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_form_id`, `invalid_form_name`,
/// `invalid_form`; `invalid_revision`, `revision_conflict` (status `conflict`); `unknown_form`,
/// `form_exists`, `too_many_forms`, `layer_not_found`, `not_a_layer`, `form_draft_changed` (status
/// `conflict`), `form_preflight`, `form_unpublished`, `form_item_unknown`, `form_comment_unknown`, `form_tests_failed`. Warnings:
/// `form_release_dropped`, `form_breaking`.
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
    /// A publish over an earlier release (docs/adr/0442 §2): the layer's records that held by the
    /// earlier release and do not by this one; none when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub breaking: Option<FormBreaking>,
}

/// The records a new release breaks (docs/adr/0442 §2): how many, and how many by each field the new
/// release finds wrong (by the fields' keys, in their bytes' order).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormBreaking {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records: u32,
    pub fields: Vec<FieldCount>,
}

/// A field and how many records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldCount {
    pub field: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records: u32,
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
    /// What the form needs of the host that shows it (docs/adr/0441 §4): `moment`, `user`, `layers`,
    /// `geometry`, `attachments`, `workflow`, `tool:<id>`; none: a pure form.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub requires: Vec<String>,
    /// The layer's fields the release binds changed since it was published (docs/adr/0444 §2): review the
    /// form and publish it again. Written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub fields_changed: bool,
}

/// Input of `cad.forms.submit` v1.
///
/// Refusals: `invalid_uid`, `invalid_moment`, `invalid_user`; `invalid_revision`, `revision_conflict`
/// (status `conflict`);
/// `unknown_form`, `form_unpublished`, `form_preflight`, `form_release_changed` (status `conflict`),
/// `entity_not_found`, `form_layer_mismatch`, `layer_locked`, `record_changed` (status `conflict`),
/// `form_invalid` (the first issue, `path` its field's value); a layer's rules and unique fields refuse
/// as every write's (`rule_broken`, `not_unique`).
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
    /// The release the form was opened with (docs/adr/0438 §6): a later release published since is
    /// `form_release_changed` (status `conflict`), so an old screen does not write past a newer form's
    /// rules; none: the latest, whichever it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub release: Option<u32>,
    /// The record's attributes as the form opened it (docs/adr/0438 §7): the record changed since
    /// (another write, an undo) is `record_changed` (status `conflict`) instead of being written over;
    /// none: whatever it holds now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opened: Option<BTreeMap<String, String>>,
    /// The record's children written in the same step (docs/adr/0440 §2): linked, unlinked, changed or
    /// made, each through its relation; the record and its children go in together or not at all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<FormChild>>", optional))]
    pub children: Vec<FormChild>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A child written with its parent's record (docs/adr/0440 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormChild {
    /// The project relation whose parent is the form's layer.
    pub relation: String,
    /// The child (lowercase UUID text with hyphens): an object of the relation's child layer, or a new
    /// record's id the caller made (the same id given again writes the same record, not another).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// Values by the child layer's fields' keys, through the child layer's form when it has one.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub values: BTreeMap<String, String>,
    /// The link to the record taken off (Bağı kopar) instead of made; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub unlink: bool,
}

/// A child as the submission wrote it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChildSubmitted {
    pub uid: String,
    pub relation: String,
    /// A new record, made by the submission; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub created: bool,
    pub attrs: BTreeMap<String, String>,
    pub changed: bool,
}

/// A record a command wrote (docs/adr/0440 §5): its id, layer, whether it was made, the fields that
/// changed by their names, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordChange {
    pub uid: String,
    pub layer: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub created: bool,
    pub fields: Vec<String>,
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
    /// The children as written (docs/adr/0440 §2), in the input's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ChildSubmitted>>", optional))]
    pub children: Vec<ChildSubmitted>,
    /// The records written (docs/adr/0440 §5): the record, then its children, in order; none
    /// unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RecordChange>>", optional))]
    pub changes: Vec<RecordChange>,
    /// The record's snapshot afterwards (docs/adr/0440 §4): its fingerprint with its children and
    /// attachments, what a reviewer approves.
    pub snapshot: String,
    /// The undo step the write is (plan and an unchanged record: none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub undo: Option<String>,
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
    /// The children whose field holds a link of the relation (the edited one's, before the edit;
    /// docs/adr/0440 §3).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub links: u32,
    /// The document's revision afterwards (plan: the one it was made against).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
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

/// Input of `cad.forms.compare` v1 (Sürümleri karşılaştır, docs/adr/0427): what changed from one version of
/// a form to another; nothing is written.
///
/// Refusals, in this order: `unknown_form`, `form_unpublished` (no release to compare from),
/// `unknown_release` (a version the form has not, at `from` or `to`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsCompare {
    /// The form's id.
    pub form: String,
    /// The release compared from; none: the latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub from: Option<u32>,
    /// The release compared to; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub to: Option<u32>,
}

/// What happened to an item from one version to the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FormChangeKind {
    Added,
    Removed,
    /// Under another tab or group, or elsewhere among its siblings.
    Moved,
    /// Its own properties (its items aside).
    Changed,
}

/// One change of a form: an item's, or the form's own (`item` none: `clearHidden`, the scenarios).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<String>,
    pub kind: FormChangeKind,
    /// The properties that changed, by their names in the form's JSON, in their order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub properties: Vec<String>,
    /// What the change touches (docs/adr/0442 §3), in this order: `layout`, `binding`, `rule`, `action`,
    /// `workflow`, `tests`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub areas: Vec<String>,
    /// Turkish, for people.
    pub words: String,
}

/// Output of `cad.forms.compare` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsCompared {
    /// The release compared from.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub from: u32,
    /// The release compared to; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub to: Option<u32>,
    pub changes: Vec<FormChange>,
    /// What the newer version needs of its host that the older did not, and the other way
    /// (`kentos_forms::requirements`, docs/adr/0442 §3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub requires_added: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub requires_removed: Vec<String>,
}

/// Input of `cad.forms.transition` v1 (İş akışı adımı, docs/adr/0428): a record taken a step of its form's
/// workflow, one undo step (“Form: <adım>”): its state field set to the step's state, with the values
/// given (on a locked record only the step's reason); the form valid when the step asks so, its reason
/// written when it names one.
///
/// Refusals, in this order: `invalid_uid`, `invalid_moment`, `invalid_user`, `invalid_revision`,
/// `revision_conflict` (status `conflict`), `unknown_form`, `form_unpublished`, `form_preflight`,
/// `no_workflow`, `entity_not_found`, `form_layer_mismatch`, `layer_locked`, `unknown_step`,
/// `step_not_allowed`, `record_locked`, `form_invalid`, `reason_required`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsTransition {
    /// The form's id.
    pub form: String,
    /// Through its draft instead of its latest release.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
    /// The record (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The step's label.
    pub step: String,
    /// Values by the layer fields' keys written with the step (the reason among them).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub values: BTreeMap<String, String>,
    /// Notes on the record's fields by their keys (docs/adr/0442 §4): written to the workflow's notes
    /// field, shown under those fields; a step without them empties it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub notes: BTreeMap<String, String>,
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

/// Output of `cad.forms.transition` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsTransitioned {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The state it was in and the state it is in.
    pub from: String,
    pub to: String,
    /// The record's attributes afterwards.
    pub attrs: BTreeMap<String, String>,
    /// The release used; none: the draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: Option<u32>,
    /// The record's snapshot the step was taken on (docs/adr/0440 §4), also at the end of its log line.
    pub snapshot: String,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Exports a form as a package (docs/adr/0435).
pub const CAD_FORMS_EXPORT: &str = "cad.forms.export";
pub const CAD_FORMS_EXPORT_VERSION: u32 = 1;
/// Imports a form package onto a layer (docs/adr/0435).
pub const CAD_FORMS_IMPORT: &str = "cad.forms.import";
pub const CAD_FORMS_IMPORT_VERSION: u32 = 1;

/// A form package's format and version.
pub const FORM_PACKAGE_FORMAT: &str = "kentos.form-package";
pub const FORM_PACKAGE_VERSION: u32 = 1;

/// A form package (docs/adr/0435): a form's body with the fields it reads and the value lists those
/// fields link, to take to another layer or project. No records, no connections, no rights.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormPackage {
    /// `kentos.form-package`.
    pub format: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    /// The form's name.
    pub name: String,
    pub body: FormBody,
    /// The fields the body binds or reads, as their layer defines them, in its order.
    pub fields: Vec<crate::LayerField>,
    /// The project's value lists those fields link (docs/adr/0430).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::FieldDomain>>", optional))]
    pub domains: Vec<crate::FieldDomain>,
}

/// Input of `cad.forms.export` v1: the form's latest release (or its draft) as a package; writes
/// nothing. Refusals: `unknown_form`, `form_unpublished`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsExport {
    pub form: String,
    /// The draft instead of the latest release.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub draft: bool,
}

/// A package field bound to a project's field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldBinding {
    /// The package field's key.
    pub from: String,
    /// The layer field's key.
    pub to: String,
}

/// Input of `cad.forms.import` v1: a package made a new form (its draft) of `layer`; its fields bound to
/// the layer's (`map`, else the same key, else the same name), the missing ones added when `addFields`.
///
/// Refusals, checked in this order: `invalid_revision`, `revision_conflict` (status `conflict`);
/// `invalid_package`; `layer_not_found`, `not_a_layer`; `invalid_form_id`, `invalid_form_name`,
/// `form_exists`, `too_many_forms`; `invalid_map`, `field_not_found`, `field_kind_mismatch`,
/// `form_fields_missing`. Warnings: `domain_kept`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsImport {
    pub package: FormPackage,
    /// The layer's id.
    pub layer: String,
    /// The new form's id; none: one made of its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    /// The new form's name; none: the package's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// A package field's key → the layer field's key it binds to.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub map: BTreeMap<String, String>,
    /// A package field the layer lacks is added to it (with its value list); else refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub add_fields: bool,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output (and plan) of `cad.forms.import` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormsImported {
    /// The new form's id.
    pub id: String,
    /// Each package field and the layer field it binds to, in the package's order.
    pub bound: Vec<FieldBinding>,
    /// The fields added to the layer, by their keys.
    pub added: Vec<String>,
    /// The value lists added to the project, by their ids.
    pub domains: Vec<String>,
    /// The new draft's preflight errors (it is a draft; publishing needs none).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub errors: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
