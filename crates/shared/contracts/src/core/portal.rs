//! The cloud portal (docs/adr/9000): who a signed-in person is on the portal (platform administrator, member of
//! organisations, professional), signing up as a professional, the catalog the portal speaks in (kinds, documents,
//! professions, statuses with their Turkish names, from the server), an organisation's portal settings and members,
//! and the platform administrators' views. Dossiers are `dossier.rs`. Ids are UUID text, times RFC 3339 text.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::cloud::{TenantRole, UserView};

/// A professional's profession; their chamber follows from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Profession {
    Architect,
    Surveyor,
    CivilEngineer,
    MechanicalEngineer,
    ElectricalEngineer,
    UrbanPlanner,
    LandscapeArchitect,
    GeologicalEngineer,
}

/// Where an administrator's check of a professional stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProfessionalStatus {
    Pending,
    Verified,
    Rejected,
    Suspended,
}

/// What an organisation is to the portal: a municipality takes applications and transfers projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OrganizationCategory {
    Municipality,
    Office,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfessionalView {
    pub user_id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub profession: Profession,
    /// Their chamber registration number, as they gave it.
    pub chamber_no: String,
    pub office_name: Option<String>,
    pub phone: Option<String>,
    pub province: Option<String>,
    /// Whether municipalities find them in their directory.
    pub listed: bool,
    pub status: ProfessionalStatus,
    pub status_note: String,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

/// One organisation the person belongs to, as the portal opens it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PortalMembership {
    pub tenant_id: String,
    pub tenant_slug: String,
    pub tenant_name: String,
    pub category: OrganizationCategory,
    pub role: TenantRole,
    /// Membership active, seat allocated, organisation active: it can be used now.
    pub usable: bool,
    pub province: Option<String>,
    pub district: Option<String>,
}

/// `GET /v1/portal/me`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PortalMe {
    pub user: UserView,
    pub login: Option<String>,
    pub platform_admin: bool,
    pub professional: Option<ProfessionalView>,
    pub organizations: Vec<PortalMembership>,
    pub unread: u32,
}

/// `GET /v1/portal/signup`: whether the platform takes sign-ups now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SignupInfo {
    pub open: bool,
    /// The form's ticket while sign-ups are open: brought back with the sign-up, no sooner than a few seconds and no
    /// later than two hours, once (docs/adr/9000 §20).
    #[serde(default)]
    pub ticket: Option<String>,
}

/// `POST /v1/portal/signup`, and an administrator opening a professional's account.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Signup {
    pub login: String,
    pub password: String,
    pub display_name: String,
    pub email: String,
    pub profession: Profession,
    pub chamber_no: String,
    pub office_name: Option<String>,
    pub phone: Option<String>,
    pub province: Option<String>,
    /// `GET /v1/portal/signup`'s ticket (signing up by oneself; an administrator opening the account needs none).
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub ticket: Option<String>,
    /// A field the form's people do not see: filled, the request is a machine's (docs/adr/9000 §20).
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub website: Option<String>,
}

/// `PUT /v1/portal/me/professional`: what a professional changes in their own profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProfessionalEdit {
    pub office_name: Option<String>,
    pub phone: Option<String>,
    pub province: Option<String>,
    pub listed: bool,
}

/// `POST /v1/portal/me/password`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PasswordChange {
    pub current: String,
    pub new: String,
}

// ── The catalog: codes and their names, from the server ──────────────────

/// A code and its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DocTypeOption {
    pub code: String,
    pub label: String,
}

/// A kind of dossier: its name, the documents it is usually sent with (and on what basis), and its legal deadline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct KindOption {
    pub code: String,
    pub label: String,
    pub description: String,
    pub documents: Vec<DocTypeOption>,
    /// Where the document list comes from: the law that names them, or the portal's suggestion.
    pub documents_basis: Option<String>,
    /// The legal deadline the municipality answers in, said in a sentence; none for most kinds.
    pub deadline: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProfessionOption {
    pub code: Profession,
    pub label: String,
    pub chamber: String,
}

/// A status's name and the tone it is shown in (`neutral`, `info`, `warning`, `success`, `danger`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatusOption {
    pub code: String,
    pub label: String,
    pub tone: String,
}

/// `GET /v1/portal/catalog`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PortalCatalog {
    pub application_kinds: Vec<KindOption>,
    pub transfer_kinds: Vec<KindOption>,
    pub doc_types: Vec<DocTypeOption>,
    pub professions: Vec<ProfessionOption>,
    pub statuses: Vec<StatusOption>,
    pub professional_statuses: Vec<StatusOption>,
}

// ── Organisations ────────────────────────────────────────────────────────

/// A municipality as a professional chooses one to apply to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MunicipalityCard {
    pub tenant_id: String,
    pub slug: String,
    pub name: String,
    pub province: Option<String>,
    pub district: Option<String>,
    pub accepts_applications: bool,
    /// The kinds it takes (all of them when it lists none).
    pub application_kinds: Vec<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
}

/// `GET/PUT /v1/tenants/{tenant}/portal`: an organisation's portal settings and seats.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrganizationPortal {
    pub tenant_id: String,
    pub slug: String,
    pub name: String,
    pub category: OrganizationCategory,
    pub province: Option<String>,
    pub district: Option<String>,
    pub accepts_applications: bool,
    pub application_kinds: Vec<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub seat_limit: u32,
    pub seats_used: u32,
    pub role: TenantRole,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPortalEdit {
    pub accepts_applications: bool,
    pub application_kinds: Vec<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MemberView {
    pub user_id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: TenantRole,
    /// `active` or `disabled`.
    pub status: String,
    pub seat: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct MemberCreate {
    pub login: String,
    pub password: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: TenantRole,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct MemberEdit {
    pub role: TenantRole,
    pub status: String,
    pub seat: bool,
}

/// A professional in a municipality's directory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfessionalCard {
    pub user_id: String,
    pub display_name: String,
    pub profession: Profession,
    pub chamber_no: String,
    pub office_name: Option<String>,
    pub province: Option<String>,
    pub status: ProfessionalStatus,
    /// Dossiers this organisation has with them.
    pub dossiers: u32,
}

// ── Platform administration ──────────────────────────────────────────────

/// `GET /v1/platform/summary`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformSummary {
    pub municipalities: u32,
    pub organisations: u32,
    pub suspended_organisations: u32,
    pub accounts: u32,
    pub disabled_accounts: u32,
    pub professionals: u32,
    pub pending_professionals: u32,
    pub seats_used: u32,
    pub seat_limit: u32,
    pub open_dossiers: u32,
    pub dossiers_this_month: u32,
    pub projects: u32,
    pub active_sessions: u32,
    pub signup_open: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformTenant {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub category: OrganizationCategory,
    pub province: Option<String>,
    pub district: Option<String>,
    /// `active` or `suspended`.
    pub status: String,
    pub seat_limit: u32,
    pub seats_used: u32,
    pub members: u32,
    pub projects: u32,
    pub dossiers: u32,
    pub accepts_applications: bool,
    pub created_at: String,
}

/// The first owner of a new organisation: an existing account, or a new one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct NewOwner {
    pub user_id: Option<String>,
    pub login: Option<String>,
    pub password: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformTenantCreate {
    pub slug: String,
    pub name: String,
    pub category: OrganizationCategory,
    pub province: Option<String>,
    pub district: Option<String>,
    pub seats: u32,
    pub owner: Option<NewOwner>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformTenantEdit {
    pub name: String,
    pub category: OrganizationCategory,
    pub province: Option<String>,
    pub district: Option<String>,
    pub seats: u32,
    pub status: String,
}

/// An organisation with its members, for its administrators' page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformTenantDetail {
    pub tenant: PlatformTenant,
    pub members: Vec<MemberView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformUser {
    pub id: String,
    pub login: Option<String>,
    pub display_name: String,
    pub email: Option<String>,
    /// `active` or `disabled`.
    pub status: String,
    pub created_at: String,
    /// Opened by its owner through signing up, not yet attested by an administrator.
    pub signup: bool,
    pub platform_admin: bool,
    pub profession: Option<Profession>,
    pub professional_status: Option<ProfessionalStatus>,
    pub organizations: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformUserCreate {
    pub login: String,
    pub password: String,
    pub display_name: String,
    pub email: Option<String>,
}

/// An account's status (`active`, `disabled`) or a new password; either or both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlatformUserEdit {
    pub status: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProfessionalReview {
    pub status: ProfessionalStatus,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformEventView {
    pub id: String,
    pub at: String,
    pub actor_name: String,
    pub action: String,
    /// What was done, in a sentence.
    pub text: String,
    pub target_kind: String,
    pub target_label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlatformSettings {
    pub signup_open: bool,
    /// Notifications also go out by e-mail / SMS (docs/adr/9000 §17).
    #[serde(default)]
    pub email_notifications: bool,
    #[serde(default)]
    pub sms_notifications: bool,
    /// How this server sends them (`sendmail`, `webhook`, `folder`); none: it is not set up to.
    #[serde(default)]
    pub email_transport: Option<String>,
    #[serde(default)]
    pub sms_transport: Option<String>,
}
