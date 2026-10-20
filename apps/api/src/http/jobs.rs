//! The routes of durable server jobs (docs/adr/0542): asking for one, listing and reading a project's jobs, cancelling
//! one and downloading a succeeded export's file. The server's workers run them (`kentosd serve` beside the API, or
//! `kentosd worker` alone).

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use kentos_application::{AppError, jobs};
use kentos_contracts::{ServerJob, ServerJobCreate, ServerJobList};
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::{Body as JsonBody, Failure};
use super::files::{FileBody, kcad_headers};
use super::projects::project_access;

fn job_id(text: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(text).map_err(|_| AppError::not_found("İş bulunamadı; listeyi yenileyin."))
}

/// `POST …/projects/{project}/jobs`: asks for a job (201); a request id sent before gives that job back.
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    JsonBody(input): JsonBody<ServerJobCreate>,
) -> Result<(StatusCode, Json<ServerJob>), Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        jobs::create(state.db()?, &a, input).await
    };
    run.await
        .map(|j| (StatusCode::CREATED, Json(j)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/jobs`: the project's jobs, newest first.
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<ServerJobList>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        jobs::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/jobs/{job}`.
pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, job)): Path<(String, String, String)>,
) -> Result<Json<ServerJob>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        jobs::get(state.db()?, &a, job_id(&job)?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST …/projects/{project}/jobs/{job}/cancel`.
pub async fn cancel(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, job)): Path<(String, String, String)>,
) -> Result<Json<ServerJob>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        jobs::cancel(state.db()?, &a, job_id(&job)?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/jobs/{job}/artifact`: a succeeded export's file (`project.download`), its SHA-256 as the
/// entity tag and the revision it shows.
pub async fn artifact(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, job)): Path<(String, String, String)>,
) -> Result<Response, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let (job, file) = jobs::artifact(state.db()?, &state.blobs, &a, job_id(&job)?).await?;
        let artifact = job.artifact.clone().unwrap_or_else(|| kentos_contracts::ServerJobArtifact {
            size: String::new(),
            sha256: String::new(),
            name: format!("{}.kcad", a.name),
        });
        let revision = job
            .result
            .as_ref()
            .and_then(|r| r["revision"].as_str())
            .unwrap_or_default()
            .to_owned();
        let mut response = Body::new(FileBody::new(file)).into_response();
        let h = response.headers_mut();
        if let Ok(v) = axum::http::HeaderValue::from_str(&artifact.size) {
            h.insert(axum::http::header::CONTENT_LENGTH, v);
        }
        kcad_headers(h, &artifact.sha256, &revision, &artifact.name);
        Ok(response)
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}
