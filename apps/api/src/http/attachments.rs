//! A project's attachments and its picture (docs/adr/9000 §15): what a phone or an agent shows of a project in the
//! cloud without a CAD view.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use kentos_application::{attachments, preview};
use kentos_contracts::{AttachmentCreate, AttachmentView};
use serde::Deserialize;

use super::AppState;
use super::auth::Caller;
use super::download::{Stored, picture, respond};
use super::error::{Body as JsonBody, Failure};
use super::projects::project_access;

/// `GET …/projects/{project}/attachments`: newest first.
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
) -> Result<Json<Vec<AttachmentView>>, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        attachments::list(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST …/projects/{project}/attachments`: a received upload becomes an attachment (201).
pub async fn add(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    JsonBody(input): JsonBody<AttachmentCreate>,
) -> Result<(StatusCode, Json<AttachmentView>), Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        attachments::add(state.db()?, &state.blobs, &a, input).await
    };
    run.await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct InlineQuery {
    /// `1`: shown in place when its kind runs nothing (a PDF, a picture).
    pub inline: Option<String>,
}

fn asked(flag: Option<&str>) -> bool {
    matches!(flag, Some("1" | "true" | "yes"))
}

/// `GET …/projects/{project}/attachments/{attachment}`: its bytes, resumable and cacheable (download.rs).
pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, attachment)): Path<(String, String, String)>,
    Query(q): Query<InlineQuery>,
) -> Result<Response, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let (view, file) = attachments::open(state.db()?, &state.blobs, &a, &attachment).await?;
        respond(
            &headers,
            Stored {
                file,
                size: view.size,
                sha256: &view.sha256,
                name: &view.name,
                media_type: &view.media_type,
                inline: asked(q.inline.as_deref()),
            },
        )
        .await
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}

/// `DELETE …/projects/{project}/attachments/{attachment}` (204).
pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project, attachment)): Path<(String, String, String)>,
) -> Result<StatusCode, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        attachments::remove(state.db()?, &state.blobs, &a, &attachment).await
    };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct PreviewQuery {
    /// Pixels across, 64–4096 (default 1200).
    pub width: Option<u32>,
    /// A file project's revision (default its newest).
    pub revision: Option<i64>,
}

/// `GET …/projects/{project}/preview.svg`: the project's drawing as one SVG, with an entity tag (304 when unchanged).
pub async fn project_preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, project)): Path<(String, String)>,
    Query(q): Query<PreviewQuery>,
) -> Result<Response, Failure> {
    let run = async {
        let a = project_access(&state, &caller, &tenant, &project).await?;
        let seen = headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok());
        let p = preview::project(
            state.db()?,
            &state.blobs,
            &a,
            q.revision,
            q.width.unwrap_or(preview::DEFAULT_WIDTH),
            seen,
        )
        .await?;
        Ok(match p.picture {
            Some(pic) => picture(&p.tag, Some(pic.svg), pic.drawn, pic.skipped),
            None => picture(&p.tag, None, 0, 0),
        })
    };
    run.await.map_err(|e| Failure::with(e, &headers))
}
