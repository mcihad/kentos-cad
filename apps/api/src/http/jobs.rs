//! The routes of durable server jobs (docs/adr/0542): asking for one, listing and reading a project's jobs, cancelling
//! one and downloading a succeeded job's file (an export's `.kcad`, a processing job's GeoPackage, docs/adr/0559, an
//! atlas's PDF, docs/adr/0561). The server's workers run them (`kentosd serve` beside the API, or `kentosd worker`
//! alone).

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::{AppError, jobs};
use kentos_contracts::{ServerJob, ServerJobCreate, ServerJobList};
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::{Body as JsonBody, Failure};
use super::files::{FileBody, kcad_headers, range_start};
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
        jobs::create(state.db()?, &a, input, &state.config.job_limits).await
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
/// entity tag and the revision it shows. An artifact never changes, so a download cut short goes on from where it
/// stopped: `Range: bytes=N-` answers the rest (206) (docs/adr/0045, 0547).
pub async fn artifact(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, job)): Path<(String, String, String)>,
) -> Result<Response, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let (job, mut file) = jobs::artifact(state.db()?, &state.blobs, &a, job_id(&job)?).await?;
        let artifact =
            job.artifact
                .clone()
                .unwrap_or_else(|| kentos_contracts::ServerJobArtifact {
                    size: String::new(),
                    sha256: String::new(),
                    name: format!("{}.kcad", a.name),
                });
        let revision = job
            .result
            .as_ref()
            .and_then(|r| {
                r["revision"]
                    .as_str()
                    .or_else(|| r["baseRevision"].as_str())
            })
            .unwrap_or_default()
            .to_owned();
        let size: u64 = artifact.size.parse().unwrap_or(0);
        let start = range_start(&headers).filter(|s| *s > 0);
        if let Some(s) = start
            && s >= size
        {
            let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            if let Ok(v) = HeaderValue::from_str(&format!("bytes */{size}")) {
                response.headers_mut().insert(header::CONTENT_RANGE, v);
            }
            return Ok(response);
        }
        if let Some(s) = start {
            use tokio::io::AsyncSeekExt as _;
            file.seek(std::io::SeekFrom::Start(s))
                .await
                .map_err(AppError::Storage)?;
        }
        let mut response = Body::new(FileBody::new(file)).into_response();
        let h = response.headers_mut();
        h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        match start {
            Some(s) => {
                h.insert(header::CONTENT_LENGTH, HeaderValue::from(size - s));
                if let Ok(v) = HeaderValue::from_str(&format!("bytes {s}-{}/{size}", size - 1)) {
                    h.insert(header::CONTENT_RANGE, v);
                }
            }
            None => {
                h.insert(header::CONTENT_LENGTH, HeaderValue::from(size));
            }
        }
        kcad_headers(h, &artifact.sha256, &revision, &artifact.name);
        if artifact.name.ends_with(".pdf") {
            h.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            );
        }
        if start.is_some() {
            *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        }
        Ok(response)
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}
