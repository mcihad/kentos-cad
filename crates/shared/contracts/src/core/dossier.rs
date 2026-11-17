//! Dossiers of the cloud portal (docs/adr/9000): a professional's application to a municipality, a municipality's
//! transfer of a project to a professional. Their lists and pages, the files of their packages in rounds, their
//! messages and timeline, what the caller may do next, and each person's notifications. The server words every name
//! and sentence; the clients show them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::portal::{Profession, ProfessionalView};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DossierDirection {
    /// A professional to a municipality.
    Application,
    /// A municipality to a professional.
    Transfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DossierStatus {
    Draft,
    // An application's.
    Submitted,
    InReview,
    ChangesRequested,
    Approved,
    Rejected,
    Withdrawn,
    // A transfer's.
    Sent,
    Accepted,
    Declined,
    Delivered,
    RevisionRequested,
    Completed,
    Cancelled,
}

/// The two sides of a dossier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DossierSide {
    Municipality,
    Professional,
}

/// A step a side takes; which ones the caller may take now is in [`DossierDetail::actions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DossierAction {
    Send,
    Withdraw,
    StartReview,
    Assign,
    RequestChanges,
    Approve,
    Reject,
    Accept,
    Decline,
    Deliver,
    RequestRevision,
    Complete,
    Cancel,
}

/// Where the matter is; what was given of it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierSite {
    pub province: Option<String>,
    pub district: Option<String>,
    pub neighbourhood: Option<String>,
    /// Ada.
    pub block: Option<String>,
    /// Parsel.
    pub parcel: Option<String>,
    pub address: Option<String>,
}

/// A legal deadline running for the municipality (İmar Kanunu m.22, m.30), counted in calendar days.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LegalClock {
    /// What is due: "Eksiklik bildirimi", "Karar".
    pub label: String,
    /// YYYY-MM-DD.
    pub due_on: String,
    /// Negative once it has passed.
    pub days_left: i32,
    /// The rule, in a sentence.
    pub basis: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierSummary {
    pub id: String,
    pub direction: DossierDirection,
    /// "2026/0007" once sent.
    pub number: Option<String>,
    pub kind: String,
    pub kind_label: String,
    pub title: String,
    pub status: DossierStatus,
    pub status_label: String,
    pub tenant_id: String,
    pub tenant_slug: String,
    pub tenant_name: String,
    pub professional_id: String,
    pub professional_name: String,
    pub profession: Option<Profession>,
    pub assignee_name: Option<String>,
    /// Packages sent so far.
    pub round: u32,
    pub site: DossierSite,
    pub created_at: String,
    pub updated_at: String,
    pub sent_at: Option<String>,
    /// A transfer's requested delivery day.
    pub due_on: Option<String>,
    pub clock: Option<LegalClock>,
    /// Whether it is waiting for the caller's side.
    pub awaiting_me: bool,
    /// The office the professional shares it with (docs/adr/9000 §18), by id and name.
    #[serde(default)]
    pub office_id: Option<String>,
    #[serde(default)]
    pub office_name: Option<String>,
}

/// `GET …/dossiers`: a page of them, and how many there are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierList {
    pub items: Vec<DossierSummary>,
    pub total: u32,
}

/// The cloud project a file was taken from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FileSource {
    pub tenant_id: String,
    pub project_id: String,
    pub project_name: String,
    /// The project's data revision the file shows, decimal.
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierFileView {
    pub id: String,
    pub side: DossierSide,
    /// `package` or `attachment`.
    pub purpose: String,
    pub round: Option<u32>,
    /// `pending` (may still be removed) or `final` (sent, kept as it was).
    pub state: String,
    pub doc_type: String,
    pub doc_type_label: String,
    pub name: String,
    pub media_type: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub size: u64,
    pub sha256: String,
    pub added_by_name: String,
    pub added_at: String,
    pub source: Option<FileSource>,
    /// A KentOS project file (KCAD): it opens as a project.
    pub is_project: bool,
    /// How far it was checked when it arrived.
    #[serde(default)]
    pub scan: crate::ScanState,
    #[serde(default)]
    pub scan_note: Option<String>,
}

/// One package as it was sent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierRound {
    pub round: u32,
    pub side: DossierSide,
    pub sent_at: Option<String>,
    pub files: Vec<DossierFileView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierMessageView {
    pub id: String,
    pub round: u32,
    pub side: DossierSide,
    /// `message`, `deficiency`, `decision` or `note`.
    pub kind: String,
    pub author_name: String,
    pub body: String,
    pub at: String,
    pub attachments: Vec<DossierFileView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierEventView {
    pub id: String,
    pub at: String,
    pub actor_name: String,
    pub side: DossierSide,
    pub action: String,
    /// What happened, in a sentence.
    pub text: String,
    pub round: u32,
}

/// A document the kind is usually sent with, and whether the package has one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DocCheckItem {
    pub doc_type: String,
    pub label: String,
    pub present: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DocChecklist {
    pub basis: String,
    /// The newest package it counts: the one being prepared, or the last sent (every package sent counts).
    pub round: u32,
    pub items: Vec<DocCheckItem>,
}

/// The organisation side of a dossier, as both sides see it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierOrganization {
    pub tenant_id: String,
    pub slug: String,
    pub name: String,
    pub province: Option<String>,
    pub district: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
}

/// `GET /v1/dossiers/{id}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierDetail {
    pub summary: DossierSummary,
    pub description: String,
    pub owner_name: Option<String>,
    /// The caller's side.
    pub side: DossierSide,
    pub actions: Vec<DossierAction>,
    /// Whether the caller may change its title, kind, place and the like now.
    pub can_edit: bool,
    /// Whether the caller prepares the next package now (files may be added to it).
    pub can_add_files: bool,
    pub can_message: bool,
    /// Whether the caller may remove the draft.
    pub can_delete: bool,
    /// Whether the caller may open a KentOS project file of it as a project: a professional in their own space, a
    /// member who may create projects in the organisation (project manager and above).
    pub can_import: bool,
    /// The caller is its responsible professional and may share it with an office of theirs or take it back.
    #[serde(default)]
    pub can_share_with_office: bool,
    /// Whether the caller issues fees on it and records their payment (docs/adr/9003): the municipality's editors and
    /// above, on an open application.
    #[serde(default)]
    pub can_assess_fees: bool,
    /// Whether the caller cancels its fees: the municipality's project managers and above.
    #[serde(default)]
    pub can_cancel_fees: bool,
    /// Its assessments not cancelled (issued or paid).
    #[serde(default)]
    pub fee_count: u32,
    /// Whether the caller e-signs its files (docs/adr/9004): the professional's side who prepares it (an office's
    /// editors too), the municipality's editors and above. Whether the server has a signing key is
    /// `GET /v1/signing/trust`'s answer.
    #[serde(default)]
    pub can_sign: bool,
    /// The package being prepared: `round + 1`.
    pub pending_round: u32,
    pub rounds: Vec<DossierRound>,
    pub pending: Vec<DossierFileView>,
    pub messages: Vec<DossierMessageView>,
    pub events: Vec<DossierEventView>,
    pub checklist: Option<DocChecklist>,
    pub professional: Option<ProfessionalView>,
    pub organization: DossierOrganization,
    /// Members the dossier may be assigned to (municipality side, editors and above).
    pub assignees: Vec<AssigneeOption>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AssigneeOption {
    pub user_id: String,
    pub display_name: String,
}

/// `POST /v1/me/dossiers`: a professional's new application, as a draft.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ApplicationCreate {
    pub tenant_id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub site: DossierSite,
    pub owner_name: Option<String>,
    /// On behalf of an office the professional is an editor of (docs/adr/9000 §18): its colleagues share the dossier.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub office_id: Option<String>,
}

/// `PUT /v1/dossiers/{id}/office`: the responsible professional shares the dossier with an office, or takes it back
/// (`null`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierOfficeSet {
    pub office_id: Option<String>,
}

/// `POST /v1/tenants/{tenant}/dossiers`: a municipality's new transfer, as a draft.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TransferCreate {
    pub professional_id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub site: DossierSite,
    /// YYYY-MM-DD.
    pub due_on: Option<String>,
}

/// `PATCH /v1/dossiers/{id}`: what its sending side changes while it is theirs to change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierEdit {
    pub kind: String,
    pub title: String,
    pub description: String,
    pub site: DossierSite,
    pub owner_name: Option<String>,
    pub due_on: Option<String>,
}

/// `POST /v1/dossiers/{id}/actions`. `expectedStatus` is the status the caller saw: another's step in between is a
/// conflict, not an overwrite.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierActionRequest {
    pub action: DossierAction,
    pub expected_status: DossierStatus,
    pub note: Option<String>,
    pub assignee_id: Option<String>,
    /// Files uploaded as attachments, sent with the note.
    #[serde(default)]
    pub attachments: Vec<String>,
}

/// `POST /v1/dossiers/{id}/messages`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MessagePost {
    pub body: String,
    #[serde(default)]
    pub attachments: Vec<String>,
}

/// `POST /v1/dossiers/{id}/files/from-project`: a cloud project's present state as a file of the package.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FromProject {
    pub tenant_id: String,
    pub project_id: String,
    pub doc_type: Option<String>,
}

/// `POST /v1/dossiers/{id}/files/{file}/import`: a KCAD file of the dossier as a new cloud project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierFileImport {
    /// Where it goes: the caller's personal space when absent.
    pub tenant_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierFileImported {
    pub tenant_id: String,
    pub project_id: String,
    pub name: String,
}

/// How many dossiers are in each status (a dashboard's figures).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatusCount {
    pub direction: DossierDirection,
    pub status: DossierStatus,
    pub count: u32,
}

/// `GET /v1/tenants/{tenant}/portal/summary` and `GET /v1/me/dossiers/summary`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DossierDashboard {
    pub counts: Vec<StatusCount>,
    /// Waiting for the caller's side.
    pub awaiting: u32,
    /// With a legal deadline passed or within three days.
    pub due_soon: u32,
    /// Assigned to the caller (municipality side).
    pub mine: u32,
    pub recent: Vec<DossierSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NotificationView {
    pub id: String,
    pub at: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub dossier_id: Option<String>,
    /// The organisation a member opens it in; none for a professional's own.
    pub tenant_slug: Option<String>,
}

/// `GET /v1/me/notifications`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Notifications {
    pub items: Vec<NotificationView>,
    pub unread: u32,
}

/// `POST /v1/me/notifications/read`: these, or all of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NotificationsRead {
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub all: bool,
}
