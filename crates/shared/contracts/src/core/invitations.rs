//! Invitations to a project by link, and guests (docs/adr/0035; TODOS.md
//! CLOUD-16, CLOUD-17).
//!
//! - `project.invite` ([`ProjectInvite`]) makes an invitation for one e-mail
//!   with the role it gives (at most editor). Its token is in the first
//!   answer only ([`InvitationChange::token`]); the inviter sends the link
//!   themselves. A new invitation for the same e-mail replaces a waiting one.
//! - `project.invitation.revoke` ([`InvitationRevoke`]) withdraws a waiting one.
//! - `GET …/invitations` lists a project's invitations ([`ProjectInvitations`])
//!   to those who may share it.
//! - `POST /v1/invitations/accept` ([`InvitationAccept`]) takes the token of a
//!   signed-in account whose verified e-mail is the invitation's
//!   ([`InvitationAccepted`]). Someone outside the project's organisation
//!   becomes a guest: they reach only the projects shared with them, while
//!   the organisation takes guests.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{GrantRole, ProjectRole, TenantKind};

/// Invites someone to a project by e-mail.
pub const PROJECT_INVITE: &str = "project.invite";
pub const PROJECT_INVITE_VERSION: u32 = 1;

/// Withdraws a waiting invitation.
pub const PROJECT_INVITATION_REVOKE: &str = "project.invitation.revoke";
pub const PROJECT_INVITATION_REVOKE_VERSION: u32 = 1;

/// Days an invitation waits when no end is given.
pub const INVITATION_DAYS: u32 = 14;

/// The longest wait an invitation may be given, in days.
pub const INVITATION_MAX_DAYS: u32 = 90;

/// Where an invitation is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum InvitationState {
    /// Waiting to be accepted.
    Pending,
    Accepted,
    Revoked,
    /// Its end passed before anyone accepted it.
    Expired,
}

/// Input of `project.invite` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectInvite {
    /// The invited person's e-mail; kept in lower case.
    pub email: String,
    /// Viewer, commenter or editor: managing and sharing stay with members.
    pub role: GrantRole,
    /// When it stops waiting (RFC 3339); [`INVITATION_DAYS`] from now when
    /// absent, at most [`INVITATION_MAX_DAYS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expires_at: Option<String>,
}

/// Input of `project.invitation.revoke` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InvitationRevoke {
    pub invitation_id: String,
}

/// One invitation of a project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectInvitation {
    pub id: String,
    pub email: String,
    pub role: GrantRole,
    pub state: InvitationState,
    pub created_by: String,
    pub created_by_name: String,
    /// RFC 3339.
    pub created_at: String,
    /// RFC 3339.
    pub expires_at: String,
    /// Who accepted it, when it was accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub accepted_by_name: Option<String>,
    /// RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub accepted_at: Option<String>,
}

/// Output of `project.invite` and `project.invitation.revoke` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InvitationChange {
    pub invitation: ProjectInvitation,
    /// A new invitation's token (64 hexadecimal digits), for the link the
    /// inviter sends. Only in the first answer: the server keeps only its
    /// hash, and a retry's stored answer has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub token: Option<String>,
    /// The stored answer of an earlier identical command (its key was seen).
    #[serde(default)]
    pub replayed: bool,
}

/// `GET …/invitations`: a project's invitations, newest first (waiting ones
/// and those of the last 30 days).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectInvitations {
    pub invitations: Vec<ProjectInvitation>,
}

/// `POST /v1/invitations/accept`: the token from the link.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InvitationAccept {
    pub token: String,
}

/// The project an accepted invitation opened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InvitationAccepted {
    pub tenant_id: String,
    pub tenant_name: String,
    pub tenant_kind: TenantKind,
    pub project_id: String,
    pub project_name: String,
    /// The role the account now has in the project.
    pub role: ProjectRole,
    /// A guest: outside the project's organisation.
    pub guest: bool,
}
