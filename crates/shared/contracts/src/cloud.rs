//! The cloud API (Faz B, CLAUDE.md §15–16, §21): sign-in, the account and
//! its tenants, projects, features, the `project.changes` command and the
//! event stream. Ids are UUID text; row versions, revisions and event
//! cursors are bigint as decimal text so JavaScript never rounds them (§24.1).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::document::{Bounds, ProjectSettings, ProjectStyles};
use crate::entity::{Entity, Vec2};
use crate::layer::LayerNode;

/// Name and version of the one lasting edit command of Faz B.
pub const PROJECT_CHANGES: &str = "project.changes";
pub const PROJECT_CHANGES_VERSION: u32 = 1;
/// Key of the project's metadata (name, settings, layer tree, styles) in `expectedVersions`.
pub const PROJECT_META_KEY: &str = "@project";
/// Event kind of a deleted project (`EventRecord.kind`): its editors stop sending.
pub const PROJECT_DELETED: &str = "project.deleted";
/// Sharing a project with a person or changing their role (docs/adr/0015).
pub const PROJECT_SHARE: &str = "project.share";
pub const PROJECT_SHARE_VERSION: u32 = 1;
/// Taking a person's grant away.
pub const PROJECT_ACCESS_REVOKE: &str = "project.access.revoke";
pub const PROJECT_ACCESS_REVOKE_VERSION: u32 = 1;
/// Event kind of a changed grant (`EventRecord.kind`, no objects): open
/// connections check their access again, and one without it is closed.
pub const PROJECT_ACCESS_CHANGED: &str = "project.access";

// ── Sign-in ──────────────────────────────────────────────────────────────

/// `GET /v1/auth/config`: which ways of signing in this server offers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AuthConfig {
    pub local: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub oidc: Option<OidcLoginInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OidcLoginInfo {
    /// Button text, e.g. the organisation's name.
    pub label: String,
    /// Where the browser goes to start (`/v1/auth/oidc/start`).
    pub start_url: String,
}

/// `POST /v1/auth/login` (local accounts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SignInMethod {
    Local,
    Oidc,
    /// An API client with an OpenID access token (no browser session).
    Bearer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TenantRole {
    Viewer,
    Editor,
    ProjectManager,
    Admin,
    Owner,
}

/// A tenant is an organisation or a person's personal space (docs/adr/0015).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TenantKind {
    /// Opened by the server for one person; its only member is its owner, others come through sharing.
    Personal,
    Organization,
}

/// `GET /v1/me`: the signed-in account and every tenant it belongs to
/// (organisations by name, then the personal space, which the server opens
/// on the first sign-in).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Me {
    pub user: UserView,
    pub memberships: Vec<MembershipView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UserView {
    pub id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub email: Option<String>,
    pub method: SignInMethod,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MembershipView {
    pub tenant_id: String,
    pub tenant_slug: String,
    /// An organisation's name; for a personal space, its person's name (the interface says “Kişisel”).
    pub tenant_name: String,
    pub tenant_kind: TenantKind,
    pub role: TenantRole,
    /// A seat is allocated: without one the tenant's projects cannot be opened.
    pub seat: bool,
    /// Membership and tenant are both active.
    pub active: bool,
    /// What this membership may do in the tenant itself: `project.create`,
    /// `member.manage`. What it may do in a project is that project's
    /// (`ProjectAccessView`): a tenant role opens no project by itself.
    pub capabilities: Vec<String>,
}

// ── Project access (docs/adr/0015) ───────────────────────────────────────

/// A role in one project. Each has the rights of the ones before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProjectRole {
    Viewer,
    Commenter,
    Editor,
    Manager,
    /// The project's owner; not given by sharing (ownership is transferred).
    Owner,
}

impl ProjectRole {
    /// Its name in the API and the database.
    pub fn name(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Commenter => "commenter",
            Self::Editor => "editor",
            Self::Manager => "manager",
            Self::Owner => "owner",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [
            Self::Viewer,
            Self::Commenter,
            Self::Editor,
            Self::Manager,
            Self::Owner,
        ]
        .into_iter()
        .find(|r| r.name() == name)
    }
}

/// A role a share gives: every project role but the owner's (ownership is transferred, not shared).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GrantRole {
    Viewer,
    Commenter,
    Editor,
    Manager,
}

impl GrantRole {
    pub const ALL: [GrantRole; 4] = [Self::Viewer, Self::Commenter, Self::Editor, Self::Manager];

    pub fn name(self) -> &'static str {
        ProjectRole::from(self).name()
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.name() == name)
    }
}

impl From<GrantRole> for ProjectRole {
    fn from(role: GrantRole) -> Self {
        match role {
            GrantRole::Viewer => Self::Viewer,
            GrantRole::Commenter => Self::Commenter,
            GrantRole::Editor => Self::Editor,
            GrantRole::Manager => Self::Manager,
        }
    }
}

/// A permission in a project. The names are fixed: commands (`permissions`
/// in the catalog), the web, the desktop, Python and AI use them as they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProjectPermission {
    /// See it in lists, open it, read its objects and events.
    #[serde(rename = "project.read")]
    Read,
    /// Change its objects.
    #[serde(rename = "feature.write")]
    FeatureWrite,
    /// Its name, settings, layer tree and styles.
    #[serde(rename = "project.edit")]
    Edit,
    /// Delete it (softly: the operator can restore it).
    #[serde(rename = "project.delete")]
    Delete,
    /// Comments and review notes.
    #[serde(rename = "project.comment")]
    Comment,
    /// Download and export its files and revisions (viewing is not DRM: what is seen can be copied).
    #[serde(rename = "project.download")]
    Download,
    /// See and open earlier revisions.
    #[serde(rename = "project.history")]
    History,
    /// Give and take away roles in it.
    #[serde(rename = "project.share")]
    Share,
    /// Hand its ownership over.
    #[serde(rename = "project.transfer")]
    Transfer,
    /// Start server jobs on it.
    #[serde(rename = "project.jobs.run")]
    JobsRun,
}

impl ProjectPermission {
    pub const ALL: [ProjectPermission; 10] = [
        Self::Read,
        Self::FeatureWrite,
        Self::Edit,
        Self::Delete,
        Self::Comment,
        Self::Download,
        Self::History,
        Self::Share,
        Self::Transfer,
        Self::JobsRun,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Read => "project.read",
            Self::FeatureWrite => "feature.write",
            Self::Edit => "project.edit",
            Self::Delete => "project.delete",
            Self::Comment => "project.comment",
            Self::Download => "project.download",
            Self::History => "project.history",
            Self::Share => "project.share",
            Self::Transfer => "project.transfer",
            Self::JobsRun => "project.jobs.run",
        }
    }
}

/// Where a person's role in a project comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AccessSource {
    /// They own it.
    Owner,
    /// It was shared with them.
    Grant,
    /// The organisation's policy lets its owners and admins into projects not shared with them.
    Policy,
}

/// The caller's access to a project: the highest of ownership, a grant and the
/// organisation's policy, and exactly what it allows. Buttons follow it; the
/// server checks every request itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectAccessView {
    pub role: ProjectRole,
    pub via: AccessSource,
    pub permissions: Vec<ProjectPermission>,
}

// ── Projects and features ────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub srid: u32,
    pub data_revision: String,
    /// RFC 3339.
    pub updated_at: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub tenant_kind: TenantKind,
    /// The owner's name ("Benimle paylaşılanlar" says whose it is); empty
    /// when the caller cannot see the owner (one who left the organisation).
    pub owner_name: String,
    pub access: ProjectAccessView,
}

/// A tenant's projects the caller may see (`GET /v1/tenants/{tenant}/projects`), or
/// the caller's own and shared ones across tenants (`GET /v1/me/projects`); newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectList {
    pub projects: Vec<ProjectSummary>,
}

/// `POST /v1/tenants/{tenant}/projects`: a new project's metadata (its objects follow as commands).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectCreate {
    pub name: String,
    pub settings: ProjectSettings,
    pub origin: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    pub styles: ProjectStyles,
}

/// `GET /v1/tenants/{tenant}/projects/{project}`: what opening needs before the objects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectInfo {
    pub id: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub tenant_kind: TenantKind,
    /// What the caller may do in it.
    pub access: ProjectAccessView,
    pub name: String,
    pub settings: ProjectSettings,
    pub origin: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    pub styles: ProjectStyles,
    pub meta_version: String,
    pub data_revision: String,
    pub feature_count: String,
    /// Event cursor at the moment this was read: subscribe after it to miss nothing.
    pub event_cursor: String,
}

/// One stored object. `entity.id` is meaningless on the wire (the browser numbers objects itself).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureRecord {
    pub id: String,
    pub version: String,
    pub entity: Entity,
}

/// `GET …/features?after=&limit=`: a page in id order; `next` is the cursor of the following page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeaturePage {
    pub features: Vec<FeatureRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
}

// ── The project.changes command ──────────────────────────────────────────

/// Input of `project.changes` v1: one atomic commit of object changes and, optionally, metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectChanges {
    pub features: Vec<FeatureChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub project: Option<ProjectPatch>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op", rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeatureChange {
    /// A new object; the client picks the UUID so a retry cannot create it twice.
    Create {
        id: String,
        entity: Entity,
    },
    /// Replaces the object; `expectedVersions[id]` must be its current version.
    Update {
        id: String,
        entity: Entity,
    },
    Delete {
        id: String,
    },
}

/// Metadata to replace; absent fields stay. Needs `expectedVersions["@project"]`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub settings: Option<ProjectSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<LayerNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub active_layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub styles: Option<ProjectStyles>,
}

/// The committed result; a retry with the same idempotency key gets the same one (`replayed`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommitResult {
    pub data_revision: String,
    pub meta_version: String,
    /// New version of every created or updated object.
    pub versions: BTreeMap<String, String>,
    pub deleted: Vec<String>,
    pub event_seq: String,
    #[serde(default)]
    pub replayed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ConflictReason {
    /// Someone saved a newer version.
    Changed,
    /// Someone deleted it.
    Deleted,
    /// A created id is already taken.
    Exists,
    /// The project's metadata changed.
    Project,
}

/// Why one object could not be saved, with the server's current copy to show the difference.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureConflict {
    pub id: String,
    pub reason: ConflictReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub actual: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub current: Option<FeatureRecord>,
}

/// Every error response: a stable code, a message for the user, the request id for support.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ApiError {
    pub error: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub conflicts: Option<Vec<FeatureConflict>>,
}

// ── Sharing (project.share, project.access.revoke) ───────────────────────

/// Input of `project.share` v1: gives a person a role in the project, or
/// changes the one they have. In an organisation the person must be a member.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectShare {
    /// The account id (`UserView.id`).
    pub user_id: String,
    pub role: GrantRole,
    /// RFC 3339; the grant ends by itself then. Absent: until revoked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expires_at: Option<String>,
}

/// Input of `project.access.revoke` v1: takes a person's grant away (nothing
/// changes if they have none). Their open connections are closed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectAccessRevoke {
    pub user_id: String,
}

/// A person's grant in a project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectGrant {
    pub user_id: String,
    pub display_name: String,
    pub role: GrantRole,
    /// RFC 3339, when it ends by itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expires_at: Option<String>,
    /// RFC 3339.
    pub updated_at: String,
}

/// Where a cloud project keeps its content (TODOS.md §12.1, CLOUD-21).
/// File projects (binary `.kcad` revisions) and references to an outside
/// PostGIS come as their own kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProjectStorage {
    /// Object by object in the server's PostGIS database (managed): every
    /// save is one transaction and the others see it at once. Every cloud
    /// project today.
    Database,
}

/// Why a person listed with a project cannot use it now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AccessBlock {
    /// Their grant has ended (`expiresAt` passed).
    Expired,
    /// Not (or no longer) a member of the organisation.
    NotMember,
    /// Their account or their membership of the organisation is not active.
    Inactive,
    /// No seat is allocated to them in the organisation.
    NoSeat,
}

/// One person with a role or a grant in a project, as the share dialog shows
/// them: the role they work with now and where it comes from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectAccessHolder {
    pub user_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub email: Option<String>,
    /// The role they work with now: the highest of ownership, the
    /// organisation's policy and an unexpired grant, the same the server
    /// works out when they open it. Absent when none lets them in now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub role: Option<ProjectRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub via: Option<AccessSource>,
    /// Why they cannot use it now (when `role` is absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub blocked: Option<AccessBlock>,
    /// Their grant, also an ended one: what sharing gives, changes and takes
    /// away (ownership and the policy are not grants).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grant: Option<GrantRole>,
    /// RFC 3339, when the grant ends by itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expires_at: Option<String>,
    /// The grant's end has passed: it no longer counts.
    pub expired: bool,
}

/// `GET …/projects/{project}/access`: who may use the project and why
/// (needs `project.share`). `people` holds the owner, the organisation's
/// owners and admins its policy lets in, and every grant (ended ones
/// included), the owner first, then by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectAccessList {
    pub tenant_kind: TenantKind,
    pub storage: ProjectStorage,
    /// The organisation's owners and admins work in its projects not shared
    /// with them (docs/adr/0015); always false in a personal space.
    pub admins_access_all_projects: bool,
    pub owner_id: String,
    pub owner_name: String,
    pub people: Vec<ProjectAccessHolder>,
}

/// A person the project can be shared with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShareCandidate {
    /// The account id `project.share` takes.
    pub user_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub email: Option<String>,
}

/// `GET …/projects/{project}/access/candidates?q=`: people whose name or
/// e-mail holds every word of `q` (Turkish letters folded), among those the
/// caller may see and share with (needs `project.share`): an organisation
/// project's active members, or, for a personal space's project, the active
/// members of the caller's own organisations. The caller and the owner are
/// left out; at most 20, by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShareCandidates {
    pub candidates: Vec<ShareCandidate>,
}

/// The result of `project.share` and `project.access.revoke`; a retry with the
/// same idempotency key gets the same one (`replayed`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectAccessChange {
    pub user_id: String,
    /// The grant now; absent after a revoke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grant: Option<ProjectGrant>,
    /// Whether anything changed (sharing the same role again, or revoking what is not there, does not).
    pub changed: bool,
    /// The `project.access` event, when something changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub event_seq: Option<String>,
    #[serde(default)]
    pub replayed: bool,
}

// ── Events (outbox) ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeatureOp {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EventFeature {
    pub id: String,
    pub op: FeatureOp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<String>,
}

/// One committed change of a project, in commit order (`seq` is the cursor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EventRecord {
    pub seq: String,
    pub data_revision: String,
    /// `project.changes`; `project.deleted` (no objects; nothing is committed
    /// after it); `project.access` (a grant changed; no objects).
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub actor: Option<String>,
    /// The request that made it: a client recognises its own commits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub request_id: Option<String>,
    pub features: Vec<EventFeature>,
    /// Name, settings, layer tree or styles changed.
    pub meta: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EventPage {
    pub events: Vec<EventRecord>,
    /// Cursor to ask from next time (the last seq, or the given one when nothing is new).
    pub next: String,
}

// ── WebSocket (`/v1/ws`) ─────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ClientMessage {
    /// Follow a project's events from `after` (missed ones are replayed first).
    #[serde(rename_all = "camelCase")]
    Subscribe {
        tenant_id: String,
        project_id: String,
        after: String,
    },
    Unsubscribe,
    /// Heartbeat; the server answers `pong` with the same `t`.
    Ping {
        t: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    Subscribed {
        project_id: String,
        after: String,
    },
    #[serde(rename_all = "camelCase")]
    Events {
        project_id: String,
        events: Vec<EventRecord>,
    },
    Pong {
        t: f64,
    },
    /// The cursor is too old or broken: reopen the project.
    #[serde(rename_all = "camelCase")]
    ResyncRequired {
        project_id: String,
    },
    Error {
        error: String,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_names_on_the_wire_are_the_fixed_ones() {
        // The names clients, the catalog and the database use are exactly the serialized ones.
        for p in ProjectPermission::ALL {
            assert_eq!(serde_json::to_value(p).unwrap(), p.name());
        }
        for r in [
            ProjectRole::Viewer,
            ProjectRole::Commenter,
            ProjectRole::Editor,
            ProjectRole::Manager,
            ProjectRole::Owner,
        ] {
            assert_eq!(serde_json::to_value(r).unwrap(), r.name());
            assert_eq!(ProjectRole::from_name(r.name()), Some(r));
        }
        for g in GrantRole::ALL {
            assert_eq!(serde_json::to_value(g).unwrap(), g.name());
            assert_eq!(GrantRole::from_name(g.name()), Some(g));
        }
        // Sharing never makes an owner.
        assert_eq!(GrantRole::from_name("owner"), None);
        assert!(serde_json::from_value::<GrantRole>(serde_json::json!("owner")).is_err());
        assert_eq!(ProjectRole::from_name("admin"), None);
    }
}
