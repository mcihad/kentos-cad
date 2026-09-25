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

/// `GET /v1/me`: the signed-in account and every tenant it belongs to.
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
    pub tenant_name: String,
    pub role: TenantRole,
    /// A seat is allocated: without one the tenant's projects cannot be opened.
    pub seat: bool,
    /// Membership and tenant are both active.
    pub active: bool,
    /// What this membership may do (`project.read`, `feature.write`, …).
    pub capabilities: Vec<String>,
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
}

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
    /// `project.changes`, or `project.deleted` (no objects; nothing is committed after it).
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
