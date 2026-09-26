//! Projects, their objects, the product commands, the access list, the
//! people to share with, the event log and the catalog over HTTP
//! (docs/adr/0015, 0028). A tenant's list and creating a project (also as the
//! `project.create` command) start from the caller's membership of the
//! tenant in the path; a project's own routes from the caller's access to
//! that project (`access::project`), which answers 404 alike for a project
//! that does not exist and one the caller has no role in. The catalog's
//! views are the caller's own, filtered by row-level security. The use cases
//! check the permissions. A deleted project (in the trash) answers 410
//! (`project_deleted`) to opening and writing, for those who had access; an
//! archived one answers 409 (`project_archived`) to writing.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::access::{self, ProjectAccess, not_found};
use kentos_application::tenancy::{self, Access};
use kentos_application::{AppError, commands, events, lifecycle, listing, people, projects};
use kentos_contracts::{
    CatalogSort, CatalogView, CommandEnvelope, EventPage, FeaturePage, ProjectAccessList,
    ProjectCreate, ProjectDetails, ProjectInfo, ProjectList, ProjectPage, ProjectType,
    ShareCandidates,
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
        listing::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/me/projects`: the caller's own projects and the ones shared with them, in every tenant.
pub async fn mine(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<ProjectList>, Failure> {
    let run = async { listing::mine(state.db()?, &caller.0).await };
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

/// `DELETE …/projects/{project}`: moves it to the trash (as `project.trash`);
/// 204 whether it went now or before (a retry); its open editors are told live.
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<StatusCode, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let rid = request_id(&headers);
        if lifecycle::delete_with(state.db()?, &state.catalog_policy(), &a, rid.as_deref())
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

/// `POST …/projects/{project}/commands`: a project's product command
/// (`project.changes`, sharing, the catalog and lifecycle commands); the
/// answer is the command's own output.
pub async fn command(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    Body(envelope): Body<CommandEnvelope>,
) -> Result<Json<serde_json::Value>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let outcome = commands::run(state.db()?, &state.catalog_policy(), &a, envelope).await?;
        if outcome.committed() {
            state.hub.notify(a.tenant, a.project);
        }
        Ok(outcome.to_json())
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/tenants/{tenant}/commands`: a command with no project yet
/// (`project.create`, the envelope's `projectId` empty), answered with 201.
pub async fn tenant_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Body(envelope): Body<CommandEnvelope>,
) -> Result<(StatusCode, Json<serde_json::Value>), Failure> {
    let run = async {
        let a = tenant_access(&state, &caller, &tenant).await?;
        commands::run_in_tenant(state.db()?, &a, envelope).await
    };
    run.await
        .map(|o| (StatusCode::CREATED, Json(o.to_json())))
        .map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/details`: the project's catalog entry, its object
/// and layer counts and the extent of its objects (any role; 410 in the trash).
pub async fn details(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<ProjectDetails>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        listing::details(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct CatalogParams {
    view: Option<String>,
    tenant: Option<String>,
    q: Option<String>,
    #[serde(rename = "type")]
    project_type: Option<String>,
    sort: Option<String>,
    limit: Option<u32>,
    after: Option<String>,
}

/// A query parameter that must be one of a contract enum's names.
fn named<T: serde::de::DeserializeOwned>(value: &str, what: &str) -> Result<T, AppError> {
    serde_json::from_value(serde_json::Value::String(value.into()))
        .map_err(|_| AppError::invalid(format!("{what} bilinmiyor: {value}")))
}

impl CatalogParams {
    fn query(self) -> Result<listing::CatalogQuery, AppError> {
        let view = self.view.as_deref().ok_or_else(|| {
            AppError::invalid_at(
                "view",
                "view gerekli: mine, organization, shared, recent, favorites, archived ya da trash.",
            )
        })?;
        Ok(listing::CatalogQuery {
            view: named::<CatalogView>(view, "Liste (view)").map_err(|e| e.at("view"))?,
            // A tenant that is not a UUID is no one's: the organisation's view answers 404 as for any other.
            tenant: match self.tenant.as_deref() {
                None | Some("") => None,
                Some(t) => Some(uuid(t, "Kurum")?),
            },
            search: self.q.unwrap_or_default(),
            project_type: self
                .project_type
                .as_deref()
                .filter(|t| !t.is_empty())
                .map(|t| named::<ProjectType>(t, "Proje türü").map_err(|e| e.at("type")))
                .transpose()?,
            sort: self
                .sort
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| named::<CatalogSort>(s, "Sıralama").map_err(|e| e.at("sort")))
                .transpose()?,
            limit: self.limit,
            after: self.after.filter(|a| !a.is_empty()),
        })
    }
}

/// `GET /v1/me/catalog?view=&tenant=&q=&type=&sort=&limit=&after=`: one page
/// of one of the caller's catalog views, searched, sorted and counted after
/// the access check.
pub async fn catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(params): Query<CatalogParams>,
) -> Result<Json<ProjectPage>, Failure> {
    let run = async {
        let query = params.query()?;
        listing::page(state.db()?, &caller.0, &query, state.config.trash_retention).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/access`: who may use the project and why (needs `project.share`).
pub async fn access_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<ProjectAccessList>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        people::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct CandidateQuery {
    q: Option<String>,
}

/// `GET …/projects/{project}/access/candidates?q=`: people the caller may
/// share the project with, by name or e-mail (needs `project.share`).
pub async fn share_candidates(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    Query(q): Query<CandidateQuery>,
) -> Result<Json<ShareCandidates>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        people::candidates(state.db()?, &a, q.q.as_deref().unwrap_or("")).await
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
