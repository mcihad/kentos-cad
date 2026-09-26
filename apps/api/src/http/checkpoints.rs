//! The routes of checkpoints (docs/adr/0034): listing a project's
//! checkpoints and downloading one's file. Making and removing one are the
//! product commands `project.checkpoint.create` and `.delete` on the
//! project's command route.

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use kentos_application::{AppError, checkpoints};
use kentos_contracts::ProjectCheckpoints;
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::Failure;
use super::files::{FileBody, kcad_headers};
use super::projects::project_access;

/// `GET …/projects/{project}/checkpoints`: newest first (`project.history`).
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<ProjectCheckpoints>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        checkpoints::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET …/projects/{project}/checkpoints/{checkpoint}`: its file
/// (`project.history` and `project.download`), with its SHA-256 as the
/// entity tag and its revision.
pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, checkpoint)): Path<(String, String, String)>,
) -> Result<Response, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let id = Uuid::parse_str(&checkpoint).map_err(|_| {
            AppError::not_found("Kontrol noktası bulunamadı; silinmiş olabilir. Listeyi yenileyin.")
        })?;
        let (info, file) = checkpoints::download(state.db()?, &state.blobs, &a, id).await?;
        let mut response = Body::new(FileBody::new(file)).into_response();
        let h = response.headers_mut();
        if let Ok(v) = axum::http::HeaderValue::from_str(&info.size) {
            h.insert(axum::http::header::CONTENT_LENGTH, v);
        }
        kcad_headers(
            h,
            &info.sha256,
            &info.revision,
            &format!("{} - {}.kcad", a.name, info.name),
        );
        Ok(response)
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}
