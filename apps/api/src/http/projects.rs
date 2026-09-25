//! Projects, their objects, the product commands, the access list and the
//! event log over HTTP (docs/adr/0015). A tenant's list and creating a
//! project start from the caller's membership of the tenant in the path; a
//! project's own routes from the caller's access to that project
//! (`access::project`), which answers 404 alike for a project that does not
//! exist and one the caller has no role in. The use cases check the
//! permissions. A deleted project answers 410 (`project_deleted`) to opening
//! and writing, for those who had access.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::access::{self, ProjectAccess, not_found};
use kentos_application::tenancy::{self, Access};
use kentos_application::{AppError, commands, events, lifecycle, projects, sharing};
use kentos_contracts::{
    CommandEnvelope, EventPage, FeaturePage, ProjectAccessList, ProjectCreate, ProjectInfo,
    ProjectList,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::{Body, Failure, request_id};

fn uuid(text: &str, what: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(text).map_err(|_| AppError::not_found(format!("{what} bulunamadı.")))
}

async fn tenant_access(
    state: &AppState,
    caller: &Caller,
    tenant: &str,
) -> Result<Access, AppError> {
    tenancy::access(state.db()?, &caller.0, uuid(tenant, "Kurum")?).await
}

/// The caller's access to the project in the path. Malformed ids are "not found" like any other.
pub async fn project_access(
    state: &AppState,
    caller: &Caller,
    tenant: &str,
    project: &str,
) -> Result<ProjectAccess, AppError> {
    let (Ok(tenant), Ok(project)) = (Uuid::parse_str(tenant), Uuid::parse_str(project)) else {
        return Err(not_found());
    };
    access::project(state.db()?, &caller.0, tenant, project).await
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
) -> Result<Json<ProjectList>, Failure> {
    let run = async {
        let a = tenant_access(&state, &caller, &tenant).await?;
        projects::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/me/projects`: the caller's own projects and the ones shared with them, in every tenant.
pub async fn mine(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<ProjectList>, Failure> {
    let run = async { projects::mine(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Body(input): Body<ProjectCreate>,
) -> Result<(StatusCode, Json<ProjectInfo>), Failure> {
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let run = async {
        let a = tenant_access(&state, &caller, &tenant).await?;
        projects::create(state.db()?, &a, input, key.as_deref()).await
    };
    run.await
        .map(|p| (StatusCode::CREATED, Json(p)))
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn info(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<ProjectInfo>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        projects::info(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `DELETE …/projects/{project}`: 204 whether it was deleted now or before (a retry); its open editors are told live.
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<StatusCode, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let rid = request_id(&headers);
        if lifecycle::delete(state.db()?, &a, rid.as_deref())
            .await?
            .is_some()
        {
            state.hub.notify(a.tenant, a.project);
        }
        Ok(StatusCode::NO_CONTENT)
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct FeatureQuery {
    after: Option<String>,
    limit: Option<i64>,
    /// Comma-separated ids: the current copies of just these.
    ids: Option<String>,
}

pub async fn features(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    Query(q): Query<FeatureQuery>,
) -> Result<Json<FeaturePage>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let db = state.db()?;
        if let Some(ids) = q.ids.as_deref() {
            let ids = ids
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| {
                    Uuid::parse_str(s)
                        .map_err(|_| AppError::invalid(format!("Nesne kimliği geçersiz: {s}")))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let features = projects::features_by_id(db, &a, &ids).await?;
            return Ok(FeaturePage {
                features,
                next: None,
            });
        }
        let after = q
            .after
            .as_deref()
            .map(|s| {
                Uuid::parse_str(s).map_err(|_| AppError::invalid("after bir nesne kimliği olmalı."))
            })
            .transpose()?;
        projects::features(db, &a, after, q.limit.unwrap_or(projects::PAGE_MAX)).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST …/projects/{project}/commands`: a product command (`project.changes`,
/// `project.share`, `project.access.revoke`); the answer is the command's own output.
pub async fn command(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    Body(envelope): Body<CommandEnvelope>,
) -> Result<Json<serde_json::Value>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let outcome = commands::run(state.db()?, &a, envelope).await?;
        if outcome.committed() {
            state.hub.notify(a.tenant, a.project);
        }
        Ok(outcome.to_json())
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/access`: the owner and the grants (needs `project.share`).
pub async fn access_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<ProjectAccessList>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        sharing::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct EventQuery {
    after: Option<String>,
    limit: Option<i64>,
}

pub async fn event_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    Query(q): Query<EventQuery>,
) -> Result<Json<EventPage>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let after = q
            .after
            .as_deref()
            .unwrap_or("0")
            .parse::<i64>()
            .map_err(|_| AppError::invalid("after bir olay imleci olmalı."))?;
        events::after(state.db()?, &a, after, q.limit.unwrap_or(events::PAGE_MAX)).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
