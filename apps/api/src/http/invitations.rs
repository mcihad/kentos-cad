//! The routes of invitations (docs/adr/0035): a project's invitations for
//! those who may share it, and accepting one's link. Making and withdrawing
//! one are the product commands `project.invite` and
//! `project.invitation.revoke` on the project's command route.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use kentos_application::invitations;
use kentos_contracts::{InvitationAccept, InvitationAccepted, ProjectInvitations};

use super::AppState;
use super::auth::Caller;
use super::error::{Body, Failure};
use super::projects::project_access;

/// `GET …/projects/{project}/invitations` (`project.share`).
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<ProjectInvitations>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        invitations::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/invitations/accept`: the signed-in account takes the
/// invitation of the link's token and learns the project it opened.
pub async fn accept(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<InvitationAccept>,
) -> Result<Json<InvitationAccepted>, Failure> {
    let run = async { invitations::accept(state.db()?, &caller.0, input).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
